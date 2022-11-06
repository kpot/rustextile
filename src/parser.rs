use std::collections::{BTreeMap, HashMap};
use std::borrow::Cow;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use indexmap::IndexMap;
use lazy_static::lazy_static;
use fancy_regex::{Regex, Captures, Replacer, Match};
pub use ammonia::Builder as AmmoniaBuilder;

use crate::charcounter::CharCounter;
use crate::regextra::{split_with_capture, fregex, multi_replace, multi_replace_with_one, unwrap_or_empty};
use crate::htmltools::{generate_tag, encode_html, join_html_attributes, unescape, has_raw_text, reverse_encode_html};
use crate::table::{process_table, TABLE_SPAN_RE_S};
use crate::urlutils::{UrlBits, UrlString};
use crate::block::{Block, BlockAttributes, BlockHtmlAttributes};
use crate::regex_snips::{
    CLS_RE_S, ALIGN_RE_S, SNIP_ACR, SNIP_ABR, SNIP_SPACE, SNIP_DIGIT,
    SNIP_WRD, SNIP_CUR, SNIP_CHAR, LONE_AMP_RE, PNCT_RE_S, DIVIDER_RE};

const SYMS_RE_S: &str = "¤§µ¶†‡•∗∴◊♠♣♥♦";
// https://www.unicode.org/reports/tr44/#GC_Values_Table
const BLOCK_TAGS_RE_S: &str = r"bq|bc|notextile|pre|h[1-6]|fn\d+|p|###";
const BLOCK_TAGS_LITE_RE_S: &str = "bq|bc|p";
const RESTRICTED_URL_SCHEMES: [&str; 4] = ["http", "https", "ftp", "mailto"];
const UNRESTRICTED_URL_SCHEMES: [&str; 9] = ["http", "https", "ftp", "mailto", "file", "tel", "callto", "sftp", "data"];

fn span_re(tag: &str) -> Regex {
    const PNCT: &str = r#".,"'?!;:‹›«»„“”‚‘’"#;
    fregex!(
        &format!(
            concat!(
                r"(?P<pre>^|(?<=[\s>{pnct}\(])|[{{\[])",
                r"(?P<tag>{tag})(?!{tag})",
                r"(?P<atts>{cls})",
                r"(?!{tag})",
                r"(?::(?P<cite>\S+[^{tag}]{space}))?",
                r"(?P<content>[^{space}{tag}]+|\S.*?[^\s{tag}\n])",
                r"(?P<end>[{pnct}]*)",
                r"{tag}",
                r"(?P<tail>$|[\[\]}}<]|(?=[{pnct}]{{1,2}}[^0-9]|\s|\)))"),
            tag=tag, cls=*CLS_RE_S, pnct=PNCT, space=SNIP_SPACE))
}

fn do_special<'t, R>(text: &'t str, start: &str, end: &str, method: R) -> Cow<'t, str>
    where R: Replacer
{
    let pattern = Regex::new(
        &format!(r"(?ms)(^|\s|[\[({{>|]){0}(.*?){1}($|[\])}}])?",
                fancy_regex::escape(start),
                fancy_regex::escape(end)))
        .expect("A valid expression");

    pattern.replace_all(text, method)
}

fn get_image_size(url: &str) -> Option<(i64, i64)> {
    const MAX_IMAGE_CHUNK: usize = 1024;
    let mut buffer = [0u8; MAX_IMAGE_CHUNK];
    if let Ok(mut response) = reqwest::blocking::get(url) {
        let mut read_total: usize = 0;
        loop {
            let read_result = response.read(&mut buffer[read_total..]);
            match read_result {
                Ok(bytes_fetched) => {
                    if bytes_fetched == 0 { break; }
                    read_total += bytes_fetched;
                    if let Ok(info) = imageinfo::ImageInfo::from_raw_data(&buffer[..read_total]) {
                        return Some((info.size.width, info.size.height));
                    }
                },
                Err(_) => {
                    return None;
                },
            }
        }
    }
    None
}

