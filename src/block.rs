use std::borrow::Cow;

use lazy_static::lazy_static;
use fancy_regex::{Regex, Captures};

use crate::regextra::fregex;
use crate::htmltools::quoteattr;
use crate::regex_snips::{SNIP_SPACE, SNIP_DIGIT, CLS_RE_S, VALIGN_RE_S, HALIGN_RE_S};
use crate::htmltools::{generate_tag, encode_html};
use crate::parser::ParserState;


#[derive(Default, Debug, Clone)]
pub(crate) struct BlockHtmlAttributes (Vec<(String, String)>);

impl BlockHtmlAttributes {
    pub fn insert(&mut self, key: &str, value: String) {
        match self.0.binary_search_by_key(&key, |item| &item.0) {
            Ok(index) => {
                self.0[index].1 = value;
            },
            Err(insertion_index) => {
                self.0.insert(insertion_index, (key.into(), value));
            }
        }
    }

    pub fn insert_css_class<S>(&mut self, name: S) -> bool
        where S: AsRef<str>
    {
        lazy_static! {
            static ref CSS_CLASS_NAME_RE: Regex = fregex!(
                r"^([-a-zA-Z 0-9_\/\[\].:!#]+)$");
        }
        let trimmed_name = name.as_ref().trim();
        if CSS_CLASS_NAME_RE.is_match(trimmed_name).unwrap_or_default() {
            match self.0.binary_search_by_key(&"class", |item| &item.0) {
                Ok(index) => {
                    let content = &mut self.0[index].1;
                    content.push(' ');
                    content.push_str(trimmed_name)
                },
                Err(insertion_index) => {
                    self.0.insert(
                        insertion_index,
                        ("class".into(), trimmed_name.into()));
                }
            }
            true
        } else {
            false
        }
    }
}

impl std::ops::AddAssign<(&str, Option<String>)> for BlockHtmlAttributes {
    fn add_assign(&mut self, rhs: (&str, Option<String>)) {
        if let (k, Some(v)) = rhs {
            self.insert(k, v);
        }
    }
}


