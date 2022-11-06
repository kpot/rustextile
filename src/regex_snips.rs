//! Common snipets for various regular expressions.
//! Some of the expressions rely on [UNICODE character properties](https://www.unicode.org/reports/tr44/#GC_Values_Table).

use lazy_static::lazy_static;
use fancy_regex::Regex;

use crate::regextra::fregex;

// PHP and mrab-regex/Python regular expressions have slightly different
// meaning of some special character classes, like "\s" or "\v", different
// from their implementation in Rust regex libraries.
// For instance, "\v" in Rust-regex is a "vertical tab (\x0B)", while in PHP
// it matches "CR", "LF" and their combination.
// Meaning the expressions below had to be adapted and may differ from their
// original forms in PHP-textile.
// Also, various categories of characters (such as "\p{Pc}") are described here:
// https://www.unicode.org/reports/tr44/#GC_Values_Table
const CLASS_RE_S: &str = r"(?:\([^)\n]+\))";       // Don't allow classes/ids,
const STYLE_RE_S: &str = r"(?:\{[^}\n]+\})";       // or styles to span across newlines
const LANGUAGE_RE_S: &str = r"(?:\[[^\]\n]+\])";   // languages,
pub(crate) const SNIP_ACR: &str = r"\p{Lu}\p{Nd}";
pub(crate) const SNIP_DIGIT: &str = r"\p{N}";
pub(crate) const SNIP_SPACE: &str = r"\s";
pub(crate) const SNIP_WRD: &str = r"(?:\p{L}|\p{M}|\p{N}|\p{Pc})";
pub(crate) const SNIP_CUR: &str = r"\p{Currency_Symbol}";
pub(crate) const SNIP_CHAR: &str = r"\S";
pub(crate) const VALIGN_RE_S: &str = r"[\-^~]";
pub(crate) const HALIGN_RE_S: &str = r"(?:\<(?!>)|(?<!<)\>|\<\>|\=|[()]+(?! ))";
pub(crate) const UPPER_CHARS: &str = r"\p{Lu}";
pub(crate) const SNIP_ABR: &str = UPPER_CHARS;
pub(crate) const PNCT_RE_S: &str = r##"[-!"#$%&()*+,/:;<=>?@\'\[\\\]\.^_`{|}~]"##;

// Blocks containing any of these tags will not be wrapped in paragraphs.
// The php version has orders the below list of tags differently.  The
// important thing to note here is that the "pre" must occur before the "p",
// "section" before the "s" and so on, otherwise the regex module doesn't
// properly match pre-s. It only matches the p in pre.
pub(crate) const BLOCK_CONTENT: &str = concat!(
    "address|article|aside|blockquote|details|div|dl|fieldset|figure|figcaption",
    "|footer|form|h1|h2|h3|h4|h5|h6|header|hgroup|main|menu|nav|ol",
    "|pre|p|section|s|table|template|ul");

lazy_static! {
    // regex string to match class, style and language attributes
    pub(crate) static ref CLS_RE_S: String = format!(
        concat!(r"(?:",
                r"{c}(?:{l}(?:{s})?|{s}(?:{l})?)?|",
                r"{l}(?:{c}(?:{s})?|{s}(?:{c})?)?|",
                r"{s}(?:{c}(?:{l})?|{l}(?:{c})?)?",
                r")?"),
        c=CLASS_RE_S, s=STYLE_RE_S, l=LANGUAGE_RE_S);


    pub(crate) static ref ALIGN_RE_S: String = format!(r"(?:{0}|{1})*", HALIGN_RE_S, VALIGN_RE_S);
    // identifies standalone ampersands which are not parts of HTML entities
    pub(crate) static ref LONE_AMP_RE: Regex = fregex!(r"(?i)&(?!#[0-9]+;|#x[a-f0-9]+;|[a-z][a-z0-9]*;)");
    pub(crate) static ref DIVIDER_RE: Regex = fregex!(
        r"(?si)^(?:</?(br|hr|img)(?:\s[^<>]*?|/?)>(?:</\1\s*?>)?)+$");
}
