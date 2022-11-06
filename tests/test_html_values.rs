//! These tests were ported from the original python-textile library.

use pretty_assertions::assert_str_eq;

const HTML_RESTRICTED_MODE_VALUES: [(&str, &str); 2] = [
    // Ensure that there's no double-escaping of links while in the restricted mode
    (concat!("\"test\":link1\n",
             "[link1]http://example.com/<script>window.alert(\"HelloWorld!\");</script>\n"),
     "<p><a href=\"http://example.com/%3Cscript%3Ewindow.alert(%22HelloWorld!%22);%3C/script%3E\">test</a></p>"),
    ("!http://example.com/<name>.png!\n",
     "<p><img alt=\"\" src=\"http://example.com/%3Cname%3E.png\" /></p>"),
];


fn normalize_newlines(text: &str) -> String {
    text.trim().replace('\t', "").lines().map(|l| l.trim()).collect()
}

const HTML_KNOWN_VALUES: [(&str, &str); 52] = [
    // ("*Colors*\n* Red\n* Green\n* Blue",
    //  "<p><strong>Colors</strong></p><ul><li>Red</li><li>Green</li><li>Blue</li></ul>"),
    // Closely packed links
    ("[\"1\":https://example.tld][\"2\":https://example.tld][\"3\":https://example.tld]",
     "<p><a href=\"https://example.tld/\">1</a><a href=\"https://example.tld/\">2</a><a href=\"https://example.tld/\">3</a></p>"),
    // Multi-line HTML insert
    ("<div class=\"test\">\n\nSoy un *super* robot.\n\n</div>",
     "<div class=\"test\">\n\n<p>Soy un <strong>super</strong> robot.</p>\n\n</div>"),
    // Textile within an HTML span
    ("<span>\"link\":http://example.com</span>", "<p><span><a href=\"http://example.com/\">link</a></span></p>"),
    // Textile within a divider block
    ("<hr title=\"Hello *strong* world!\">", "<hr title=\"Hello *strong* world!\">"),
    // Double quote in links
    ("\"\"The use of the character \"\"\" in textile\"\":help.html..!",
     "<p><a href=\"help.html\">&#8220;The use of the character &#8220;&quot;&#8221; in textile&#8221;</a>..!</p>"),
    // A link with an array in square braces
    ("^[\"same\":https://github.com/netcarver/?lang=en&q[]]^",
     "<p><sup><a href=\"https://github.com/netcarver/?lang=en&amp;q[]\">same</a></sup></p>"),
    // Double escaping within blockquotes
    ("bq.. Don't suck the **brown stuff(tm)** off of \"2 pence\":http://royalmint.gov.uk coins; it ain't chocolate.\n\np. That's all.",
     "<blockquote>\n\t<p>Don&#8217;t suck the <b>brown stuff&#8482;</b> off of <a href=\"http://royalmint.gov.uk/\">2 pence</a> coins; it ain&#8217;t chocolate.</p>\n</blockquote>\n\n<p>That&#8217;s all.</p>"),
    // HR tags
    ("<hr>\n\n<hr/>\n", "<hr>\n\n<hr/>"),
    // FTP Link aliases
    ("\"link with ftp alias\":uri-alias\n\n[uri-alias]ftp://foo@bar.net",
     "<p><a href=\"ftp://foo@bar.net/\">link with ftp alias</a></p>"),
    // Notes with IDs
    (concat!("Notes can[#test] have ids.\n\n",
             "note#test(#noteid). \"Proof\":https://example.com/page is here\n\n",
             "notelist:1.\n"),
     concat!("<p>Notes can<sup><a href=\"#noteUID-2\"><span id=\"noterefUID-1\">1</span></a></sup> have ids.</p>\n\n",
             "<ol>\n",
             "\t<li id=\"noteid\"><sup><a href=\"#noterefUID-1\">1</a></sup><span id=\"noteUID-2\"> </span><a href=\"https://example.com/page\">Proof</a> is here</li>\n",
             "</ol>")),
    // Using $-links coupled with link aliases
    ("\"$\":0\n[0]https://textpattern.com/start\n",
     "<p><a href=\"https://textpattern.com/start\">textpattern.com/start</a></p>"),
    // Proper HTML escaping for URLs
    ("!http://example.com/<name>.png!\n",
     "<p><img alt=\"\" src=\"http://example.com/%3Cname%3E.png\" /></p>"),
    (concat!("\"test\":link1\n",
    "[link1]http://example.com/<script>window.alert(\"HelloWorld!\");</script>\n"),
     "<p><a href=\"http://example.com/%3Cscript%3Ewindow.alert(%22HelloWorld!%22);%3C/script%3E\">test</a></p>"),

    // Proper HTML encoding within pre and code tags
    ("<pre>sdada <a href=\"link\">Link</a> @\"code\"@ dadsada</pre>",
     "<pre>sdada &lt;a href=&quot;link&quot;&gt;Link&lt;/a&gt; <code>\"code\"</code> dadsada</pre>"),
    // Definition list with a paragraph
    (concat!("|-(cold) milk :=\n",
             "Nourishing beverage for baby cows. =:\n",
             "|"),

     concat!("\t<table>\n",
             "\t\t<tr>\n",
             "\t\t\t<td><dl>\n",
             "\t<dt class=\"cold\">milk</dt>\n",
             "\t<dd><p>Nourishing beverage for baby cows.</p></dd>\n",
             "</dl></td>\n",
             "\t\t</tr>\n\t</table>")),
    // Nested lists of mixed types
    (concat!("* bullet\n",
             "*# number\n",
             "*# number\n",
             "*#* bullet\n",
             "*# number\n",
             "*# number with\n",
             "a break\n",
             "* bullet\n",
             "** okay\n"),
     concat!("\t<ul>\n",
             "\t\t<li>bullet\n",
             "\t\t<ol>\n",
             "\t\t\t<li>number</li>\n",
             "\t\t\t<li>number\n",
             "\t\t\t<ul>\n",
             "\t\t\t\t<li>bullet</li>\n",
             "\t\t\t</ul></li>\n",
             "\t\t\t<li>number</li>\n",
             "\t\t\t<li>number with<br>\n",
             "a break</li>\n",
             "\t\t</ol></li>\n",
             "\t\t<li>bullet\n",
             "\t\t<ul>\n",
             "\t\t\t<li>okay</li>\n",
             "\t\t</ul></li>\n",
             "\t\t</ul>")),

    ("\"a link\":http://example.com/?param1=1&param2=2",
     "<p><a href=\"http://example.com/?param1=1&amp;param2=2\">a link</a></p>"),
    (concat!("*W skład zestawu wchodzą*:\n\n",
             "*W skład zestawu wchodzą*: "),
     concat!("<p><strong>W skład zestawu wchodzą</strong>:</p>\n\n",
             "<p><strong>W skład zestawu wchodzą</strong>: </p>")),
    // Non-standalone ampersands should not be escaped
    (concat!("&#8220;<span lang=\"en\">test</span>&#8221;\n\n",
             "&#x201c;<span lang=\"en\">test</span>&#x201d;\n\n",
             "&nbsp;<span lang=\"en\">test</span>&nbsp;\n"),
     concat!("<p>&#8220;<span lang=\"en\">test</span>&#8221;</p>\n\n",
             "<p>&#x201c;<span lang=\"en\">test</span>&#x201d;</p>\n\n",
             "<p>&nbsp;<span lang=\"en\">test</span>&nbsp;</p>")),
    // A standalone comment block should be left untouched
    ("An ordinary block\n\n<!-- Comment block -->",
     "<p>An ordinary block</p>\n\n<!-- Comment block -->"),
    ("I am crazy about \"Hobix\":hobix\nand \"it\'s\":hobix \"all\":hobix I ever\n\"link to\":hobix!\n\n[hobix]http://hobix.com",
     concat!("<p>I am crazy about <a href=\"http://hobix.com/\">Hobix</a><br>\nand <a href=\"http://hobix.com/\">it&#8217;s</a> ",
             "<a href=\"http://hobix.com/\">all</a> I ever<br>\n<a href=\"http://hobix.com/\">link to</a>!</p>")),
    ("a -b\na- b\n", "<p>a -b<br>\na- b</p>"),
    ("bc.. Paragraph 1\n\nParagraph 2\n\nParagraph 3\n", "<pre><code>Paragraph 1\n\nParagraph 2\n\nParagraph 3</code></pre>"),
    ("I spoke.\nAnd none replied.", "<p>I spoke.<br>\nAnd none replied.</p>"),
    ("I __know__.\nI **really** __know__.", "<p>I <i>know</i>.<br>\nI <b>really</b> <i>know</i>.</p>"),
    ("I\'m %{color:red}unaware%\nof most soft drinks.", "<p>I&#8217;m <span style=\"color:red;\">unaware</span><br>\nof most soft drinks.</p>"),
    ("I seriously *{color:red}blushed*\nwhen I _(big)sprouted_ that\ncorn stalk from my\n%[es]cabeza%.",
     concat!("<p>I seriously <strong style=\"color:red;\">blushed</strong><br>\nwhen I <em class=\"big\">sprouted</em>",
             " that<br>\ncorn stalk from my<br>\n<span lang=\"es\">cabeza</span>.</p>")),
    ("<pre>\n<code>\na.gsub!( /</, \"\" )\n</code>\n</pre>",
     "<pre>\n<code>\na.gsub!( /&lt;/, \"\" )\n</code>\n</pre>"),
    (concat!("<div style=\"float:right;\">\n\nh3. Sidebar\n\n\"Hobix\":http://hobix.com/\n\"Ruby\":http://ruby-lang.org/\n\n</div>\n\n",
             "The main text of the\npage goes here and will\nstay to the left of the\nsidebar."),
     concat!("<div style=\"float:right;\">\n\n\t<h3>Sidebar</h3>\n\n",
             "<p><a href=\"http://hobix.com/\">Hobix</a><br>\n",
             "<a href=\"http://ruby-lang.org/\">Ruby</a></p>\n\n</div>\n\n",
             "<p>The main text of the<br>\n",
             "page goes here and will<br>\nstay to the left of the<br>\nsidebar.</p>")),


    ("!http://hobix.com/sample.jpg!", "<p><img alt=\"\" src=\"http://hobix.com/sample.jpg\" /></p>"),
    ("!openwindow1.gif(Bunny.)!", "<p><img alt=\"Bunny.\" src=\"openwindow1.gif\" title=\"Bunny.\" /></p>"),
    ("!openwindow1.gif!:http://hobix.com/", "<p><a href=\"http://hobix.com/\"><img alt=\"\" src=\"openwindow1.gif\" /></a></p>"),
    ("!>obake.gif!\n\nAnd others sat all round the small\nmachine and paid it to sing to them.",
     concat!("<p><img alt=\"\" class=\"align-right\" src=\"obake.gif\" /></p>\n\n\t",
             "<p>And others sat all round the small<br>\nmachine and paid it to sing to them.</p>")),
    ("!http://render.mathim.com/A%5EtAx%20%3D%20A%5Et%28Ax%29.!",
     "<p><img alt=\"\" src=\"http://render.mathim.com/A%5EtAx%20%3D%20A%5Et%28Ax%29.\" /></p>"),
    ("notextile. <b> foo bar baz</b>\n\np. quux\n",
     "<b> foo bar baz</b>\n\n<p>quux</p>"),
    ("\"foo\":http://google.com/one--two", "<p><a href=\"http://google.com/one--two\">foo</a></p>"),
    // issue 24 colspan
    ("|\\2. spans two cols |\n| col 1 | col 2 |", "\t<table>\n\t\t<tr>\n\t\t\t<td colspan=\"2\">spans two cols </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> col 1 </td>\n\t\t\t<td> col 2 </td>\n\t\t</tr>\n\t</table>"),
    // issue 2 escaping
    ("\"foo ==(bar)==\":#foobar", "<p><a href=\"#foobar\">foo (bar)</a></p>"),
    // issue 14 newlines in extended pre blocks
    ("pre.. Hello\n\nAgain\n\np. normal text", "<pre>Hello\n\nAgain</pre>\n\n<p>normal text</p>"),
    // url with parentheses
    ("\"python\":http://en.wikipedia.org/wiki/Python_(programming_language)", "<p><a href=\"http://en.wikipedia.org/wiki/Python_(programming_language)\">python</a></p>"),
    // table with hyphen styles
    ("table(linkblog-thumbnail).\n|(linkblog-thumbnail-cell). apple|bear|", "\t<table class=\"linkblog-thumbnail\">\n\t\t<tr>\n\t\t\t<td class=\"linkblog-thumbnail-cell\">apple</td>\n\t\t\t<td>bear</td>\n\t\t</tr>\n\t</table>"),
    // issue 32 empty table cells
    ("|thing|||otherthing|", "\t<table>\n\t\t<tr>\n\t\t\t<td>thing</td>\n\t\t\t<td></td>\n\t\t\t<td></td>\n\t\t\t<td>otherthing</td>\n\t\t</tr>\n\t</table>"),
    // issue 36 link reference names http and https
    ("\"signup\":signup\n[signup]http://myservice.com/signup", "<p><a href=\"http://myservice.com/signup\">signup</a></p>"),
    ("\"signup\":signup\n[signup]https://myservice.com/signup", "<p><a href=\"https://myservice.com/signup\">signup</a></p>"),
    // nested formatting
    ("*_test text_*", "<p><strong><em>test text</em></strong></p>"),
    ("_*test text*_", "<p><em><strong>test text</strong></em></p>"),
    // quotes in code block
    ("<code>\"quoted string\"</code>", "<p><code>\"quoted string\"</code></p>"),
    ("<pre>some preformatted text</pre>other text", "<pre>some preformatted text</pre>other text"),
    // at sign and notextile in table
    ("|@<A1>@|@<A2>@ @<A3>@|\n|<notextile>*B1*</notextile>|<notextile>*B2*</notextile> <notextile>*B3*</notextile>|", "\t<table>\n\t\t<tr>\n\t\t\t<td><code>&lt;A1&gt;</code></td>\n\t\t\t<td><code>&lt;A2&gt;</code> <code>&lt;A3&gt;</code></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td>*B1*</td>\n\t\t\t<td>*B2* *B3*</td>\n\t\t</tr>\n\t</table>"),
    // cite attribute
    ("bq.:http://textism.com/ Text...", "<blockquote cite=\"http://textism.com/\">\n<p>Text&#8230;</p>\n</blockquote>"),
    ("Hello [\"(Mum) & dad\"]", "<p>Hello [&#8220;(Mum) &amp; dad&#8221;]</p>"),
    ("a -b-", "<p>a <del>b</del></p>"),
];

fn run_fixtures(textile: &rustextile::Textile, fixtures: &[(&str, &str)]) {
    for (text, expected) in fixtures {
        let processed = normalize_newlines(&textile.parse(text));
        let expected = normalize_newlines(expected);
        assert_str_eq!(processed, expected, "Input Textile: {:#?}\nFull output: {}", text, processed);
    }
}

#[test]
fn test_known_restricted_values_html() {
    let textile = rustextile::Textile::default()
        .set_html_kind(rustextile::HtmlKind::HTML5)
        .set_restricted(true);
    run_fixtures(&textile, &HTML_RESTRICTED_MODE_VALUES);
}

#[test]
fn test_known_values_html() {
    let textile = rustextile::Textile::default()
        .set_html_kind(rustextile::HtmlKind::HTML5)
        .set_uid("UID");
    run_fixtures(&textile, &HTML_KNOWN_VALUES);
}
