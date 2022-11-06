//! Rustextile is a parser of a popular [Textile](https://textile-lang.com/)
//! markup language written in stable Rust.
//!
//! It is mostly a port of the "canonical"
//! [PHP Textile](https://github.com/textile/php-textile) implementation
//! and supports all of its markup features (as of php-textile v3.7.7), including
//!
//! * Decorated text spans
//! * Images
//! * Tables
//! * Ordered/unordered lists
//! * Definition lists
//! * Complex quotations
//! * Code blocks
//! * CSS styles, classes and ID attributes
//! * Raw HTML inserts
//! * Footnotes and references
//! * "Restricted" parsing for untrusted user input
//! * Rendering in either XHTML or HTML5
//! * Extra [safety perks](Textile::set_sanitize) to ensure nothing harmful
//!   can be sneaked into the output even without the use of restricted parsing.
//!
//! # Usage
//!
//! Edit your `Cargo.toml` to include
//!
//! ```toml
//! [dependencies]
//! rustextile = "1"
//! ```
//! # Code Example
//!
//! ```rust
//! use rustextile::{Textile, HtmlKind};
//!
//! // Processing some ordinary Textile markup
//! let textile = Textile::default()
//!     .set_html_kind(HtmlKind::XHTML);
//!
//! let html = textile.parse("h1. It works!");
//! assert_eq!(html, "<h1>It works!</h1>");
//!
//! let html = textile.parse("<strong>Raw HTML insert</strong>");
//! assert_eq!(html, r#"<p><strong>Raw <span class="caps">HTML</span> insert</strong></p>"#);
//!
//! // Forcing all links to have a specific "rel" attribute
//! let textile = textile.set_rel(Some("nofollow"));
//! let html = textile.parse(r#"This "link":https://example.com/ won't be scanned by Google"#);
//! assert_eq!(html, r#"<p>This <a href="https://example.com/" rel="nofollow">link</a> won&#8217;t be scanned by Google</p>"#);
//!
//! // Same restrictions can be placed on the parser
//! let textile = Textile::default().set_restricted(true);
//! let html = textile.parse("<div>Now raw HTML is restricted</div>");
//! assert_eq!(html, r#"<p>&lt;div&gt;Now raw <span class="caps">HTML</span> is restricted&lt;/div&gt;</p>"#);
//!
//! // Extra sanitation of the output through the Ammonia library.
//! let textile = Textile::default()
//!     .set_sanitize(true);
//! let html = textile.parse(r#"<script type="text/javascript">alert("Say hi!")</script>JS has been sanitized away!"#);
//! assert_eq!(html, "<p>JS has been sanitized away!</p>");
//! // ... which can be finely tuned to do what you need exactly
//! let textile = textile.adjust_sanitizer(|s| s.rm_tags(&["p", "del"]));
//! let html = textile.parse("Sanitizer -can also- be tuned");
//! assert_eq!(html, "Sanitizer can also be tuned");
//!
//! ````

mod regextra;
mod htmltools;
mod charcounter;
mod block;
mod parser;
mod html;
mod table;
mod urlutils;
mod regex_snips;

pub use crate::parser::{Textile, HtmlKind, AmmoniaBuilder};