fn make_glyph_replacers(is_html5: bool) -> [(Regex, &'static str); 22] {
    lazy_static! {
        static ref CUR: String = format!(
            r"(?:[{0}]{1}*)?", SNIP_CUR, SNIP_SPACE);
    }
    [
        // dimension sign
        (fregex!(
            &format!(
                concat!(r#"(?i)(?<=\b|x)([0-9]+[\])]?['"]? ?)[x]( ?[\[(]?)"#,
                        r"(?=[+-]?{0}[0-9]*\.?[0-9]+)"),
                *CUR)),
         r"$1&#215;$2"),
        // apostrophe's
        (fregex!(&format!(r"({0}|\))'({0})", SNIP_WRD)),
         r"$1&#8217;$2"),
        // back in '88
        (fregex!(&format!(r"({0})'(\d+{1}?)\b(?![.]?[{1}]*?')", SNIP_SPACE, SNIP_WRD)),
         r"$1&#8217;$2"),
        // single opening following an open bracket.
        (fregex!(r"([(\[{])'(?=\S)"), r"$1&#8216;"),
        // single closing
        (fregex!(&format!(r"(\S)'(?={0}|{1}|<|$)", SNIP_SPACE, PNCT_RE_S)),
         r"$1&#8217;"),
        // single opening
        (fregex!(r"'"), r"&#8216;"),
        // double opening following an open bracket. Allows things like
        // Hello ["(Mum) & dad"]
        (fregex!(r#"([(\[{])"(?=\S)"#), r"$1&#8220;"),
        // double closing
        (fregex!(&format!(r#"(\S)"(?={0}|{1}|<|$)"#, SNIP_SPACE, PNCT_RE_S)),
         r"$1&#8221;"),
        // double opening
        (fregex!(r#"""#), r"&#8220;"),
        // ellipsis
        (fregex!(r"([^.]?)\.{3}"), r"$1&#8230;"),
        // ampersand
        (fregex!(r"(\s?)&(\s)"), r"$1&amp;$2"),
        // em dash
        (fregex!(r"(\s?)--(\s?)"), r"$1&#8212;$2"),
        // en dash
        (fregex!(r" - "), r" &#8211; "),
        // trademark
        (fregex!(&format!(r"(?i)(\b ?|{0}|^)[(\[]TM[\])]", SNIP_SPACE)),
         r"$1&#8482;"),
        // registered
        (fregex!(&format!(r"(?i)(\b ?|{0}|^)[(\[]R[\])]", SNIP_SPACE)),
         r"$1&#174;"),
        // copyright
        (fregex!(&format!(r"(?i)(\b ?|{0}|^)[(\[]C[\])]", SNIP_SPACE)),
         r"$1&#169;"),
        // 1/2
        (fregex!(r"[(\[]1\/2[\])]"), r"&#189;"),
        // 1/4
        (fregex!(r"[(\[]1\/4[\])]"), r"&#188;"),
        // 3/4
        (fregex!(r"[(\[]3\/4[\])]"), r"&#190;"),
        // degrees
        (fregex!(r"[(\[]o[\])]"), r"&#176;"),
        // plus/minus
        (fregex!(r"[(\[]\+\/-[\])]"), r"&#177;"),
        // 3+ uppercase acronym
        (fregex!(&format!(r"\b([{0}][{1}]{{2,}})\b(?:[(]([^)]*)[)])", SNIP_ABR, SNIP_ACR)),
         if is_html5 {r#"<abbr title="$2">$1</abbr>"#} else {r#"<acronym title="$2">$1</acronym>"#}),
    ]
}

#[derive(Clone, Debug)]
pub(crate) struct NoteInfo {
    pub id: String,
    //attrs: Option<Attrs>,
    pub content: Option<String>,
    pub link: Option<String>,
    pub attrs: Option<String>,
    pub seq: Option<String>,
    pub refids: Vec<String>,
}


fn get_special_options<'a,'b>(pre: &'a str, tail: &'b str) -> (&'a str, &'b str) {
    const SPAN_WRAPPERS: [(&str, &str); 1] = [
        ("[", "]"),
    ];
    for (before, after) in SPAN_WRAPPERS {
        if pre == before && tail == after {
            return ("", "")
        }
    }
    (pre, tail)
}

fn make_url_readable(url: &str) -> &str {
    for pattern in ["://", ":"] {
        if let Some(pos) = url.find(pattern) {
            return &url[pos + pattern.len()..]
        }
    }
    url
}

pub(crate) struct ParserState<'t> {
    pub notes: BTreeMap<String, NoteInfo>,
    pub footnotes: IndexMap<String, String>,
    shelf: IndexMap<String, String>,
    urlrefs: IndexMap<String, UrlString<'t>>,
    note_index: u32,
    link_index: u32,
    ref_index: u32,
    span_depth: u32,
    ref_cache: IndexMap<u32, String>,
    pub textile: &'t Textile,
    ol_starts: IndexMap<String, usize>,
    unreferenced_notes: BTreeMap<String, NoteInfo>,
    notelist_cache: IndexMap<String, String>,
}


impl <'t> ParserState<'t> {
    fn new(textile: &'t Textile) -> Self {
        Self {
            textile,
            notes: Default::default(),
            footnotes: Default::default(),
            shelf: Default::default(),
            urlrefs: Default::default(),
            note_index: 1,
            link_index: 0,
            ref_index: 0,
            span_depth: 0,
            ol_starts: Default::default(),
            ref_cache: Default::default(),
            notelist_cache: Default::default(),
            unreferenced_notes: Default::default(),
        }
    }

    pub fn increment_link_index(&mut self) -> u32 {
        self.link_index += 1;
        self.link_index
    }

    /// Parses the note definitions and formats them as HTML
    pub fn parse_note_defs(&mut self, m: &Captures) -> &'static str {
        let label = &m["label"];
        let link = &m["link"];
        let att = &m["att"];
        let content = &m["content"];

        // Assign an id if the note reference parse hasn't found the label yet.
        if !self.notes.contains_key(label) {
            let new_index = self.increment_link_index();
            self.notes.insert(
                label.to_owned(),
                NoteInfo {
                    id: format!(
                        "{0}{1}",
                        self.textile.link_prefix,
                        new_index),
                    content: None,
                    link: None,
                    attrs: None,
                    seq: None,
                    refids: Default::default(),
                });

        }
        // Ignores subs
        if self.notes.contains_key(label) {
            let note_content = self.graf(content).into_owned();
            if let Some(mut note) = self.notes.get_mut(label) {
                if note.link.is_none() {
                    note.link = if link.is_empty() { None } else { Some(link.into()) };
                    note.attrs = Some(
                        BlockAttributes
                            ::parse(att, None, true, self.textile.restricted)
                            .into());
                    note.content = Some(note_content);
                }
            }
        }

        ""
    }
    /// Given the pieces of a back reference link, create an <a> tag.
    fn make_back_ref_link(info: &NoteInfo, g_links: &str, i: char) -> Cow<'t, str> {
        fn char_code_to_entity(c: u32) -> String {
            let entity = format!("&#{};", c);
            unescape(&entity).into_owned()
        }

        let backlink_type = match info.link {
            Some(ref link) => link.as_str(),
            None => g_links,
        };
        let allow_inc = !SYMS_RE_S.contains(i);
        let mut i_ = i as u32;

        match backlink_type {
            "!" => Cow::Borrowed(""),
            "^" => {
                if !info.refids.is_empty() {
                    Cow::Owned(format!("<sup><a href=\"#noteref{0}\">{1}</a></sup>",
                                       info.refids[0], char_code_to_entity(i_)))
                } else {
                    Cow::Borrowed("")
                }
            },
            _ => {
                let mut result = String::new();
                for refid in info.refids.iter() {
                    let sup = format!(
                        "<sup><a href=\"#noteref{0}\">{1}</a></sup>",
                        refid, char_code_to_entity(i_));
                    if allow_inc {
                        i_ += 1;
                    }
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&sup);
                }
                Cow::Owned(result)
            }
        }
    }

    /// Parse the text for endnotes
    fn place_note_lists<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        if !self.notes.is_empty() {
            let mut o = BTreeMap::<String, NoteInfo>::new();
            for (label, info) in self.notes.iter() {
                let mut info_clone = info.clone();
                if let Some(ref i) = info.seq {
                    info_clone.seq = Some(label.clone());
                    o.insert(i.clone(), info_clone);
                } else {
                    self.unreferenced_notes.insert(label.clone(), info_clone);
                }
            }
            self.notes = o;
        }
        lazy_static! {
            static ref TEXT_RE: Regex = fregex!(
                &format!(
                    r"<p>notelist({0})(?:\:([\w|{1}]))?([\^!]?)(\+?)\.?[\s]*</p>",
                    *CLS_RE_S, SYMS_RE_S));
        }
        // Given the text that matches as a note, format it into HTML.
        let f_note_lists = |cap: &Captures| -> String {
            let (att, g_links, extras) = (&cap[1], &cap[3], &cap[4]);

            let start_char = match cap.get(2) {
                Some(m) => m.as_str().chars().next().expect("Not empty"),
                None => 'a'
            };
            let index = format!("{0}{1}{2}", g_links, extras, start_char);
            let mut result = String::new();

            if !self.notelist_cache.contains_key(&index) {
                let mut o = Vec::<String>::new();
                if !self.notes.is_empty() {
                    for (_seq, info) in self.notes.iter() {
                        let links = Self::make_back_ref_link(info, g_links, start_char);
                        let li = if let NoteInfo {
                            id: ref infoid,
                            attrs: Some(ref atts),
                            content: Some(ref content),
                            ..
                        } = *info {
                            format!("\t\t<li{0}>{1}<span id=\"note{2}\"> </span>{3}</li>",
                                    atts, links, infoid, content)
                        } else {
                            format!("\t\t<li>{0} Undefined Note [#{1}].</li>",
                                    links, info.seq.as_deref().unwrap_or_default())
                        };
                        o.push(li);
                    }
                }
                if extras == "+" && !self.unreferenced_notes.is_empty() {
                    for info in self.unreferenced_notes.values() {
                        let atts = info.attrs.as_deref().unwrap_or_default();
                        let content = info.content.as_deref().unwrap_or_default();
                        o.push(format!("\t\t<li{0}>{1}</li>", atts, content));
                    }
                }
                result = o.join("\n");
                self.notelist_cache.insert(index, result.clone());
            }
            if result.is_empty() {
                result
            } else {
                let list_atts: String = BlockAttributes
                    ::parse(att, None, true, self.textile.restricted)
                    .into();
                format!("<ol{0}>\n{1}\n\t</ol>", list_atts, result)
            }
        };
        TEXT_RE.replace_all(text, f_note_lists)
    }

    pub fn shelve(&mut self, text: String) -> String {
        self.ref_index += 1;
        let item_id = format!("{0}{1}:shelve", self.textile.uid, self.ref_index);
        self.shelf.insert(item_id.clone(), text);
        item_id
    }

    pub fn shelve_url(&mut self, text: UrlString) -> String {
        let escaped_url = text.to_html_string();
        self.ref_index += 1;
        self.ref_cache.insert(self.ref_index, escaped_url);
        format!("{0}{1}{2}", self.textile.uid, self.ref_index, ":url")
    }

    pub fn retrieve(&self, text: String) -> String {
        let mut new_text = text;
        loop {
            let old = new_text.clone();
            for (k, v) in self.shelf.iter() {
                new_text = new_text.replace(k, v);
            }
            if new_text == old {
                break;
            }
        }
        new_text
    }

    fn retrieve_urls<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        let mut regex_cache = self.textile.regex_cache.borrow_mut();
        let pattern = regex_cache
            .entry(line!())
            .or_default()
            .entry("")
            .or_insert_with(
                || fregex!(&format!(r"{0}(?P<token>[0-9]+):url", self.textile.uid)));

        let retrieve_url = |cap: &Captures| -> String {
            let token = &cap["token"];
            match token.parse::<u32>() {
                Ok(key) => {
                    let url = self.ref_cache.get(&key).cloned().unwrap_or_default();
                    if url.is_empty() {
                        url
                    } else if let Some(rurl) = self.urlrefs.get(&url) {
                        rurl.to_html_string()
                    } else {
                        url
                    }
                },
                Err(_) => {
                    String::new()
                },
            }
        };
        pattern.replace_all(text, retrieve_url)
    }

    fn f_textile(&mut self, cap: &Captures) -> String {
        let (before, notextile) = (&cap[1], &cap[2]);
        let after = unwrap_or_empty(cap.get(3));
        let (before, after) = get_special_options(before, after);
        String::from(before) + &self.shelve(notextile.to_owned()) + after
    }

    pub fn no_textile(&mut self, text: &str) -> String {
        let step1 = do_special(text, "<notextile>", "</notextile>", |cap: &Captures| {Self::f_textile(self, cap)});
        let step2 = do_special(&step1, "==", "==", |cap: &Captures| {Self::f_textile(self, cap)});
        step2.into_owned()
    }

    pub fn code(&mut self, text: &str) -> String {
        fn f_code(parser: &mut ParserState, cap: &Captures) -> String {
            let (before, text) = (&cap[1], &cap[2]);
            let after = unwrap_or_empty(cap.get(3));
            let (before, after) = get_special_options(before, after);
            let text = encode_html(text, false, false);
            String::from(before) + &parser.shelve(format!("<code>{0}</code>", text)) + after
        }

        fn f_pre(parser: &mut ParserState, cap: &Captures) -> String {
            let (before, text) = (&cap[1], &cap[2]);
            let after = unwrap_or_empty(cap.get(3));
            let (before, after) = get_special_options(before, after);
            // text needs to be escaped
            let text = encode_html(text, true, false);
            String::from(before) + "<pre>" + &parser.shelve(text) + "</pre>" + after
        }

        let text = do_special(text, "<code>", "</code>", |cap: &Captures| f_code(self, cap));
        let text = do_special(&text, "@", "@", |cap: &Captures| f_code(self, cap));
        do_special(&text, "<pre>", "</pre>", |cap: &Captures| f_pre(self, cap)).into_owned()
    }

    fn get_html_comments<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        // Search the string for HTML comments, e.g. <!-- comment text -->
        do_special(text, "<!--", "-->", |cap: &Captures| -> String {
            // If self.restricted is True, clean the matched contents of the HTML
            // comment.  Otherwise, return the comments unchanged.
            // The original php had an if statement in here regarding restricted mode.
            // nose reported that this line wasn't covered.  It's correct.  In
            // restricted mode, the html comment tags have already been converted to
            // &lt;!*#8212; and &#8212;&gt; so they don't match in getHTMLComments,
            // and never arrive here.
            let (before, comment_text) = (&cap[1], &cap[2]);
            format!("{0}<!--{1}-->", before, self.shelve(comment_text.to_owned()))
        })
    }

    /// Assuming that in the restricted mode all input was html-encoded
    /// prior to any real parsing, we need to undo the encoding in order to
    /// handle the links properly (once the normalization is done they will
    /// be html-encoded again anyway).
    pub(crate) fn unrestrict_url<'u>(&self, url: &'u str) -> Cow<'u, str> {
        if self.textile.restricted {
            reverse_encode_html(url)
        } else {
            url.into()
        }
    }

    /// Capture and store URL references in `self.urlrefs`.
    fn get_refs<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        fn make_url_ref_re(schemes: &[&str]) -> Regex {
            fregex!(
                &format!(
                    r"(?:(?<=^)|(?<=\s))\[(.+)\]((?:{0}:\/\/|\/)\S+)(?=\s|$)",
                    schemes.join("|")))
        }
        lazy_static! {
            static ref RESTRICTED_URLREF_RE: Regex = make_url_ref_re(&RESTRICTED_URL_SCHEMES[..]);
            static ref UNRESTRICTED_URLREF_RE: Regex = make_url_ref_re(&UNRESTRICTED_URL_SCHEMES[..]);
        }
        let urlref_re: &Regex = if self.textile.restricted {
            &RESTRICTED_URLREF_RE
        } else {
            &UNRESTRICTED_URLREF_RE
        };
        urlref_re.replace_all(text, |cap: &Captures| -> &str {
            let flag = &cap[1];
            let url = self.unrestrict_url(&cap[2]).into_owned();
            self.urlrefs.insert(
                flag.to_string(),
                url.into());
            ""
        })
    }


    fn image<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref PATTERN: Regex = fregex!(
                &format!(
                    concat!(
                        r"(?:[\[{{])?",               // pre
                        r"\!",                        // opening !
                        r"([<>=]|&lt;|&gt;)?",        // optional alignment atts
                        r"({0})",                     // optional style,class atts
                        r"(?:\.\s)?",                 // optional dot-space
                        r"([^\s(!]+)",                // presume this is the src
                        r"\s?",                       // optional space
                        r"(?:\(([^\)]+)\))?",         // optional title
                        r"\!",                        // closing
                        r"(?::(\S+)(?<![\]).,]))?",   // optional href
                        r"(?:[\]}}]|(?=[.,\s)|]|$))", // lookahead: space or end of string
                    ),
                    *CLS_RE_S));
        }
        let f_image = |cap: &Captures| -> String {
            let url = &cap[3];
            if !self.is_valid_url(url) {
                return cap[0].to_owned();
            }
            let mut atts = if let Some(attributes) = cap.get(2) {
                BlockAttributes::parse(attributes.as_str(), None, true, self.textile.restricted).html_attrs()
            } else {
                BlockHtmlAttributes::default()
            };


            if let Some(align) = cap.get(1) {
                let alignment = match align.as_str() {
                    "<" | "&lt;" => "left",
                             "=" => "center",
                    ">" | "&gt;" => "right",
                    _ => unreachable!("Not allowed by regex")
                };
                let use_align_class = match self.textile.align_class_enabled {
                    Some(v) => v,
                    None => match self.textile.html_type {
                        HtmlKind::XHTML => false,
                        HtmlKind::HTML5 => true,
                    }
                };
                if use_align_class {
                    atts.insert_css_class(format!("align-{}", alignment));
                } else {
                    atts.insert("align", alignment.to_owned());
                }
            }

            let optional_title = cap.get(4).map(|m| m.as_str());
            atts.insert("alt", optional_title.unwrap_or_default().to_owned());

            if !UrlBits::parse(url).is_relative() && self.textile.get_sizes {
                if let Some((width, height)) = get_image_size(url) {
                    atts.insert("height", height.to_string());
                    atts.insert("width", width.to_string());
                }
            };
            let url_id = self.shelve_url(
                self.unrestrict_url(url).into());
            atts.insert("src", url_id);

            if let Some(title) = optional_title {
                atts.insert("title", title.to_owned());
            }

            let img = generate_tag("img", None, &atts);
            let out = if let Some(href) = cap.get(5) {
                let shelved_href = self.shelve_url(
                    self.unrestrict_url(href.as_str()).into());
                if !shelved_href.is_empty() {
                    generate_tag(
                        "a",
                        Some(&img),
                        &[("href".into(), shelved_href)])
                } else {
                    img
                }
            } else {
                img
            };
            self.shelve(out)
        };
        PATTERN.replace_all(text, f_image)
    }


    fn links(&mut self, text: &str) -> String {
        let marked_text = self.mark_start_of_links(text);
        let result = self.replace_links(&marked_text).into_owned();
        result
    }

    // Finds and marks the start of well formed links in the input text."""
    // Slice text on '":<not space>' boundaries. These always occur in
    // inline links between the link text and the url part and are much more
    // infrequent than '"' characters so we have less possible links to
    // process.
    fn mark_start_of_links(&self, text: &str) -> String {
        lazy_static! {
            static ref SLICE_RE: Regex = fregex!(
                &format!("\":(?={})", SNIP_CHAR));
        }

        let mut slices: Vec<_> = split_with_capture(&SLICE_RE, text).collect();

        if slices.len() <= 1 {
            return text.into();
        }
        let mut output: Vec<Cow<str>> = Vec::new();

        let last_slice = slices.pop().expect("Verified, not empty");
        lazy_static! {
            static ref START_NOSPACE_RE: Regex = fregex!(r"^\S|=$");
            static ref END_NOSPACE_RE: Regex = fregex!(r"\S$");
        }
        for s in slices {
            // If there is no possible start quote then this slice is not
            // a link
            if !s.contains('"') {
                output.push(Cow::Borrowed(s));
                continue;
            }
            // Cut this slice into possible starting points wherever we find
                // a '"' character. Any of these parts could represent the start
            // of the link text - we have to find which one.
            let mut possible_start_quotes: Vec<_> = s.split('"').collect();

            // Start our search for the start of the link with the closest
            // prior quote mark.
            let mut possibility = possible_start_quotes
                .pop()
                .expect("checked above, at least one value must be present");

            // Init the balanced count. If this is still zero at the end of
            // our do loop we'll mark the " that caused it to balance as the
            // start of the link and move on to the next slice.
            let mut balanced = 0;
            let mut linkparts = Vec::<&str>::new();
            let mut i = 0;

            while balanced != 0 || i == 0 {
                // Starting at the end, pop off the previous part of the
                // slice's fragments.

                // Add this part to those parts that make up the link text.
                linkparts.push(possibility);

                if !possibility.is_empty() {
                    if START_NOSPACE_RE.find(possibility).unwrap_or(None).is_some() {
                        balanced -= 1;
                    }
                    if END_NOSPACE_RE.find(possibility).unwrap_or(None).is_some() {
                        balanced += 1;
                    }
                    if let Some(p) = possible_start_quotes.pop() {
                        possibility = p;
                    }
                } else {
                    // If quotes occur next to each other, we get zero
                    // length strings.  eg. ...""Open the door,
                    // HAL!"":url...  In this case we count a zero length in
                    // the last position as a closing quote and others as
                    // opening quotes.
                    balanced += if i == 0 { 1 } else { - 1 };
                    i += 1;
                    if let Some(p) = possible_start_quotes.pop() {
                        possibility = p;
                    } else {
                        // If out of possible starting segments we back the
                        // last one from the linkparts array
                        linkparts.pop();
                        break;
                    }
                    // If the next possibility is empty or ends in a space
                    // we have a closing ".
                    if possibility.is_empty() || possibility.ends_with(' ') {
                        // force search exit
                        balanced = 0;
                    }
                }

                if balanced <= 0 {
                    possible_start_quotes.push(possibility);
                    break;
                }
            }

            // Rebuild the link's text by reversing the parts and sticking
            // them back together with quotes.
            linkparts.reverse();
            let link_content = linkparts.join("\"");
            // Rebuild the remaining stuff that goes before the link but
            // that's already in order.
            let pre_link = possible_start_quotes.join("\"");
            // Re-assemble the link starts with a specific marker for the
            // next regex.
            let o = format!(
                "{0}{1}linkStartMarker:\"{2}",
                pre_link, self.textile.uid, link_content);
            output.push(Cow::Owned(o));
        }


        // Add the last part back
        output.push(Cow::Borrowed(last_slice));
        // Re-assemble the full text with the start and end markers
        output.join("\":")
    }

    fn table<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref PATTERN: Regex = fregex!(
                &format!(
                    concat!(
                        r"(?ms)^(?:table(?P<tatts>_?{s}{a}{c})\.",
                        r"(?P<summary>.*?)\n)?^(?P<rows>{a}{c}\.? ?\|.*\|)",
                        r"[\s]*\n\n"),
                    s=*TABLE_SPAN_RE_S,
                    a=*ALIGN_RE_S,
                    c=*CLS_RE_S));
        }
        let text = format!("{0}\n\n", text);
        match PATTERN.captures(&text) {
            Ok(Some(cap)) => process_table(
                self,
                unwrap_or_empty(cap.name("tatts")),
                &cap["rows"],
                cap.name("summary").map(|m| m.as_str())).into(),
            _ => text.into()
        }
    }

    /// Parse the text for definition lists and send them to be
    /// formatted.
    pub(crate) fn redcloth_list<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref PATTERN: Regex = fregex!(
                &format!(r"(?ms)^([-]+{0}[ .].*:=.*)$(?![^-])", *CLS_RE_S));
            static ref SPLITTER: Regex = fregex!(
                r"(?m)\n(?=[-])");

            // parses the attributes and the content
            static ref ATTR_CONTENT_RE: Regex = fregex!(
                &format!(r"(?ms)^[-]+({0})\.? (.*)$", *CLS_RE_S));
            // splits the content into the term and definition
            static ref XM_RE: Regex = fregex!(
                &format!(r"(?s)^(.*?){0}*:=(.*?){0}*(=:|:=)?{0}*$",
                         SNIP_SPACE));
        }

        let f_rc_list = |cap: &Captures| -> String {
            let mut out = Vec::<Cow<str>>::new();
            for line in split_with_capture(&SPLITTER, &cap[0]) {
                if let Ok(Some(m)) = ATTR_CONTENT_RE.captures(line) {
                    let atts = &m[1];
                    let content = m[2].trim();
                    let html_atts_str: String = BlockAttributes
                        ::parse(atts, None, true, self.textile.restricted)
                        .into();

                    let xm_capture = XM_RE.captures(content);
                    let (term, definition) = if let Ok(Some(ref xm)) = xm_capture {
                        (xm[1].trim(), xm[2].trim_matches(' '))
                    } else {
                        (content, "")
                    };

                    // if this is the first time through, out as a bool is False
                    if out.is_empty() {
                        let dltag = if definition.is_empty() {
                            format!("<dl{0}>", html_atts_str).into()
                        } else {
                            "<dl>".into()
                        };
                        out.push(dltag);
                    }

                    if !term.is_empty() {
                        let newline_started_def = definition.starts_with('\n');
                        let mut definition = definition
                            .trim()
                            .replace('\n', self.textile.proper_br_tag());

                        if newline_started_def {
                            definition = format!("<p>{0}</p>", definition);
                        }
                        let term = term.replace('\n', self.textile.proper_br_tag());

                        let term = self.graf(&term);
                        let definition = self.graf(&definition);

                        out.push(format!("\t<dt{0}>{1}</dt>", html_atts_str, term).into());
                        if !definition.is_empty() {
                            out.push(format!("\t<dd>{0}</dd>", definition).into());
                        }
                    }

                } else {
                    continue;
                }
            }
            if !out.is_empty() {
                out.push(Cow::Borrowed("</dl>"));
            }
            out.join("\n")
        };

        PATTERN.replace_all(text, f_rc_list)
    }

    pub(crate) fn textile_lists<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref PATTERN: Regex = fregex!(
                &format!(
                    concat!(r"(?ms)^((?:[*;:]+|[*;:#]*#(?:_|\d+)?){0}[ .].*)$",
                            r"(?![^#*;:])"),
                    *CLS_RE_S));
            static ref SPLITTER: Regex = fregex!(r"(?m)\n(?=[*#;:])");
            static ref LINE_PARSER: Regex = fregex!(
                &format!(
                    concat!(
                        r"(?s)^(?P<tl>[#*;:]+)(?P<st>_|\d+)?(?P<atts>{0})[ .]",
                        r"(?P<content>.*)$"),
                    *CLS_RE_S));
        }
        struct ListItem<'t> {
            atts: &'t str,
            content: Cow<'t, str>,
            level: usize,
            tl: &'t str,
            st: &'t str,
        }

        fn list_type(tl: &str) -> &'static str {
            lazy_static! {
                static ref START_RE: Regex = fregex!(r"^([#*]+)");
            }
            match START_RE.captures(tl) {
                Ok(Some(m)) => if m[1].ends_with('#') { "ol" } else { "ul" },
                _ => "dl"
            }
        }

        let f_textile_list = |cap: &Captures| -> String {
            let text = &cap[0];
            let lines = split_with_capture(&SPLITTER, text);
            let mut list_items = Vec::<ListItem>::new();
            for line in lines {
                if let Ok(Some(m)) = LINE_PARSER.captures(line) {
                    // A new list item starts here
                    let tl = unwrap_or_empty(m.name("tl"));
                    list_items.push(
                        ListItem {
                            tl,
                            atts: unwrap_or_empty(m.name("atts")),
                            content: unwrap_or_empty(m.name("content")).into(),
                            level: tl.len(),
                            st: unwrap_or_empty(m.name("st")),
                        });
                } else {
                    // just a continuation of the previous list item
                    if let Some(last_item) = list_items.last_mut() {
                        last_item.content += "\n";
                        last_item.content += line;
                    }
                }
            }
            if list_items.is_empty() || list_items[0].level > 1 {
                return cap[0].to_owned();
            }
            let mut prev: Option<&ListItem> = None;

            let mut lists = IndexMap::<&str, usize>::new();
            let mut out = Vec::<String>::new();
            let mut litem = "";
            for (index, item) in list_items.iter().enumerate() {
                let content = item.content.trim();
                let ltype = list_type(item.tl);
                litem = if item.tl.contains(';') {
                    "dt"
                } else if item.tl.contains(':') {
                    "dd"
                } else {
                    "li"
                };
                let next = list_items.get(index + 1);
                let show_item = !content.is_empty();

                let mut atts = BlockAttributes
                    ::parse(item.atts, None, true, self.textile.restricted)
                    .html_attrs();
                // let mut start: Option<usize> = None;
                if ltype == "ol" {
                    let start_value = self.ol_starts.entry(item.tl.to_string()).or_insert(1);
                    if prev.map(|p| item.level > p.level).unwrap_or(true) {
                        if item.st.is_empty() {
                            *start_value = 1;
                        } else if item.st != "_" {
                            if let Ok(int_st) = item.st.parse() {
                                *start_value = int_st;
                            }
                        }

                        if !item.st.is_empty() {
                            atts.insert("start", start_value.to_string());
                        }
                    }

                    if show_item {
                        *start_value += 1;
                    }
                }

                if let Some(p) = prev {
                    if p.tl.contains(';') && item.tl.contains(':') {
                        lists.insert(item.tl, 2);
                    }
                }
                let tabs = "\t".repeat(item.level - 1);
                let mut line = if !lists.contains_key(item.tl) {
                    lists.insert(item.tl, 1);
                    if show_item {
                        format!(
                            "{0}<{1}{2}>\n{0}\t<{3}>{4}",
                            tabs, ltype, atts.to_string(),
                            litem, content)
                    } else {
                        format!(
                            "{0}<{1}{2}>",
                            tabs, ltype, atts.to_string())
                    }
                } else if show_item {
                    format!(
                        "{0}\t<{1}{2}>{3}",
                        tabs, litem, atts.to_string(), content)
                } else {
                    String::new()
                };

                if show_item && next.map(|n| n.level <= item.level).unwrap_or(true) {
                    line += &format!("</{0}>", litem);
                }

                for (k, v) in lists.clone().iter().rev() {
                    let indent = k.len();
                    if next.map(|n| indent > n.level).unwrap_or(true) {
                        if *v != 2 {
                            line += &format!("\n{0}</{1}>", tabs, list_type(k));
                            if indent > 1 {
                                line += "</";
                                line += litem;
                                line += ">";
                            }
                        }
                        lists.shift_remove(k);
                    }
                }
                prev = Some(item);
                out.push(line);
            }
            let merged_out = out.join("\n");
            self.do_tag_br(litem, &merged_out).into_owned()
        };

        PATTERN.replace_all(text, f_textile_list)
    }

    pub(crate) fn do_tag_br<'a>(&mut self, tag: &'static str, input: &'a str) -> Cow<'a, str> {
        let f_do_br = |cap: &Captures| -> String {
            lazy_static! {
                static ref RE: Regex = fregex!(
                    r"(?i)(.+)(?!(?<=</dd>|</dt>|</li>|<br/>)|(?<=<br>)|(?<=<br />))\n(?![\s|])");
            }
            let content = RE.replace_all(
                &cap[3],
                match self.textile.html_type {
                    HtmlKind::HTML5 => "$1<br>",
                    HtmlKind::XHTML => "$1<br />"
                });
            format!("<{0}{1}>{2}{3}", &cap[1], &cap[2], content, &cap[4])
        };

        let mut regex_cache = self.textile.regex_cache.borrow_mut();
        let pattern = regex_cache
            .entry(line!())
            .or_default()
            .entry(tag)
            .or_insert_with(
                || fregex!(
                    &format!(r"(?s)<({0})([^>]*?)>(.*)(</\1>)",
                             fancy_regex::escape(tag))));
        pattern.replace_all(input, f_do_br)
    }

    fn do_p_br<'a>(&mut self, input: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref TAG_RE: Regex = fregex!(r"(?s)<(p|h[1-6])([^>]*?)>(.*)(</\1>)");
            static ref BR_RE: Regex = fregex!(
                &format!(r"(?i)<br[ ]*/?>{0}*\n(?![{0}|])", SNIP_SPACE));
            static ref NEWLINE_RE: Regex = fregex!(r"\n(?![\s|])");
        }

        let f_do_p_br = |cap: &Captures| -> String {
            let text = &cap[3];
            let text = BR_RE.replace_all(text, "\n");
            let text = NEWLINE_RE.replace_all(
                &text,
                self.textile.proper_br_tag());
            format!("<{0}{1}>{2}{3}", &cap[1], &cap[2], text, &cap[4])
        };
        TAG_RE.replace_all(input, f_do_p_br)
    }


    fn footnote_ref<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref PATTERN: Regex = fregex!(
                &format!(
                    r"(?<=\S)\[(?P<id>{0}+)(?P<nolink>!?)\](?P<space>{1}?)",
                    SNIP_DIGIT,
                    SNIP_SPACE));
        }

        let f_footnote_id = |cap: &Captures| -> String {
            let mut fn_att = Vec::<(String, String)>::new();
            fn_att.push(("class".to_owned(), "footnote".to_owned()));

            let match_id = &cap["id"];
            if !self.footnotes.contains_key(match_id) {
                let new_index = self.increment_link_index();
                let fn_id = format!("{0}{1}", self.textile.link_prefix, new_index);
                fn_att.push(("id".to_owned(), format!("fnrev{0}", &fn_id)));
                self.footnotes.insert(match_id.to_owned(), fn_id);
            }
            let fn_id = &self.footnotes[match_id];
            let link_tag = generate_tag(
                "a",
                Some(match_id),
                &[("href".to_owned(), format!("#fn{0}", fn_id))]);
            let sup_tag = match cap.name("nolink") {
                Some(m) if m.as_str() == "!" => {
                    generate_tag("sup", Some(match_id), &fn_att)
                },
                _ => generate_tag("sup", Some(&link_tag), &fn_att)
            };
            format!("{0}{1}", sup_tag, &cap["space"])
        };

        PATTERN.replace_all(text, f_footnote_id)
    }

    /// Search the text looking for note references.
    fn note_ref<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref TEXT_RE: Regex = fregex!(
                &format!(
                    concat!(
                        r"\[",          // start
                        r"({0})",      // !atts
                        r"\#",
                        r"([^\]!]+)",  // !label
                        r"([!]?)",     // !nolink
                        r"\]"),
                    *CLS_RE_S));
        }

        // Parse and format the matched text into note references.
        // By the time this function is called, all the defs will have been
        // processed into the notes array. So now we can resolve the link numbers
        // in the order we process the refs...
        let f_parse_note_refs = |cap: &Captures| -> String {
            let (atts, label, nolink) = (&cap[1], &cap[2], &cap[3]);
            let html_atts = BlockAttributes::parse(atts, None, true, self.textile.restricted).html_attrs();

            // Assign a sequence number to this reference if there isn't one already
            let num = if let Some(NoteInfo{seq: Some(num), ..}) = self.notes.get(label) {
                num.clone()
            } else {
                let num = self.note_index.to_string();
                self.notes.insert(
                    label.to_string(),
                    NoteInfo {
                        seq: Some(num.clone()),
                        id: "".to_owned(),
                        refids: Default::default(),
                        attrs: None,
                        content: None,
                        link: None,
                    });
                self.note_index += 1;
                num
            };

            //  Make our anchor point and stash it for possible use in backlinks when
            //  the note list is generated later...
            let new_index = self.increment_link_index();
            let refid = format!("{0}{1}", self.textile.link_prefix, new_index);
            let is_note_id_empty = self.notes[label].id.is_empty();
            let new_id: Cow<str> = if is_note_id_empty {
                let new_index = self.increment_link_index();
                format!("{0}{1}", self.textile.link_prefix, new_index).into()
            } else {
                "".into()
            };
            // Build the link (if any)...
            let mut result = format!("<span id=\"noteref{0}\">{1}</span>", &refid, num);
            if nolink != "!" {
                result = format!("<a href=\"#note{0}\">{1}</a>", &new_id, result);
            }
            self.notes.entry(label.to_owned()).and_modify(|note_ref| {
                note_ref.refids.push(refid);
                if is_note_id_empty {
                    note_ref.id.replace_range(.., &new_id);
                }
            });
            // Build the reference...
            generate_tag("sup", Some(&result), &html_atts)
        };
        TEXT_RE.replace_all(text, f_parse_note_refs)
    }



    /// Because of the split command, the regular expressions are different for
    /// when the text at the beginning and the rest of the text.
    /// for example:
    /// let's say the raw text provided is "*Here*'s some textile"
    /// before it gets to this glyphs method, the text has been converted to
    /// "<strong>Here</strong>'s some textile"
    /// When run through the split, we end up with ["<strong>", "Here",
    /// "</strong>", "'s some textile"].  The re.search that follows tells it
    /// not to ignore html tags.
    /// If the single quote is the first character on the line, it's an open
    /// single quote.  If it's the first character of one of those splits, it's
    /// an apostrophe or closed single quote, but the regex will bear that out.
    /// A similar situation occurs for double quotes as well.
    /// So, for the first pass, we use the glyph_search_initial set of
    /// regexes.  For all remaining passes, we use glyph_search
    fn glyphs<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref HTML5_GLYPH_REPLACERS: [(Regex, &'static str); 22] = make_glyph_replacers(true);
            static ref XHTML_GLYPH_REPLACERS: [(Regex, &'static str); 22] = make_glyph_replacers(false);
            static ref SPLITTER_RE: Regex = fregex!(r"(<[\w\/!?].*?>)");
        }

        let text = text.trim_end_matches('\n');
        let mut result = Vec::new();

        let replacers = match self.textile.html_type {
            HtmlKind::HTML5 => &HTML5_GLYPH_REPLACERS[..],
            HtmlKind::XHTML => &XHTML_GLYPH_REPLACERS[..],
        };
        // split the text by any angle-bracketed tags
        for (i, raw_line) in split_with_capture(&SPLITTER_RE, text).enumerate() {
            result.push(
                if i % 2 == 0 {
                    let raw_line = if !self.textile.restricted {
                        Cow::Owned(
                            LONE_AMP_RE.replace_all(raw_line, "&amp;")
                                  .replace('<', "&lt;")
                                  .replace('>', "&gt;"))
                    } else {
                        Cow::Borrowed(raw_line)
                    };
                    multi_replace(
                        raw_line,
                        replacers
                            .iter()
                            .map(|item| (&item.0, item.1))
                            .chain(self.textile.dyn_glyph_replacers.iter()
                                   .map(|item| (&item.0, item.1.as_str())))
                    ).into()
                } else {
                    Cow::Borrowed(raw_line)
                });
        }
        result.join("").into()
    }

    fn replace_links<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        /// Replaces links with tokens and stores them on the shelf
        const STOPCHARS:&str = r#"\s|^'"*"#;
        let mut regex_cache = self.textile.regex_cache.borrow_mut();
        let needle = format!("{0}linkStartMarker:", self.textile.uid);
        let pattern = regex_cache
            .entry(line!())
            .or_default()
            .entry("")
            .or_insert_with(
                || fregex!(
                    &format!(
                        concat!(
                            // Optionally open with a square bracket eg. Look ["here":url]
                            r"(?P<pre>\[)?",
                            // marks start of the link
                            "{0}\"",
                            // grab the content of the inner "..." part of the link, can be anything but
                            // do not worry about matching class, id, lang or title yet
                            r"(?P<inner>(?:.|\n)*?)",
                            // literal ": marks end of atts + text + title block
                            "\":",
                            // url upto a stopchar
                            r"(?P<urlx>[^{1}]*)"),
                        needle, STOPCHARS)));

        let mut f_link = |cap: &Captures| -> String {
            let in_ = &cap[0];
            let mut pre = unwrap_or_empty(cap.get(1)).to_owned();
            let inner = cap[2].replace('\n', self.textile.proper_br_tag());
            let mut url = &cap[3];
            if inner.is_empty() {
                return format!(r#"{0}"{1}":{2}"#, pre, inner, url);
            }
            lazy_static! {
                static ref BLOCK_RE: Regex = fregex!(
                    &format!(
                        concat!(
                            r"^",
                            r"(?P<atts>{0})", // $atts (if any)
                            r"{1}*", // any optional spaces
                            r"(?P<text>",  // $text is...
                            r"(!.+!)", // an image
                            r"|", //  else...
                            r".+?", //  link text
                            r")", // end of $text
                            r"(?:\((?P<title>[^)]+?)\))?",  // $title (if any)
                            r"$"),
                        *CLS_RE_S, SNIP_SPACE));
            }

            let (atts, text, title) = if let Ok(Some(m)) = BLOCK_RE.captures(&inner) {
                let m_text = unwrap_or_empty(m.name("text"));
                (unwrap_or_empty(m.name("atts")),
                 if m_text.is_empty() { inner.as_str() } else { m_text },
                 unwrap_or_empty(m.name("title")))
            } else {
                ("", inner.as_str(), "")
            };
            let mut pop = String::new();
            let mut tight = String::new();
            let csb_count: usize = url.matches(']').count();
            let mut counts = CharCounter::new(['[', ']', '(', ')']);
            counts[']'] = Some(csb_count);
            // Look for footnotes or other square-bracket delimited stuff at the end
            // of the url...
            //
            // eg. "text":url][otherstuff... will have "[otherstuff" popped back
            // out.
            //
            // "text":url?q[]=x][123]    will have "[123]" popped off the back, the
            // remaining closing square brackets will later be tested for balance
            if csb_count > 0 {
                lazy_static! {
                    static ref URL_RE: Regex = fregex!(r"(?P<url>^.*\])(?P<tight>\[.*?)$");
                }

                if let Ok(Some(m)) = URL_RE.captures(url) {
                    url = unwrap_or_empty(m.get(1));
                    tight.replace_range(.., &m[2]);
                }
            }
            // Split off any trailing text that isn't part of an array assignment.
            // eg. "text":...?q[]=value1&q[]=value2 ... is ok
            // "text":...?q[]=value1]following  ... would have "following" popped
            // back out and the remaining square bracket will later be tested for
            // balance
            if csb_count > 0 {
                lazy_static! {
                    static ref URL_RE: Regex = fregex!(r"(?P<url>^.*\])(?!=)(?P<end>.*?)$");
                }
                if let Ok(Some(m)) = URL_RE.captures(url) {
                    url = unwrap_or_empty(m.name("url"));
                    tight = format!("{0}{1}", &m["end"], tight);
                }
            }

            // Now we have the array of all the multi-byte chars in the url we will
            // parse the  uri backwards and pop off  any chars that don't belong
            // there (like . or , or unmatched brackets of various kinds).
            let mut first = true;
            let mut url_chars: Vec<_> = url.chars().collect();

            loop {
                let mut popped = false;
                if let Some(c) = url_chars.pop() {
                    match c {
                        '!' | '?' | ':' | ';' | '.' | ',' => {
                            // Textile URL shouldn't end in these characters, we pop them off
                            // the end and push them out the back of the url again
                            pop.insert(0, c);
                            popped = true;
                        },
                        '>' => {
                            let url_left: String = url_chars.iter().collect();

                            lazy_static! {
                                static ref RE: Regex = fregex!(r"^(?P<url_chars>.*)(?P<tag></[a-z]+)$");
                            }
                            if let Ok(Some(m)) = RE.captures(&url_left) {
                                url_chars.splice(.., m["url_chars"].chars());
                                pop = format!("{0}{1}{2}", &m["tag"], c, pop);
                                popped = true;
                            }
                        },
                        ']' => {
                            // If we find a closing square bracket we are going to see if it is
                            // balanced.  If it is balanced with matching opening bracket then it
                            // is part of the URL else we spit it back out of the URL."""
                            // If counts['['] is None, count the occurrences of '['
                            if counts['['].is_none() {
                                counts['['] = Some(url.matches('[').count());
                            }
                            if counts['['] == counts[']'] {
                                // It is balanced, so keep it
                                url_chars.push(c)
                            } else {
                                // In the case of un-matched closing square brackets we just eat it
                                popped = true;
                                counts.dec(']');
                                if first {
                                    pre.clear();
                                }
                            }
                        },
                        ')' => {
                            if counts[')'].is_none() {
                                counts['('] = Some(url.matches('(').count());
                                counts[')'] = Some(url.matches(')').count());
                            }

                            if counts['('] == counts[')'] {
                                url_chars.push(c);
                            } else {
                                // Unbalanced so spit it out the back end
                                pop.insert(0, c);
                                counts.dec(')');
                                popped = true;
                            }
                        },
                        _ => {
                            url_chars.push(c);
                        }
                    }
                }

                first = false;
                if !popped {
                    break;
                }
            }

            let url: String = url_chars.iter().collect();

            let url = self.unrestrict_url(&url);
            let uri_parts = UrlBits::parse(&url);
            let allowed_schemes = if self.textile.restricted {
                &RESTRICTED_URL_SCHEMES[..]
            } else {
                &UNRESTRICTED_URL_SCHEMES[..]
            };
            let scheme_in_list = allowed_schemes.contains(&(uri_parts.scheme()));
            let is_valid_url = uri_parts.scheme().is_empty() || scheme_in_list;
            if !is_valid_url {
                return in_.replace(&format!("{0}linkStartMarker:", self.textile.uid), "");
            }

            let text: Cow<str> = if text == "$" {
                if scheme_in_list {
                    make_url_readable(&url).into()
                } else if let Some(rurl) = self.urlrefs.get(url.as_ref()) {
                    encode_html(make_url_readable(rurl.source()), true, true).into()
                } else {
                    url
                }
            } else {
                text.into()
            };

            let text = text.trim();
            let title = encode_html(title, false, false);

            let text = if !self.textile.noimage {
                self.image(text)
            } else {
                Cow::Borrowed(text)
            };
            let text = self.span(&text);
            let text = self.glyphs(&text);


            let normalized_url = uri_parts.to_string();
            let url_id = self.shelve_url(
                UrlString::Normalized(normalized_url.into()));
            let mut attributes = BlockAttributes::parse(atts, None, true, self.textile.restricted).html_attrs();
            attributes.insert("href", url_id);
            if !title.is_empty() {
                attributes.insert("title", self.shelve(title));
            }
            if let Some(ref rel) = self.textile.rel {
                attributes.insert("rel", rel.clone());
            }
            let a_text = generate_tag("a", Some(&text), &attributes);
            let a_shelf_id = self.shelve(a_text);
            let result = format!("{0}{1}{2}{3}", pre, a_shelf_id, pop, tight);
            result
        };


        let mut prev_text = Cow::Borrowed(text);
        let mut abort = false;
        while !abort && prev_text.contains(&needle) {
            let new_text = pattern.replace_all(&prev_text, &mut f_link);
            if new_text == prev_text {
                abort = true;
            }
            prev_text = new_text.into_owned().into()
        }
        prev_text
    }

    fn is_valid_url(&self, url: &str) -> bool {
        let uri_parts = UrlBits::parse(url);
        if uri_parts.scheme().is_empty() {
            true
        } else {
            let allowed_schemes = if self.textile.restricted {
                &RESTRICTED_URL_SCHEMES[..]
            } else {
                &UNRESTRICTED_URL_SCHEMES[..]
            };
            allowed_schemes.contains(&(uri_parts.scheme()))
        }
    }

    pub fn graf<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        let lite = self.textile.lite;
        let text = Cow::Borrowed(text);
        let text = if !lite {self.no_textile(&text).into()} else {text};
        let text = if !lite {self.code(&text).into()} else {text};
        let text = self.get_html_comments(&text);
        let text = self.get_refs(&text);
        let ltext = self.glyph_quoted_quote(&text);
        let text = self.links(&ltext);
        let text = if !self.textile.noimage {self.image(&text)} else {text.into()};
        let text = if !lite {self.table(&text)} else {text};
        let text = if !lite {self.redcloth_list(&text)} else {text};
        let text = if !lite { self.textile_lists(&text)} else {text };
        let text = self.span(&text);
        let text = self.footnote_ref(&text);
        let text = self.note_ref(&text);
        let text = self.glyphs(&text);
        Cow::Owned(text.trim_end_matches('\n').to_owned())
    }

    fn span<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        lazy_static! {
            static ref TAG_PATTERNS: [Regex; 10] = [
                span_re(r"\*\*"), span_re(r"\*"), span_re(r"\?\?"),
                span_re(r"\-"), span_re(r"__"), span_re(r"_"), span_re(r"%"),
                span_re(r"\+"), span_re(r"~"), span_re(r"\^")
            ];
        }
        self.span_depth += 1;
        let can_replace = self.span_depth <= self.textile.max_span_depth;

        let f_span = |cap: &Captures| -> String {
            // pre, tag, atts, cite, content, end, tail = match.groups()
            let tag = match &cap[2] {
                "*" => "strong",
                "**" =>"b",
                "??" =>"cite",
                "_" => "em",
                "__" =>"i",
                "-" => "del",
                "%" => "span",
                "+" => "ins",
                "~" => "sub",
                "^" => "sup",
                _ => unreachable!("Not allowed by the regex")
            };
            let atts = &cap[3];
            let mut html_atts = BlockAttributes::parse(atts, None, true, self.textile.restricted).html_attrs();
            if let Some(cite) = cap.get(4) {
                html_atts.insert("cite", cite.as_str().trim().to_owned());
            }
            let content = &cap[5];
            let content = self.span(content);
            let end = &cap[6];
            let (pre, tail) = get_special_options(
                unwrap_or_empty(cap.get(1)),
                unwrap_or_empty(cap.get(7)));
            let mut open_tag = String::from("<") + tag;
            join_html_attributes(&mut open_tag, &html_atts);
            open_tag.push('>');
            let close_tag = format!("</{}>", tag);
            let (open_tag_id, close_tag_id) = self.store_tags(open_tag, close_tag);
            String::from(pre) + &open_tag_id + &content + end + &close_tag_id + tail
        };

        let mut text = Cow::Borrowed(text);
        if can_replace {
            text = Cow::Owned(multi_replace_with_one(text, TAG_PATTERNS.iter(), f_span));
        }
        self.span_depth -= 1;
        text
    }

    fn store_tags(&mut self, open_tag: String, close_tag: String) -> (String, String) {
        self.ref_index += 1;
        self.ref_cache.insert(self.ref_index, open_tag);
        let open_tag_id = format!("{0}{1}:ospan ", self.textile.uid, self.ref_index);

        self.ref_index += 1;
        self.ref_cache.insert(self.ref_index, close_tag);
        let close_tag_id = format!(" {0}{1}:cspan", self.textile.uid, self.ref_index);
        (open_tag_id, close_tag_id)
    }

    fn retrieve_tags(&self, text: &str) -> String {
        let f_retrieve_tags = |cap: &Captures| -> String {
            let tag_id = cap[1].parse::<u32>().expect("must be an integer");
            self.ref_cache.get(&tag_id).cloned().unwrap_or_default()
        };
        let result = {
            let mut regex_cache = self.textile.regex_cache.borrow_mut();
            let open_tag_re: &Regex =
                regex_cache
                .entry(line!())
                .or_default()
                .entry("")
                .or_insert_with(
                    || fregex!(&format!("{0}(?P<token>[0-9]+):ospan ", self.textile.uid)));
            open_tag_re.replace_all(text, f_retrieve_tags)
        };
        let result = {
            let mut regex_cache = self.textile.regex_cache.borrow_mut();
            let close_tag_re: &Regex =
                regex_cache
                .entry(line!())
                .or_default()
                .entry("")
                .or_insert_with(
                    || fregex!(&format!(" {0}(?P<token>[0-9]+):cspan", self.textile.uid)));
            close_tag_re.replace_all(&result, f_retrieve_tags)
        };
        result.into_owned()
    }

    pub fn block<'b>(&mut self, text: &'b str) -> String {
        fn textile_block_re(block_tags_pattern: &str) -> Regex {
            fregex!(
                &format!(
                    concat!(r"(?s)^(?P<tag>{0})(?P<atts>{1}{2}{1})\.(?P<ext>\.?)",
                            r"(?::(?P<cite>\S+))? (?P<graf>.*)$"),
                    block_tags_pattern, *ALIGN_RE_S, *CLS_RE_S))
        }
        lazy_static! {
            static ref TEXTILE_TAG_RE: Regex = textile_block_re(
                BLOCK_TAGS_RE_S);
            static ref TEXTILE_LIGHT_TAG_RE: Regex = textile_block_re(
                BLOCK_TAGS_LITE_RE_S);
            static ref MULTI_ENDLINE_RE: Regex = fregex!(r"(\n{2,})");
            static ref BR_TAG_RE: Regex = fregex!(r"(?i)<br\s*?/?>");
        }
        let mut out = Vec::<Cow<'b, str>>::new();
        let tag_pattern: &Regex = if self.textile.lite {
            &TEXTILE_LIGHT_TAG_RE
        } else {
            &TEXTILE_TAG_RE
        };
        let mut whitespace = String::new();
        let mut eat_whitespace = false;
        let mut ext = "";
        let mut tag = "";
        let mut atts = "";
        let mut cite = None;
        let mut last_outer_closing = String::new();
        let mut eat = false;
        let textblocks = split_with_capture(&MULTI_ENDLINE_RE, text);
        for block in textblocks {
            if block.trim().is_empty() {
                if !eat_whitespace {
                    whitespace += block;
                }
                continue;
            }

            if ext.is_empty() {
                tag = "p";
                atts = "";
                cite = None;
                eat = false;
            }

            eat_whitespace = false;
            let mut is_anonymous_block = true;
            let block_output = if let Ok(Some(m)) = tag_pattern.captures(block) {
                is_anonymous_block = false;
                // Last block was extended, so close it
                if !ext.is_empty() {
                    if let Some(last_out) = out.last_mut() {
                        last_out.to_mut().push_str(&last_outer_closing);
                    }
                }
                tag = unwrap_or_empty(m.get(1));
                atts = unwrap_or_empty(m.get(2));
                ext = unwrap_or_empty(m.get(3));
                cite = m.get(4).as_ref().map(Match::as_str);
                let content = unwrap_or_empty(m.get(5));
                let bdata = Block::new(tag, atts, cite, content, self);
                eat = bdata.eat;
                last_outer_closing.replace_range(.., &bdata.outer_closing);

                bdata.outer_opening
                    + &bdata.inner_opening
                    + &bdata.content
                    + &bdata.inner_closing
                    + if ext.is_empty() { &bdata.outer_closing } else { "" }
            } else {
                let raw_block = DIVIDER_RE.is_match(block).unwrap_or_default();
                if !ext.is_empty() || (!block.starts_with(' ') && !raw_block) {
                    let bdata =  Block::new(tag, atts, cite, block, self);
                    eat = bdata.eat;
                    last_outer_closing.replace_range(.., &bdata.outer_closing);
                    // Skip outer tag because this is part of a continuing extended block
                    if bdata.content.is_empty() || (tag == "p" && !has_raw_text(&bdata.content)) {
                        bdata.content
                    } else {
                        bdata.inner_opening + &bdata.content + &bdata.inner_closing
                    }
                } else if raw_block && self.textile.restricted {
                    self.shelve(encode_html(block, self.textile.restricted, false))
                } else if raw_block {
                    self.shelve(block.to_owned())
                } else {
                    self.graf(block).into_owned()
                }
            };
            let block_output = self.do_p_br(&block_output);
            let block_output = whitespace.clone() + &BR_TAG_RE
                .replace_all(
                    &block_output,
                    self.textile.proper_br_tag());

            if !ext.is_empty() && is_anonymous_block {
                if let Some(last_out) = out.last_mut() {
                    last_out.to_mut().push_str(&block_output);
                }
            } else if !eat {
                out.push(block_output.into());
            }

            if eat {
                eat_whitespace = true;
            } else {
                whitespace.clear();
            }
        }
        if !ext.is_empty() {
            if let Some(last_output) = out.last_mut() {
                *last_output += last_outer_closing.as_str();
            }
        }
        out.join("")
    }

    fn glyph_quoted_quote<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        const QUOTE_STARTS: &str = "\"'({[«»‹›„‚‘”";
        lazy_static! {
            static ref PATTERN_RE: Regex = fregex!(
                &format!(" (?P<pre>[{}])(?P<quoted>\"?|\"[^\"]+)(?P<post>.) ",
                         fancy_regex::escape(QUOTE_STARTS)));
        }

        fn matching_quote(quote: char) -> Option<char> {
            match quote {
                '"' => Some('"'),
                '\'' => Some('\''),
                '(' => Some(')'),
                '{' => Some('}'),
                '[' => Some(']'),
                '«' => Some('»'),
                '»' => Some('«'),
                '‹' => Some('›'),
                '›' => Some('‹'),
                '„' => Some('“'),
                '‚' => Some('‘'),
                '‘' => Some('’'),
                '”' => Some('“'),
                _ => None
            }
        }

        let f_glyph_quoted_quote = |m: &Captures| -> String {
            // Check the correct closing character was found.
            let mut pre_char_buf = [0u8; 4];
            let mut post_char_buf = [0u8; 4];
            if let Some(pre_char) = m["pre"].chars().next() {
                if let Some(post_char) = m["post"].chars().next() {
                    if Some(post_char) != matching_quote(pre_char) {
                        return m[0].to_owned();
                    }
                    let new_pre = match pre_char {
                        '"' => "&#8220;",
                        '\'' => "&#8216;",
                        ' ' => "&nbsp;",
                        // a frugal replacement for char::to_string()
                        x => x.encode_utf8(&mut pre_char_buf)
                    };
                    let new_post = match post_char {
                        '"' => "&#8221;",
                        '\'' => "&#8217;",
                        ' ' => "&nbsp;",
                        x => x.encode_utf8(&mut post_char_buf)
                    };
                    let found = &m["quoted"];
                    let found: Cow<str> = if found.len() > 1 {
                        self.glyphs(found).trim_end().to_owned().into()
                    } else if found == "\"" {
                        "&quot;".into()
                    } else {
                        found.into()
                    };
                    return self.shelve(format!(" {new_pre}{found}{new_post} "))
                }
            }
            unreachable!("Should be reached, check regular expression");
        };
        PATTERN_RE.replace_all(text, f_glyph_quoted_quote)
    }

}


