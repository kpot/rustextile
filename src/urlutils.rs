//! This module represents a simple wrapper arount `url::Url` which makes
//! it possible to parse relative URLs transparently, somethinging that
//! the [`url`](https://github.com/servo/rust-url) library doesn't want to do
//! at the time.
//! The reason why this contraption was created instead of using
//! [urlparse](https://github.com/yykamei/rust-urlparse) is because the
//! `url` libary is a well supported and robust way to not only parse URLs,
//! but to normalize them as well.
//! Non-valid urls are percent-encoded and interpreted as relative, to make them
//! inoperable but visible to the content creator.

use std::borrow::Cow;

use lazy_static::lazy_static;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::htmltools::encode_html;

/// A wrapper around the [url](https://docs.rs/url/latest/url/) library that
/// makes it possible to work with relative URLs. For that a hidden dummy base
/// URL will be added internally to "complete" relative URLs, and then stripped
/// again when `to_string` method is called.
/// For relative urls the `scheme` method will return empty strings.
#[derive(Debug)]
pub(crate) enum UrlBits {
    AbsoluteUrl(url::Url),
    RelativeUrl {
        url: url::Url,
        source: String,
    },
}

// A discardable pseudo "base url" that is parsed before a relative
// URL is joined to it.
const PSEUDO_BASE: &str = "http://example.com";
lazy_static! {
    static ref BASE: url::Url = url::Url::parse(PSEUDO_BASE)
        .expect("A valid url");
}

impl UrlBits {
    fn make_relative_url(url: &str) -> std::io::Result<Self> {
        let new_url = BASE.join(url).map_err(
            |e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Self::RelativeUrl {
            url: new_url,
            source: url.to_owned()
        })
    }

    pub fn parse(url: &str) -> Self {
        match url::Url::parse(url) {
            Ok(u) => Self::AbsoluteUrl(u),
            Err(_) => {
                Self::make_relative_url(url).unwrap_or_else(|_| {
                    let safed_url = utf8_percent_encode(url, NON_ALPHANUMERIC).to_string();
                    Self::make_relative_url(&safed_url).unwrap_or_else(|_|
                        Self::RelativeUrl { url: BASE.clone(), source: "".into() }
                    )
                })
            },
        }
    }

    pub fn scheme(&self) -> &str {
        match self {
            UrlBits::AbsoluteUrl(url) => url.scheme(),
            UrlBits::RelativeUrl { .. } => "",
        }
    }

    pub fn is_relative(&self) -> bool {
        match self {
            UrlBits::AbsoluteUrl(_) => false,
            UrlBits::RelativeUrl {..} => true,
        }
    }
}


impl ToString for UrlBits {
    fn to_string(&self) -> String {
        match self {
            UrlBits::AbsoluteUrl(url) => url.to_string(),
            UrlBits::RelativeUrl {url, source} => {
                let str_url = url.to_string();
                match (str_url.find('?'), source.find('?')) {
                    (Some(url_pos), Some(src_pos)) => {
                        let normalized_query = &str_url[url_pos + 1..];
                        let source_start = source[..src_pos + 1].to_owned();
                        source_start + normalized_query
                    },
                    (_, _) => {
                        match (str_url.rfind('#'), source.rfind('#')) {
                            (Some(url_frag_pos), Some(src_frag_pos)) => {
                                let normal_frag = &str_url[url_frag_pos + 1..];
                                let source_start = source[..src_frag_pos + 1].to_owned();
                                source_start + normal_frag
                            },
                            (None, Some(src_frag_pos)) => {
                                source[..src_frag_pos].to_owned()
                            },
                            (_, _) => {
                                source.clone()
                            },
                        }
                    },
                }
            },
        }
    }
}

/// Replaces ordinary `String`s with URLs wherever possible.
/// This helps to ensure we don't have any any non-normalized URLs,
/// and that we perform parsing/normalization/merging only once.
#[derive(Clone, Debug)]
pub(crate) enum UrlString<'t> {
    Normalized(Cow<'t, str>),
    Raw(Cow<'t, str>)
}

impl <'t> From<String> for UrlString<'t> {
    fn from(source: String) -> Self {
        Self::Raw(Cow::Owned(source))
    }
}

impl <'t> From<Cow<'t, str>> for UrlString<'t> {
    fn from(source: Cow<'t, str>) -> Self {
        Self::Raw(source)
    }
}

impl <'t> ToString for UrlString<'t> {
    fn to_string(&self) -> String {
        match self {
            Self::Normalized(url_text) => url_text.clone().into_owned(),
            Self::Raw(url_text) => {
                if url_text.is_empty() {
                    String::new()
                } else {
                    UrlBits::parse(url_text).to_string()
                }
            }
        }
    }
}

impl <'t> UrlString<'t> {
    pub(crate) fn source(&self) -> &Cow<'t, str> {
       match self {
           UrlString::Normalized(t) => t,
           UrlString::Raw(t) => t,
       }
    }

    pub(crate) fn to_html_string(&self) -> String {
        encode_html(&self.to_string(), true, true)
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;
    use crate::urlutils::{UrlBits, UrlString};

    #[test]
    fn test_url_bits() {
        // A valid relative URL
        let bits = UrlBits::parse("http://example.com/&.html");
        assert!(!bits.is_relative());
        assert_eq!(bits.scheme(), "http");
        assert_eq!(bits.to_string(), "http://example.com/&.html");

        assert_eq!(UrlString::from(Cow::Borrowed("http://example.com/<&test>.html")).to_html_string(),
                   "http://example.com/%3C&amp;test%3E.html");

        let bits = UrlBits::parse("http://example.com/<script>window.alert(\"Hello World!\");</script>.png");
        assert_eq!(bits.to_string(), "http://example.com/%3Cscript%3Ewindow.alert(%22Hello%20World!%22);%3C/script%3E.png");

        let bits = UrlBits::parse("some_page.html?q=Some query#Some text");
        assert!(bits.is_relative());
        assert_eq!(bits.scheme(), "");
        assert_eq!(bits.to_string(), "some_page.html?q=Some%20query#Some%20text");

        // Another valid relative URL
        let bits = UrlBits::parse("../../some_page.html#Some text");
        assert!(bits.is_relative());
        assert_eq!(bits.scheme(), "");
        assert_eq!(bits.to_string(), "../../some_page.html#Some%20text");

        // A valid absolute URL
        let bits = UrlBits::parse("https://example.com/some_page.html?q=Some query#Some text");
        assert!(!bits.is_relative());
        assert_eq!(bits.scheme(), "https");
        assert_eq!(bits.to_string(), "https://example.com/some_page.html?q=Some%20query#Some%20text");

        // An invalid absolute URL (should be automatically escaped and interpreted as relaive)
        let bits = UrlBits::parse("https:::://example.com/some_page.html?q=Some query#Some text");
        assert!(bits.is_relative());
        assert_eq!(bits.scheme(), "");
        assert_eq!(bits.to_string(), "https%3A%3A%3A%3A%2F%2Fexample%2Ecom%2Fsome%5Fpage%2Ehtml%3Fq%3DSome%20query%23Some%20text");
    }

}
