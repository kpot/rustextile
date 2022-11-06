use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

use lazy_static::lazy_static;
use fancy_regex::{Regex, Captures};

use crate::html::HTML5;
use crate::regextra::fregex;
use crate::regex_snips::{BLOCK_CONTENT, DIVIDER_RE};

pub(crate) fn encode_html(text: &str, quotes: bool, line_spacers: bool) -> String {
    let mut result = String::with_capacity(2 * text.len());
    let pattern = if quotes {
        if line_spacers {
            &['&', '<', '>', '"', '\'', '\n', '\r', '\t'][..]
        } else {
            &['&', '<', '>', '"', '\''][..]
        }
    } else if line_spacers {
        &['&', '<', '>', '\n', '\r', '\t'][..]
    } else {
        &['&', '<', '>'][..]
    };
    let mut leftover = text;
    while let Some(sep_index) = leftover.find(pattern) {
        result.push_str(&leftover[0..sep_index]);
        let sep = &leftover[sep_index..sep_index+1];
        let replacement = match sep {
            "&" => "&amp;",
            "<" => "&lt;",
            ">" => "&gt;",
            "\"" => "&quot;",
            "'" => "&#39;",
            "\n" => "&#13;",
            "\r" => "&#10;",
            "\t" => "&#9;",
            _ => unreachable!("An impossible symbol to encode: {}", sep)
        };
        result.push_str(replacement);
        leftover = &leftover[sep_index + 1..];
    }
    result.push_str(leftover);
    result
}

pub(crate) fn reverse_encode_html(text: &str) -> Cow<str> {
    lazy_static! {
        static ref ENTITY_RE: Regex = fregex!(
            "(&(?:amp|lt|gt|quot|#39|#13|#10|#9);)");
    }
    ENTITY_RE.replace_all(text, |cap: &Captures| {
        let entity = &cap[1];
        match entity {
            "&lt;" => "<",
            "&gt;" => ">",
            "&quot;" => "\"",
            "&#39;" => "'",
            "&#13;" => "\n",
            "&#10;" => "\r",
            "&#9;" => "\t",
            _ => unreachable!("Entity {entity:#?} must be part of the regular expression")
        }
    })
}

/// Escapes and quotes an XML/HTML attribute value.
/// Functional analog of xml.sax.saxutils.quoteattr from Python3
pub(crate) fn quoteattr(data: &str) -> String {
    let data = encode_html(data, false, true);
    if data.contains('"') {
        if data.contains('\'') {
            format!("\"{}\"", data.replace('"', "&quot;"))
        } else {
            format!("'{}'", data)
        }
    } else {
        format!("\"{}\"", data)
    }
}

// Based on [the latest HTML standard](https://html.spec.whatwg.org/multipage/syntax.html#attributes-2)
fn is_valid_attribute_char(c: char) -> bool {
    !(c.is_control()
      || c.is_whitespace()
      || ('\u{FDD0}'..='\u{FDEF}').contains(&c)
      || c == '='
      || c == '/'
      || c == '>'
      || c == '"'
      || c == '\'')
}


pub(crate) fn join_html_attributes(result: &mut String, attributes: &[(String, String)]) {
    let valid_attrs = attributes.iter().filter(|(name, _)| name.chars().all(is_valid_attribute_char));
    for (aname, avalue) in valid_attrs {
        result.push(' ');
        result.push_str(aname);
        result.push('=');
        result.push_str(&quoteattr(avalue));
    }
}

pub(crate) trait AsOptionStr {
    fn as_option_str(&self) -> Option<&str>;
}

impl AsOptionStr for &Option<String> {
    fn as_option_str(&self) -> Option<&str> {
        self.as_deref()
    }
}

impl AsOptionStr for &str {
    fn as_option_str(&self) -> Option<&str> {
        Some(*self)
    }
}

impl AsOptionStr for &String {
    fn as_option_str(&self) -> Option<&str> {
        Some(self.as_str())
    }
}