/// Determines which flavor of HTML the [`Textile`] parser will produce.
/// Check [`Textile::set_html_kind`] for details.
pub enum HtmlKind {
    XHTML,
    HTML5
}

type AmmoniaConfigurator = dyn for <'a, 'b> Fn(&'a mut AmmoniaBuilder<'b>) -> &'a AmmoniaBuilder<'b>;

/// The core structure responsible for converting Textile markup into HTML.
///
/// Example:
/// ```
/// use rustextile::{Textile, HtmlKind};
/// let textile = Textile::default()
///     .set_html_kind(HtmlKind::XHTML)
///     .set_restricted(true);
/// let html = textile.parse("h1. It works!");
/// assert_eq!(html, "<h1>It works!</h1>");
/// ```
pub struct Textile {
    uid: String,
    pub(crate) link_prefix: String,
    pub(crate) restricted: bool,
    pub(crate) raw_block_enabled: bool,
    pub(crate) align_class_enabled: Option<bool>,
    block_tags: bool,
    pub(crate) lite: bool,
    noimage: bool,
    get_sizes: bool,
    max_span_depth: u32,
    html_type: HtmlKind,
    rel: Option<String>,
    regex_cache: std::cell::RefCell<HashMap<u32, HashMap<&'static str, Regex>>>,
    dyn_glyph_replacers: [(Regex, String); 1],
    sanitizer_config: Option<Box<AmmoniaConfigurator>>,
}