impl std::ops::Deref for BlockHtmlAttributes {
    type Target = [(String, String)];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl ToString for BlockHtmlAttributes {
    fn to_string(&self) -> String {
        let mut result = String::new();
        for (key, value) in self.0.iter() {
            result.push(' ');
            result.push_str(key);
            result.push('=');
            result.push_str(&quoteattr(value));
        }
        result
    }
}


#[derive(Debug, Clone, Default)]
pub(crate) struct BlockAttributes {
    pub colspan: Option<String>,
    pub style: Option<String>,
    pub class: Option<String>,
    pub id: Option<String>,
    pub rowspan: Option<String>,
    pub lang: Option<String>,
    pub span: Option<String>,
    pub width: Option<String>,
}

impl BlockAttributes {
    pub fn parse(block_attributes: &str, element: Option<&str>, include_id: bool, restricted: bool) -> Self {
        lazy_static! {
            static ref COLSPAN_RE: Regex = fregex!(r"\\(\d+)");
            static ref ROWSPAN_RE: Regex = fregex!(r"/(\d+)");
            static ref ATTR_VALIGN_RE: Regex = fregex!(&format!(r"^{}", VALIGN_RE_S));
            static ref ATTR_STYLE_RE: Regex = fregex!(r"\{([^}]*)\}");
            static ref ATTR_LANG_RE: Regex = fregex!(r"\[([^\]]+)\]");
            static ref ATTR_ACLASS_RE: Regex = fregex!(r"\(([^()]+)\)");
            static ref CSS_ID_RE: Regex = fregex!(r"^([-a-zA-Z0-9_\.\:]*)$");
            static ref ATTR_PADDING_LEFT_RE: Regex = fregex!(r"([(]+)");
            static ref ATTR_PADDING_RIGHT_RE: Regex = fregex!(r"([)]+)");
            static ref ATTR_COL_RE: Regex = fregex!(r"^(?:\\(\d+)\.?)?\s*(\d+)?");
            static ref CSS_CLASSES_RE: Regex = fregex!(r"^([-a-zA-Z 0-9_\.\/\[\]:!]*)$");
        }
        let mut style = Vec::<String>::new();

        if block_attributes.is_empty() {
            return Self {
                colspan: None,
                style: None,
                class: None,
                id: None,
                rowspan: None,
                lang: None,
                span: None,
                width: None,
            }
        }

        let mut matched = block_attributes.to_owned();
        let (colspan, rowspan) = if element == Some("td") {
            (COLSPAN_RE.captures(&matched).unwrap_or(None).map(|m| m[1].to_owned()),
             ROWSPAN_RE.captures(&matched).unwrap_or(None).map(|m| m[1].to_owned()))
        } else {
            (None, None)
        };

        if element == Some("td") || element == Some("tr") {
            if let Ok(Some(m)) = ATTR_VALIGN_RE.find(&matched) {
                let alignment = match m.as_str() {
                    "^" => "top",
                    "-" => "middle",
                    "~" => "bottom",
                    _ => unreachable!("Unsupported table vertical alignment: {}", m.as_str())
                };
                style.push(format!("vertical-align:{}", alignment));
            }
        }

        if !restricted {
            if let Ok(Some(m)) = ATTR_STYLE_RE.captures(&matched) {
                style.extend(
                    m[1].trim_end_matches(';')
                        .split(';')
                        .map(|n| n.trim().to_owned()));
                matched = matched.replace(&m[0], "");
            }
        }

        let lang = match ATTR_LANG_RE.captures(&matched) {
            Ok(Some(m)) => {
                let result = Some(m[1].to_owned());
                matched = matched.replace(&m[0], "");
                result
            }
            _ => None,
        };

        let (aclass, block_id) = match ATTR_ACLASS_RE.captures(&matched) {
            Ok(Some(m)) => {
                let id_class_mix = &m[1];
                let result = match id_class_mix.split_once('#') {
                    // No # separator founc
                    None => (
                        // classes
                        match CSS_CLASSES_RE.is_match(id_class_mix) {
                            Ok(true) => Some(id_class_mix.to_owned()),
                            _ => None,
                        },
                        // id
                        None
                    ),
                    // attibute is separable by # into left and right sides
                    Some((left, right)) => (
                        // classes
                        if !left.is_empty() {
                            match CSS_CLASSES_RE.is_match(left) {
                                Ok(true) => Some(left.to_owned()),
                                _ => None,
                            }
                        } else {
                            None
                        },
                        // id
                        match CSS_ID_RE.is_match(right) {
                            Ok(true) => Some(right.to_owned()),
                            _ => None,
                        }
                    )
                };

                matched = matched.replace(&m[0], "");
                if restricted { (None, None) } else { result }
            },
            _ => (None, None)
        };

        if let Ok(Some(m)) = ATTR_PADDING_LEFT_RE.captures(&matched) {
            style.push(format!("padding-left:{}em", m[1].len()));
            matched = matched.replace(&m[0], "");
        }

        if let Ok(Some(m)) = ATTR_PADDING_RIGHT_RE.captures(&matched) {
            style.push(format!("padding-right:{}em", m[1].len()));
            matched = matched.replace(&m[0], "");
        }

        lazy_static! {
            static ref ATTR_HALIGN_RE: Regex = fregex!(
                &format!(r"({})", HALIGN_RE_S));
        }
        if let Ok(Some(m)) = ATTR_HALIGN_RE.captures(&matched) {
            let alignment = match &m[1] {
                "<" => "left",
                "=" => "center",
                ">" => "right",
                "<>" => "justify",
                value => unreachable!(
                    "Unexpected block horizontal alignment: {}", value),
            };
            style.push(format!("text-align:{}", alignment));
        }

        let (span, width) = if element == Some("col") {
            match ATTR_COL_RE.captures(&matched) {
                Ok(Some(c)) => (
                    c.get(1).map(|m| m.as_str().to_owned()),
                    c.get(2).map(|m| m.as_str().to_owned()),
                ),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        Self {
            colspan,
            rowspan,
            lang,
            span,
            width,
            id: if include_id { block_id } else { None },
            style: if style.is_empty() { None } else { Some(style.join("; ") + ";") },
            class: aclass,
        }
    }

    pub fn html_attrs(self) -> BlockHtmlAttributes {
        let mut chunks = BlockHtmlAttributes::default();
        chunks += ("class", self.class);
        chunks += ("colspan", self.colspan);
        chunks += ("id", self.id);
        chunks += ("lang", self.lang);
        chunks += ("rowspan", self.rowspan);
        chunks += ("span", self.span);
        chunks += ("style", self.style);
        chunks += ("width", self.width);
        chunks
    }

}

impl From<BlockAttributes> for String {
    fn from(ba: BlockAttributes) -> String {
        ba.html_attrs().to_string()
    }
}

#[derive(Debug)]
pub(crate) struct Block {
    pub outer_opening: String,
    pub outer_closing: String,
    pub inner_opening: String,
    pub inner_closing: String,
    pub content: String,
    pub eat: bool,
}

impl Block {
    pub fn new<S>(
        tag: &str,
        attrs: &str,
        cite: Option<S>,
        content: &str,
        ps: &mut ParserState
    ) -> Self
        where S: AsRef<str>
    {
        lazy_static! {
            static ref FNID_RE: Regex = fregex!(&format!(r"fn(?P<fnid>{0}+)", SNIP_DIGIT));
            static ref CODE_LANG_RE: Regex = fregex!(r"^[a-zA-Z0-9_-]+$");
        }
        let cite = cite.map(|v| v.as_ref().to_owned());
        let mut new_content = Cow::Borrowed(content);
        let mut eat = false;
        let mut attributes = BlockAttributes::parse(attrs, None, true, ps.textile.restricted);
        let orig_html_attributes = attributes.clone().html_attrs();

        let mut inner_opening = String::new();
        let mut inner_closing = String::new();
        let mut outer_opening = String::new();
        let mut outer_closing = String::new();
        if tag == "p" {
            // is this an anonymous block with a note definition?
            lazy_static! {
                static ref NOTEDEF_RE: Regex = fregex!(
                    &format!(
                        concat!(
                            r"^note#", // start of note def marker
                            r"(?P<label>[^%<*!@#^(\[{{ {space}.]+)", // label
                            r"(?P<link>[*!^]?)", // link
                            r"(?P<att>{cls})", // att
                            r"\.?", // optional period.
                            r"[{space}]+", // whitespace ends def marker
                            r"(?P<content>.*)$", // content""".format(
                        ),
                        space=SNIP_SPACE, cls=*CLS_RE_S));
            };
            let notedef = NOTEDEF_RE.replace_all(
                &new_content,
                |matches: &Captures| { ps.parse_note_defs(matches) });
            if notedef.is_empty() {
                return Block {
                    inner_opening,
                    inner_closing,
                    outer_opening,
                    outer_closing,
                    eat: true,
                    content: notedef.into_owned(),
                };
            }

        }
        let new_tag = if let Ok(Some(m)) = FNID_RE.captures(tag) {
            let m_fnid = &m["fnid"];
            let fnid = ps
                .footnotes
                .get(m_fnid)
                .cloned()
                .unwrap_or_else(|| {
                    let new_index = ps.increment_link_index();
                    format!("{0}{1}", ps.textile.link_prefix, new_index)
                });

            let mut sup_html_attrs = BlockHtmlAttributes::default();

            // if class has not been previously specified, set it to "footnote"
            if attributes.class.is_none() {
                attributes.class = Some("footnote".to_string());
            }

            // if there's no specified id, use the generated one.
            if attributes.id.is_none() {
                let fn_tag_id = format!("fn{}", fnid);
                attributes.id = Some(fn_tag_id);
            } else  {
                sup_html_attrs.insert("id", format!("fn{}", fnid));
            }

            let sup = if !attrs.contains('^') {
                generate_tag("sup", Some(m_fnid), &sup_html_attrs)
            } else {
                let fnrev = generate_tag(
                    "a",
                    Some(m_fnid),
                    &[("href".to_owned(), format!("#fnrev{}", fnid))]);
                generate_tag("sup", Some(&fnrev), &sup_html_attrs)
            };
            new_content = format!("{} {}", sup, &new_content).into();
            "p"
        } else {
            tag
        };

        match new_tag {
            "bq" => {
                let mut html_attributes = attributes.html_attrs();
                if let Some(ref cite) = cite {
                    let shelved_url = ps.shelve_url(
                        ps.unrestrict_url(cite.as_str()).into());
                    html_attributes.insert("cite", shelved_url);
                }
                outer_opening = format!("<blockquote{0}>\n", html_attributes.to_string());
                inner_opening = format!("\t<p{0}>", orig_html_attributes.to_string());
                inner_closing = "</p>".into();
                outer_closing = "\n</blockquote>".into();
            },
            "bc" => {
                new_content = ps.shelve(encode_html(&new_content, true, false)).into();
                let mut inner_atts = BlockHtmlAttributes::default();
                if let Some(lang) = attributes.lang.clone() {
                    attributes.lang = None;
                    if let Ok(true) = CODE_LANG_RE.is_match(&lang) {
                        let code_attrs = BlockAttributes {
                            class: Some(lang),
                            ..Default::default()
                        };
                        inner_atts = code_attrs.html_attrs();
                    }
                }
                let outer_atts = attributes.html_attrs();
                outer_opening = format!("<pre{}><code{}>", outer_atts.to_string(), inner_atts.to_string());
                outer_closing = "</code></pre>".into();
            }
            "pre" => {
                new_content = ps.shelve(encode_html(&new_content, true, false)).into();
                outer_opening = format!("<pre{}>", attributes.html_attrs().to_string());
                outer_closing = "</pre>".into();
            },
            "notextile" => {
                new_content = ps.shelve(new_content.into_owned()).into();
            },
            "###" => {
                eat = true;
            },
            _ => {
                inner_opening = format!("<{}{}>", new_tag, attributes.html_attrs().to_string());
                inner_closing = format!("</{}>", new_tag);
            }
        }
        new_content = if !eat {
            ps.graf(&new_content).into_owned().into()
        } else {
            "".into()
        };
        Block {
            outer_opening,
            outer_closing,
            inner_opening,
            inner_closing,
            eat,
            content: new_content.into_owned(),
        }
    }

}


#[cfg(test)]
mod test {
    use crate::block::BlockHtmlAttributes;

    #[test]
    fn test_html_attributes_manipulation() {
        let mut atts = BlockHtmlAttributes::default();
        assert!(atts.to_string().is_empty());
        atts.insert("id", "id-value&data".into());
        assert_eq!(atts.to_string(), " id=\"id-value&amp;data\"");
        assert!(atts.insert_css_class("align-left "));
        assert_eq!(atts.to_string(), " class=\"align-left\" id=\"id-value&amp;data\"");
        assert!(atts.insert_css_class("otherclass"));
        assert_eq!(atts.to_string(), " class=\"align-left otherclass\" id=\"id-value&amp;data\"");
        assert!(!atts.insert_css_class("invalid@class@name"));
        assert_eq!(atts.to_string(), " class=\"align-left otherclass\" id=\"id-value&amp;data\"");
        assert!(atts.insert_css_class("md:mt-3"));
        assert_eq!(atts.to_string(), " class=\"align-left otherclass md:mt-3\" id=\"id-value&amp;data\"");
        assert!(atts.insert_css_class("!mt-3"));
        assert_eq!(atts.to_string(), " class=\"align-left otherclass md:mt-3 !mt-3\" id=\"id-value&amp;data\"");
    }
}