// Generates a complete HTML tag with a given name, attributes and content.
// Any of the attributes containing "illegal" characters won't be added.
// If the tag`s name contains invalid characters, whole content will be "safed"
// (by `encoded_html`) and returned instead.
pub(crate) fn generate_tag<S>(
    tag: S, content: Option<&str>, attributes: &[(String, String)]
) -> String
    where S: AsOptionStr
{
    if let Some(tag) = tag.as_option_str() {
        if tag.is_empty() {
            return content.unwrap_or_default().to_owned();
        }
        if !tag.chars().all(char::is_alphanumeric) {
            return encode_html(content.unwrap_or_default(), true, false);
        }

        let mut result = String::from("<") + tag;
        join_html_attributes(&mut result, attributes);
        match content {
            Some(text) => {
                result.push('>');
                result.push_str(text);
                result.push_str("</");
                result.push_str(tag);
                result.push('>');
            },
            None => {
                result.push_str(" />");
            },
        }
        result
    } else {
        content.unwrap_or_default().to_owned()
    }
}

lazy_static! {
    static ref INVALID_CHARREFS: HashMap<u32, char> = HashMap::from([
        (0x00, '\u{fffd}'),  // REPLACEMENT CHARACTER
        (0x0d, '\r'),      // CARRIAGE RETURN
        (0x80, '\u{20ac}'),  // EURO SIGN
        (0x81, '\u{81}'),    // <control>
        (0x82, '\u{201a}'),  // SINGLE LOW-9 QUOTATION MARK
        (0x83, '\u{0192}'),  // LATIN SMALL LETTER F WITH HOOK
        (0x84, '\u{201e}'),  // DOUBLE LOW-9 QUOTATION MARK
        (0x85, '\u{2026}'),  // HORIZONTAL ELLIPSIS
        (0x86, '\u{2020}'),  // DAGGER
        (0x87, '\u{2021}'),  // DOUBLE DAGGER
        (0x88, '\u{02c6}'),  // MODIFIER LETTER CIRCUMFLEX ACCENT
        (0x89, '\u{2030}'),  // PER MILLE SIGN
        (0x8a, '\u{0160}'),  // LATIN CAPITAL LETTER S WITH CARON
        (0x8b, '\u{2039}'),  // SINGLE LEFT-POINTING ANGLE QUOTATION MARK
        (0x8c, '\u{0152}'),  // LATIN CAPITAL LIGATURE OE
        (0x8d, '\u{8d}'),    // <control>
        (0x8e, '\u{017d}'),  // LATIN CAPITAL LETTER Z WITH CARON
        (0x8f, '\u{8f}'),    // <control>
        (0x90, '\u{90}'),    // <control>
        (0x91, '\u{2018}'),  // LEFT SINGLE QUOTATION MARK
        (0x92, '\u{2019}'),  // RIGHT SINGLE QUOTATION MARK
        (0x93, '\u{201c}'),  // LEFT DOUBLE QUOTATION MARK
        (0x94, '\u{201d}'),  // RIGHT DOUBLE QUOTATION MARK
        (0x95, '\u{2022}'),  // BULLET
        (0x96, '\u{2013}'),  // EN DASH
        (0x97, '\u{2014}'),  // EM DASH
        (0x98, '\u{02dc}'),  // SMALL TILDE
        (0x99, '\u{2122}'),  // TRADE MARK SIGN
        (0x9a, '\u{0161}'),  // LATIN SMALL LETTER S WITH CARON
        (0x9b, '\u{203a}'),  // SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
        (0x9c, '\u{0153}'),  // LATIN SMALL LIGATURE OE
        (0x9d, '\u{9d}'),    // <control>
        (0x9e, '\u{017e}'),  // LATIN SMALL LETTER Z WITH CARON
        (0x9f, '\u{0178}'),  // LATIN CAPITAL LETTER Y WITH DIAERESIS
    ]);

}

fn is_invalid_codepoint(cp: u32) -> bool {
    matches!(cp,
        0x0001..=0x0008 | 0x000E..=0x001F | 0x007F..=0x009F | 0xFDD0..=0xFDEF
        | 0xb | 0xfffe | 0xffff | 0x1fffe | 0x1ffff | 0x2fffe | 0x2ffff
        | 0x3fffe | 0x3ffff | 0x4fffe | 0x4ffff | 0x5fffe |  0x5ffff
        |  0x6fffe |  0x6ffff |  0x7fffe |  0x7ffff | 0x8fffe |  0x8ffff
        |  0x9fffe |  0x9ffff |  0xafffe |  0xaffff |  0xbfffe |  0xbffff
        | 0xcfffe |  0xcffff |  0xdfffe |  0xdffff |  0xefffe |  0xeffff
        | 0xffffe |  0xfffff | 0x10fffe |  0x10ffff)
}