fn normalize_newlines(text: &str) -> String {
    lazy_static! {
        static ref CHANGES: [(Regex, &'static str); 2] = [
            (fregex!(r"\r\n?"), "\n"),
            (fregex!(r"(?m)^[ \t]*\n"), "\n"),
        ];
    }
    multi_replace(text.into(), CHANGES.iter().map(|i| (&i.0, i.1)))
        .trim_matches('\n')
        .into()
}

fn time_based_uid() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut hasher = DefaultHasher::new();
    hasher.write_u128(now.as_nanos());
    format!("{:x}", hasher.finish())
}

const SNIP_NAB: &str = r"\p{Ll}";
lazy_static! {
    static ref DYN_3PLUS_RE: Regex = fregex!(
        &format!(
            concat!(
                r#"({space}|^|[>(;-])([{abr}]{{3,}})([{nab}]*)"#,
                r#"(?={space}|{pnct}|<|$)(?=[^">]*?(<|$))"#),
            space=SNIP_SPACE,
            abr=SNIP_ABR,
            nab=SNIP_NAB,
            pnct=PNCT_RE_S));
}

impl Default for Textile {
    fn default() -> Self {
        let result = Textile {
            link_prefix: String::new(), // to be filled by set_uid()
            uid: String::new(), // to be filled by set_uid()
            restricted: false,
            raw_block_enabled: false,
            align_class_enabled: None,
            block_tags: true,
            lite: false,
            noimage: false,
            get_sizes: false,
            max_span_depth: 5,
            html_type: HtmlKind::HTML5,
            rel: None,
            sanitizer_config: None,
            regex_cache: std::cell::RefCell::new(Default::default()),
            dyn_glyph_replacers: [
                // 3+ uppercase
                // will be properly filled later by set_uid
                (DYN_3PLUS_RE.clone(), String::new()),
            ]
        };
        result.set_uid(&time_based_uid())
    }
}

impl Textile {

    /// Given a Textile-formatted text, converts it into HTML (or XHTML,
    /// if [`set_html_kind`](Textile::set_html_kind)[`(HtmlKind::XHTML)`](HtmlKind::XHTML)
    /// was called previously).
    pub fn parse(&self, text: &str) -> String {

        if text.trim().is_empty() {
            return text.to_owned();
        }

        let text = if self.restricted {
            Cow::Owned(encode_html(text, false, false))
        } else {
            Cow::Borrowed(text)
        };

        let mut state = ParserState::new(self);
        let text = normalize_newlines(&text)
            .replace(&state.textile.uid, "");

        let text = if self.block_tags {
            let text = state.block(&text);
            state.place_note_lists(&text).into_owned()
        } else {
            let text = text + "\n\n";
            // Treat quoted quote as a special glyph.
            let text = state.glyph_quoted_quote(&text);
            // Inline markup (em, strong, sup, sub, del etc).
            let text = state.span(&text);
            // Glyph level substitutions (mainly typographic -- " & ' => curly
            // quotes, -- => em-dash etc.
            state.glyphs(&text).into_owned()
        };

        let text = state.retrieve(text);
        let text = text.replace(
            &format!("{0}:glyph:", &state.textile.uid),
            "");

        let text = state.retrieve_tags(&text);
        let text = state.retrieve_urls(&text);

        let text = match self.sanitizer_config {
            Some(ref configurator) =>
                configurator(
                    ammonia::Builder::default().link_rel(None)
                )
                .clean(&text)
                .to_string()
                .into(),
            None => text,
        };

        // if the text contains a break tag (<br> or <br />) not followed by
        // a newline, replace it with a new style break tag and a newline.
        lazy_static! {
            static ref BR_PATTERN: Regex = fregex!(r"<br( /)?>(?!\n)");
        }

        let text = BR_PATTERN.replace_all(
            &text,
            match self.html_type {
                HtmlKind::XHTML => "<br />\n",
                HtmlKind::HTML5 => "<br>\n",
            });

        let text = text.trim_end_matches('\n');

        text.to_string()
    }

    /// Enables automatic addition of `width` and `height` attributes
    /// to `<img .. />` image tags, based on their actual dimensions.
    /// This requires sending one HTTP requests per image to determine the size
    /// of each, though each request will fetch only a small chunk of the image
    /// (1 KiB) enough for determening its size.
    pub fn set_getting_image_size(mut self, value: bool) -> Self {
        self.get_sizes = value;
        self
    }

    /// Whether Textile block tags (such as `bc.`) should be parsed
    /// and processed. Enabled by default.
    pub fn set_block_tags(mut self, value: bool) -> Self {
        self.block_tags = value;
        self
    }

    /// Which flavor of HTML the parser should output: either XHTML or HTML5.
    /// This affects whether `<acronim>` or `<abbr>` will be used,
    /// `<br>` or `<br />` and so on.
    ///
    /// See also [`Textile::set_align_class`] for details on how the images
    /// will be handled in each case.
    pub fn set_html_kind(mut self, html_type: HtmlKind) -> Self {
        self.html_type = html_type;
        self
    }

    /// Controls the restricted mode, which (when enabled) forces the parser to
    ///
    /// * escape any raw HTML
    /// * ignores any potentially unsafe Textile attributes within the document
    ///   (the ones that force a particular "style", "class" or "id" within the HTML).
    /// * allows only certain URL schemes ("http", "https", "ftp", "mailto").
    ///
    /// Check also [`Textile::set_lite`], [`Textile::set_images`] and [`Textile::set_sanitize`],
    /// which provide alternative kinds of restrictions.
    pub fn set_restricted(mut self, value: bool) -> Self {
        self.restricted = value;
        self
    }

    /// Enables the "lite mode", which limits the set of allowed Textile
    /// blocks to paragraphs and blockquotes only.
    ///
    /// Check also [`Textile::set_images`], [`Textile::set_restricted`]
    /// or [`Textile::set_sanitize`] if you need to put more restrictions
    /// on how the parser handles its input.
    pub fn set_lite(mut self, value: bool) -> Self {
        self.lite = value;
        self
    }

    /// Controls whether images are allowed in the input.
    pub fn set_images(mut self, value: bool) -> Self {
        self.noimage = !value;
        self
    }

    /// Forces a certain "rel" property on all links processed by the parser.
    /// As an example, you can set it to `"nofollow"` to prevent search engines
    /// from scanning them.
    pub fn set_rel<S>(mut self, value: Option<S>) -> Self where S: AsRef<str> {
        self.rel = value.map(|v| v.as_ref().to_owned());
        self
    }

    /// Controls whether images must be aligned by using the `align`
    /// HTML5 attribute (which became deprecated in HTML5) or by adding
    /// an `"align-{left|rignt|center}"` class to the `<img>` tag.
    ///
    /// If not set, for XHTML the "align" attribute is going to be used,
    /// and for HTML5 the `align-...` class will be added instead.
    pub fn set_align_class(mut self, value: bool) {
        self.align_class_enabled = Some(value);
    }

    /// Enables and disables raw blocks.
    ///
    /// When raw blocks are enabled, any paragraph blocks wrapped in a tag
    /// not matching HTML block or phrasing tags will not
    /// be parsed, and instead is left as is.
    pub fn set_raw_blocks(mut self, value: bool) -> Self {
        self.raw_block_enabled = value;
        self
    }

    /// Controls the final extra HTML sanitation step, which is done by
    /// the [Ammonia](https://docs.rs/ammonia/latest/ammonia/) library. A quote
    /// from the Ammonia's documentation:
    ///
    /// > "Ammonia is designed to prevent cross-site scripting, layout breaking,
    /// > and clickjacking caused by untrusted user-provided HTML being mixed
    /// > into a larger web page"
    ///
    /// This sanitation will use the default Ammonia's settings. You can adjust
    /// them by calling [`Textile::adjust_sanitizer`] method.
    /// Also check [`Textile::set_images`], [`Textile::set_lite`]
    /// and [`Textile::set_restricted`] to learn about other safety measures.
    pub fn set_sanitize(mut self, enable: bool) -> Self {
        if enable {
            self.adjust_sanitizer(|sanitizer| sanitizer)
        } else {
            self.sanitizer_config = None;
            self
        }
    }

    /// Just like [`Textile::set_sanitize`] this method enables additional
    /// sanitation of the output HTML through the
    /// [Ammonia](https://docs.rs/ammonia/latest/ammonia/) library. But you can
    /// also configure the sanitizer yourself.
    ///
    /// Example:
    ///
    /// ```rust
    /// use rustextile::Textile;
    /// let parser = Textile::default()
    ///     .adjust_sanitizer(|sanitizer| sanitizer.link_rel(Some("noopener")));
    /// let html = parser.parse(r#""a link":https://example.com"#);
    /// assert_eq!(html, r#"<p><a href="https://example.com/" rel="noopener">a link</a></p>"#);
    /// ```
    pub fn adjust_sanitizer<F>(mut self, configurator: F) -> Self
        where for <'a, 'b> F: Fn(&'a mut AmmoniaBuilder<'b>) -> &'a AmmoniaBuilder<'b> + 'a
    {
        self.sanitizer_config = Some(Box::new(configurator));
        self
    }

    /// Allows to control a small random token which is used by the parser
    /// internally to construct unique HTML id attributes and links necessary
    /// for footnotes.
    ///
    /// Normally you don't need to use this method. Its intended purpose
    /// is to guarantee stable outputs in automated tests.
    pub fn set_uid(mut self, base_id: &str) -> Self {
        self.uid = format!("textileRef:{0}:", base_id);
        self.link_prefix = format!("{0}-", base_id);
        let dyn_3plus_replacement = format!(
            r#"$1<span class="caps">{0}:glyph:$2</span>$3"#,
            &self.uid);
        self.dyn_glyph_replacers = [
            // 3+ uppercase
            (DYN_3PLUS_RE.clone(), dyn_3plus_replacement),
        ];
        self
    }

    pub(crate) fn proper_br_tag(&self) -> &str {
        match self.html_type {
            HtmlKind::XHTML => "<br />",
            HtmlKind::HTML5 => "<br>",
        }
    }
}

#[cfg(test)]
mod test {
    use super::get_image_size;

    #[test]
    fn test_get_image_size() {
        // Getting a real image
        let url = "https://en.wikipedia.org/favicon.ico";
        let size = get_image_size(url);
        assert_ne!(size, None);
        if let Some((width, height)) = size {
            assert!(width > 0);
            assert!(height > 0);
        }

        // Getting an impossible image
        let size = get_image_size("../picture.jpg");
        assert_eq!(size, None);
    }

    #[test]
    fn test_footnote_ref() {
        let t = super::Textile::default();
        let mut state = super::ParserState::new(&t);
        let result = state.footnote_ref("foo[1]");
        let expect = format!(
            "foo<sup class=\"footnote\" id=\"fnrev{0}1\"><a href=\"#fn{0}1\">1</a></sup>",
            t.link_prefix);
        assert_eq!(result, expect);
    }
}
