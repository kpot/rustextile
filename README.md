# Rustextile - Rust Textile parser

Rustextile is a parser of a popular [Textile][1] markup language written in pure Rust.
It is a port of two libraries: the [python-textile][2] library
and the "canonical" [PHP Textile][3] implementation
(on which the python-textile library is based too).

## Functionality

This port passes the same automated tests as the original libraries do,
and supports the same full set of functionality, including

* Decorated text spans
* Images
* Tables
* Ordered/unordered lists
* Definition lists
* Complex quotations
* Code blocks
* CSS styles, classes and ID attributes
* Raw HTML inserts
* Footnotes and references
* "Restricted" parsing for untrusted user input and other safety perks
* Rendering in either XHTML or HTML5
* and [more...][1]

There is another similar Rust library called [textile-rs][4],
which was written from scratch, but sadly supports only basic capabilities of Textile
and is not fully compatible with documents created for more advanced canonical parser.

This implementation is a direct port of the canonical PHP parser. It uses a similar code structure, the same regular expressions, mostly the same variable names and the same tests fixtures.
This makes it not only more compatible, but also allows to back-port new features and fixes from still actively developing PHP version.

For me this was also a good demonstration that one can rewrite a PHP or Python code in Rust without sacrificing brevity and readability typical for such high-level interpreted languages.

[1]:https://textile-lang.com/
[2]:https://github.com/textile/python-textile
[3]:https://github.com/textile/php-textile
[4]:https://crates.io/crates/textile