fn replace_charref(s: &Captures) -> String {
    let s = &s[1];
    if let Some(stripped) = s.strip_prefix('#') {
        // numeric charref
        let num = match s.chars().nth(1) {
            Some('x') | Some('X') => u32::from_str_radix(s[2..].trim_end_matches(';'), 16),
            _ => u32::from_str(stripped.trim_end_matches(';'))
        }.expect("Must be convertible to int");

        if let Some(v) = INVALID_CHARREFS.get(&num) {
            v.to_string()
        } else if (0xD800..=0xDFFF).contains(&num) || num > 0x10FFFF {
            "\u{FFFD}".to_string()
        } else if is_invalid_codepoint(num) {
            "".to_string()
        } else {
            char::from_u32(num).expect("A valid char").to_string()
        }
    } else {
        // named charref
        if let Some(v) = HTML5.get(s) {
            v.to_string()
        } else {
            // find the longest matching name (as defined by the standard)
            if s.len() > 1 {
                let mut x = s.len() - 1;
                while x > 1 {
                    if let Some(m) = HTML5.get(&s[..x]) {
                        return m.to_string() + &s[x..];
                    }
                    x -= 1;
                }
            }
            "&".to_string() + s
        }
    }
}

/// A full equivalent of `html.unescape` from Python. Transforms a string
/// by replacing "escaped" HTML characters (such as `&gt;`) into their original
/// form (character `>` in this instance).
pub(crate) fn unescape(s: &str) -> Cow<str> {
    if !s.contains('&') {
        Cow::Borrowed(s)
    } else {
        lazy_static! {
            static ref CHARREF: Regex = fregex!(
                concat!(r"&(#[0-9]+;?",
                        r"|#[xX][0-9a-fA-F]+;?",
                        r"|[^\t\n\f <&#;]{1,32};?)"));
        }
        CHARREF.replace_all(s, replace_charref)
    }
}


pub(crate) fn has_raw_text(text: &str) -> bool {
    const PHRASING_CONTENT: &str = concat!(
        "abbr|acronym|area|audio|a|bdo|br|button|b|canvas|cite|code|command|",
        "data|datalist|del|dfn|em|embed|iframe|img|input|ins|i|kbd|keygen|",
        "label|link|map|mark|math|meta|meter|noscript|object|output|progress|",
        "q|ruby|samp|script|select|small|span|strong|sub|sup|svg|textarea|",
        "time|var|video|wbr",
    );
    lazy_static! {
        static ref UNWRAPPABLE_RE: Regex = fregex!(
            &format!(r"(?si)</?(?:{0})(?:\s[^<>]*?|/?)>", BLOCK_CONTENT));
        static ref WRAPPED_RE: Regex = fregex!(
            r"(?si)^</?([^\s<>/]+)[^<>]*?>(?:.*</\1\s*?>)?$");
        static ref PHRASING_RE: Regex = fregex!(
            &format!(r"(?i)^(?:{0})$", PHRASING_CONTENT));
    }


    if UNWRAPPABLE_RE.is_match(text).unwrap_or_default()
            || DIVIDER_RE.is_match(text).unwrap_or_default() {
        false
    } else if let Some(m) = WRAPPED_RE.captures(text).unwrap_or_default() {
        PHRASING_RE.is_match(&m[1]).unwrap_or_default()
    } else {
        true
    }
}


#[cfg(test)]
mod tests {
    use super::{quoteattr, unescape, encode_html, has_raw_text};

    #[test]
    fn test_quoteattr() {
        assert_eq!(
            quoteattr("So called \"escaped\"\nmulti-line <value>"),
            "'So called \"escaped\"&#13;multi-line &lt;value&gt;'");
    }

    #[test]
    fn test_unescape() {
        let original = r#"<a href="http://example.com">Some&nbsp;link</a>"#;
        let escaped = encode_html(original, true, false);
        assert_eq!(escaped, "&lt;a href=&quot;http://example.com&quot;&gt;Some&amp;nbsp;link&lt;/a&gt;");
        let unescaped = unescape(&escaped);
        assert_eq!(unescaped, original);
    }

    #[test]
    fn test_has_raw_text() {
        assert!(!has_raw_text("<p>foo bar biz baz</p>"));
        assert!(has_raw_text(" why yes, yes it does"));
    }
}
