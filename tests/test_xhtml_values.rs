//! These tests were ported from the original python-textile library.
//!
use pretty_assertions::assert_str_eq;

const XHTML_KNOWN_VALUES: [(&str, &str); 104] = [
    ("hello, world", "<p>hello, world</p>"),

    ("A single paragraph.\n\nFollowed by another.",
     "<p>A single paragraph.</p>\n\n<p>Followed by another.</p>"),

    ("I am <b>very</b> serious.\n\n<pre>\nI am <b>very</b> serious.\n</pre>",
     "<p>I am <b>very</b> serious.</p>\n\n<pre>\nI am &lt;b&gt;very&lt;/b&gt; serious.\n</pre>"),

    ("I spoke.\nAnd none replied.", "<p>I spoke.<br />\nAnd none replied.</p>"),

    (r#""Observe!""#, "<p>&#8220;Observe!&#8221;</p>"),

    ("Observe -- very nice!", "<p>Observe &#8212; very nice!</p>"),

    ("Observe - tiny and brief.", "<p>Observe &#8211; tiny and brief.</p>"),

    ("Observe...", "<p>Observe&#8230;</p>"),

    ("Observe ...", "<p>Observe &#8230;</p>"),

    ("Observe: 2 x 2.", "<p>Observe: 2 &#215; 2.</p>"),

    ("one(TM), two(R), three(C).", "<p>one&#8482;, two&#174;, three&#169;.</p>"),

    ("h1. Header 1", "<h1>Header 1</h1>"),

    ("h2. Header 2", "<h2>Header 2</h2>"),

    ("h3. Header 3", "<h3>Header 3</h3>"),

    ("An old text\n\nbq. A block quotation.\n\nAny old text",
     "<p>An old text</p>\n\n<blockquote>\n\t<p>A block quotation.</p>\n</blockquote>\n\n<p>Any old text</p>"),

    ("I _believe_ every word.", "<p>I <em>believe</em> every word.</p>"),

    ("And then? She *fell*!", "<p>And then? She <strong>fell</strong>!</p>"),

    ("I __know__.\nI **really** __know__.", "<p>I <i>know</i>.<br />\nI <b>really</b> <i>know</i>.</p>"),

    ("??Cat's Cradle?? by Vonnegut", "<p><cite>Cat&#8217;s Cradle</cite> by Vonnegut</p>"),

    ("Convert with @str(foo)@", "<p>Convert with <code>str(foo)</code></p>"),

    ("I\'m -sure- not sure.", "<p>I&#8217;m <del>sure</del> not sure.</p>"),

    ("You are a +pleasant+ child.", "<p>You are a <ins>pleasant</ins> child.</p>"),

    ("a ^2^ + b ^2^ = c ^2^", "<p>a <sup>2</sup> + b <sup>2</sup> = c <sup>2</sup></p>"),

    ("log ~2~ x", "<p>log <sub>2</sub> x</p>"),

    ("I'm %unaware% of most soft drinks.", "<p>I&#8217;m <span>unaware</span> of most soft drinks.</p>"),

    ("I'm %{color:red}unaware%\nof most soft drinks.", "<p>I&#8217;m <span style=\"color:red;\">unaware</span><br />\nof most soft drinks.</p>"),
    ("p(example1). An example", "<p class=\"example1\">An example</p>"),
    ("p(#big-red). Red here", "<p id=\"big-red\">Red here</p>"),
    ("p(example1#big-red2). Red here", "<p class=\"example1\" id=\"big-red2\">Red here</p>"),
    ("p{color:blue;margin:30px}. Spacey blue", "<p style=\"color:blue; margin:30px;\">Spacey blue</p>"),
    ("p[fr]. rouge", "<p lang=\"fr\">rouge</p>"),
    ("I seriously *{color:red}blushed*\nwhen I _(big)sprouted_ that\ncorn stalk from my\n%[es]cabeza%.", "<p>I seriously <strong style=\"color:red;\">blushed</strong><br />\nwhen I <em class=\"big\">sprouted</em> that<br />\ncorn stalk from my<br />\n<span lang=\"es\">cabeza</span>.</p>"),
    ("p<. align left", "<p style=\"text-align:left;\">align left</p>"),
    ("p>. align right", "<p style=\"text-align:right;\">align right</p>"),
    ("p=. centered", "<p style=\"text-align:center;\">centered</p>"),
    ("p<>. justified", "<p style=\"text-align:justify;\">justified</p>"),
    ("p(. left ident 1em", "<p style=\"padding-left:1em;\">left ident 1em</p>"),
    ("p((. left ident 2em", "<p style=\"padding-left:2em;\">left ident 2em</p>"),
    ("p))). right ident 3em", "<p style=\"padding-right:3em;\">right ident 3em</p>"),
    ("h2()>. Bingo.", "<h2 style=\"padding-left:1em; padding-right:1em; text-align:right;\">Bingo.</h2>"),
    ("h3()>[no]{color:red}. Bingo", "<h3 lang=\"no\" style=\"color:red; padding-left:1em; padding-right:1em; text-align:right;\">Bingo</h3>"),
    ("<pre>\n<code>\na.gsub!( /</, \"\" )\n</code>\n</pre>", "<pre>\n<code>\na.gsub!( /&lt;/, \"\" )\n</code>\n</pre>"),
    ("<div style=\"float:right;\">\n\nh3. Sidebar\n\n\"Hobix\":http://hobix.com/\n\"Ruby\":http://ruby-lang.org/\n\n</div>\n\nThe main text of the\npage goes here and will\nstay to the left of the\nsidebar.",
     "<div style=\"float:right;\">\n\n<h3>Sidebar</h3>\n\n<p><a href=\"http://hobix.com/\">Hobix</a><br />\n<a href=\"http://ruby-lang.org/\">Ruby</a></p>\n\n</div>\n\n<p>The main text of the<br />\npage goes here and will<br />\nstay to the left of the<br />\nsidebar.</p>"),
    ("# A first item\n# A second item\n# A third", "<ol>\n\t<li>A first item</li>\n\t<li>A second item</li>\n\t<li>A third</li>\n</ol>"),
    ("# Fuel could be:\n## Coal\n## Gasoline\n## Electricity\n# Humans need only:\n## Water\n## Protein", "<ol>\n\t<li>Fuel could be:\n\t<ol>\n\t\t<li>Coal</li>\n\t\t<li>Gasoline</li>\n\t\t<li>Electricity</li>\n\t</ol></li>\n\t<li>Humans need only:\n\t<ol>\n\t\t<li>Water</li>\n\t\t<li>Protein</li>\n\t</ol></li>\n\t</ol>"),
    ("* A first item\n* A second item\n* A third", "<ul>\n\t<li>A first item</li>\n\t<li>A second item</li>\n\t<li>A third</li>\n</ul>"),
    ("* Fuel could be:\n** Coal\n** Gasoline\n** Electricity\n* Humans need only:\n** Water\n** Protein", "<ul>\n\t<li>Fuel could be:\n\t<ul>\n\t\t<li>Coal</li>\n\t\t<li>Gasoline</li>\n\t\t<li>Electricity</li>\n\t</ul></li>\n\t<li>Humans need only:\n\t<ul>\n\t\t<li>Water</li>\n\t\t<li>Protein</li>\n\t</ul></li>\n\t</ul>"),
    ("I searched \"Google\":http://google.com.", "<p>I searched <a href=\"http://google.com/\">Google</a>.</p>"),
    ("I searched \"a search engine (Google)\":http://google.com.", "<p>I searched <a href=\"http://google.com/\" title=\"Google\">a search engine</a>.</p>"),
    ("I am crazy about \"Hobix\":hobix\nand \"it's\":hobix \"all\":hobix I ever\n\"link to\":hobix!\n\n[hobix]http://hobix.com",
     "<p>I am crazy about <a href=\"http://hobix.com/\">Hobix</a><br />\nand <a href=\"http://hobix.com/\">it&#8217;s</a> <a href=\"http://hobix.com/\">all</a> I ever<br />\n<a href=\"http://hobix.com/\">link to</a>!</p>"),
    ("!http://hobix.com/sample.jpg!", "<p><img alt=\"\" src=\"http://hobix.com/sample.jpg\" /></p>"),
    ("!openwindow1.gif(Bunny.)!", "<p><img alt=\"Bunny.\" src=\"openwindow1.gif\" title=\"Bunny.\" /></p>"),
    ("!openwindow1.gif!:http://hobix.com/", "<p><a href=\"http://hobix.com/\"><img alt=\"\" src=\"openwindow1.gif\" /></a></p>"),
    ("!>obake.gif!\n\nAnd others sat all round the small\nmachine and paid it to sing to them.", "<p><img align=\"right\" alt=\"\" src=\"obake.gif\" /></p>\n\n<p>And others sat all round the small<br />\nmachine and paid it to sing to them.</p>"),
    ("We use CSS(Cascading Style Sheets).", "<p>We use <acronym title=\"Cascading Style Sheets\"><span class=\"caps\">CSS</span></acronym>.</p>"),
    ("|one|two|three|\n|a|b|c|", "\t<table>\n\t\t<tr>\n\t\t\t<td>one</td>\n\t\t\t<td>two</td>\n\t\t\t<td>three</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td>a</td>\n\t\t\t<td>b</td>\n\t\t\t<td>c</td>\n\t\t</tr>\n\t</table>"),
    ("| name | age | sex |\n| joan | 24 | f |\n| archie | 29 | m |\n| bella | 45 | f |", "\t<table>\n\t\t<tr>\n\t\t\t<td> name </td>\n\t\t\t<td> age </td>\n\t\t\t<td> sex </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> joan </td>\n\t\t\t<td> 24 </td>\n\t\t\t<td> f </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> archie </td>\n\t\t\t<td> 29 </td>\n\t\t\t<td> m </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> bella </td>\n\t\t\t<td> 45 </td>\n\t\t\t<td> f </td>\n\t\t</tr>\n\t</table>"),
    ("|_. name |_. age |_. sex |\n| joan | 24 | f |\n| archie | 29 | m |\n| bella | 45 | f |", "\t<table>\n\t\t<tr>\n\t\t\t<th>name </th>\n\t\t\t<th>age </th>\n\t\t\t<th>sex </th>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> joan </td>\n\t\t\t<td> 24 </td>\n\t\t\t<td> f </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> archie </td>\n\t\t\t<td> 29 </td>\n\t\t\t<td> m </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> bella </td>\n\t\t\t<td> 45 </td>\n\t\t\t<td> f </td>\n\t\t</tr>\n\t</table>"),
    ("<script>alert(\"hello\");</script>", "<p><script>alert(&#8220;hello&#8221;);</script></p>"),
    ("pre.. Hello\n\nHello Again\n\np. normal text", "<pre>Hello\n\nHello Again</pre>\n\n<p>normal text</p>"),
    ("<pre>this is in a pre tag</pre>", "<pre>this is in a pre tag</pre>"),
    ("\"test1\":http://foo.com/bar--baz\n\n\"test2\":http://foo.com/bar---baz\n\n\"test3\":http://foo.com/bar-17-18-baz", "<p><a href=\"http://foo.com/bar--baz\">test1</a></p>\n\n<p><a href=\"http://foo.com/bar---baz\">test2</a></p>\n\n<p><a href=\"http://foo.com/bar-17-18-baz\">test3</a></p>"),
    ("\"foo ==(bar)==\":#foobar", "<p><a href=\"#foobar\">foo (bar)</a></p>"),
    ("!http://render.mathim.com/A%5EtAx%20%3D%20A%5Et%28Ax%29.!", "<p><img alt=\"\" src=\"http://render.mathim.com/A%5EtAx%20%3D%20A%5Et%28Ax%29.\" /></p>"),
    ("* Point one\n* Point two\n## Step 1\n## Step 2\n## Step 3\n* Point three\n** Sub point 1\n** Sub point 2", "<ul>\n\t<li>Point one</li>\n\t<li>Point two\n\t<ol>\n\t\t<li>Step 1</li>\n\t\t<li>Step 2</li>\n\t\t<li>Step 3</li>\n\t</ol></li>\n\t<li>Point three\n\t<ul>\n\t\t<li>Sub point 1</li>\n\t\t<li>Sub point 2</li>\n\t</ul></li>\n\t</ul>"),
    ("@array[4] = 8@", "<p><code>array[4] = 8</code></p>"),
    ("#{color:blue} one\n# two\n# three", "<ol style=\"color:blue;\">\n\t<li>one</li>\n\t<li>two</li>\n\t<li>three</li>\n</ol>"),
    ("Links (like \"this\":http://foo.com), are now mangled in 2.1.0, whereas 2.0 parsed them correctly.", "<p>Links (like <a href=\"http://foo.com/\">this</a>), are now mangled in 2.1.0, whereas 2.0 parsed them correctly.</p>"),
    ("@monospaced text@, followed by text", "<p><code>monospaced text</code>, followed by text</p>"),
    ("h2. A header\n\n\n\n\n\nsome text", "<h2>A header</h2>\n\n\n\n\n\n<p>some text</p>"),
    ("pre.. foo bar baz\nquux", "<pre>foo bar baz\nquux</pre>"),
    ("line of text\n\n    leading spaces", "<p>line of text</p>\n\n    leading spaces"),
    ("\"some text\":http://www.example.com/?q=foo%20bar and more text", "<p><a href=\"http://www.example.com/?q=foo%20bar\">some text</a> and more text</p>"),
    ("(??some text??)", "<p>(<cite>some text</cite>)</p>"),
    ("(*bold text*)", "<p>(<strong>bold text</strong>)</p>"),
    ("H[~2~]O", "<p>H<sub>2</sub>O</p>"),
    ("p=. Où est l'école, l'église s'il vous plaît?", "<p style=\"text-align:center;\">Où est l&#8217;école, l&#8217;église s&#8217;il vous plaît?</p>"),
    ("p=. *_The_* _*Prisoner*_", "<p style=\"text-align:center;\"><strong><em>The</em></strong> <em><strong>Prisoner</strong></em></p>"),
    ("p=. \"An emphasised _word._\" & \"*A spanned phrase.*\" ", "<p style=\"text-align:center;\">&#8220;An emphasised <em>word.</em>&#8221; &amp; &#8220;<strong>A spanned phrase.</strong>&#8221; </p>"),
    ("p=. \"*Here*'s a word!\" ", "<p style=\"text-align:center;\">&#8220;<strong>Here</strong>&#8217;s a word!&#8221; </p>"),
    ("p=. \"Please visit our \"Textile Test Page\":http://textile.sitemonks.com\" ", "<p style=\"text-align:center;\">&#8220;Please visit our <a href=\"http://textile.sitemonks.com/\">Textile Test Page</a>&#8221; </p>"),
    ("| Foreign EXPÓŅÉNTIAL |", "\t<table>\n\t\t<tr>\n\t\t\t<td> Foreign <span class=\"caps\">EXPÓŅÉNTIAL</span> </td>\n\t\t</tr>\n\t</table>"),
    ("Piękne ŹDŹBŁO", "<p>Piękne <span class=\"caps\">ŹDŹBŁO</span></p>"),
    ("p=. Tell me, what is AJAX(Asynchronous Javascript and XML), please?", "<p style=\"text-align:center;\">Tell me, what is <acronym title=\"Asynchronous Javascript and XML\"><span class=\"caps\">AJAX</span></acronym>, please?</p>"),
    ("p{font-size:0.8em}. *TxStyle* is a documentation project of Textile 2.4 for \"Textpattern CMS\":http://texpattern.com.", "<p style=\"font-size:0.8em;\"><strong>TxStyle</strong> is a documentation project of Textile 2.4 for <a href=\"http://texpattern.com/\">Textpattern <span class=\"caps\">CMS</span></a>.</p>"),
    ("\"Übermensch\":http://de.wikipedia.org/wiki/Übermensch", "<p><a href=\"http://de.wikipedia.org/wiki/%C3%9Cbermensch\">Übermensch</a></p>"),
    ("Here is some text with a <!-- Commented out[1] --> block.\n\n<!-- Here is a single <span>line</span> comment block -->\n\n<!-- Here is a whole\nmultiline\n<span>HTML</span>\nComment\n-->\n\nbc. <!-- Here is a comment block in a code block. -->",
     "<p>Here is some text with a <!-- Commented out[1] --> block.</p>\n\n<!-- Here is a single <span>line</span> comment block -->\n\n<!-- Here is a whole\nmultiline\n<span>HTML</span>\nComment\n-->\n\n<pre><code>&lt;!-- Here is a comment block in a code block. --&gt;</code></pre>"),
    ("\"Textile(c)\" is a registered(r) 'trademark' of Textpattern(tm) -- or TXP(That's textpattern!) -- at least it was - back in '88 when 2x4 was (+/-)5(o)C ... QED!\n\np{font-size: 200%;}. 2(1/4) 3(1/2) 4(3/4)",
     "<p>&#8220;Textile&#169;&#8221; is a registered&#174; &#8216;trademark&#8217; of Textpattern&#8482; &#8212; or <acronym title=\"That&#8217;s textpattern!\"><span class=\"caps\">TXP</span></acronym> &#8212; at least it was &#8211; back in &#8217;88 when 2&#215;4 was &#177;5&#176;C &#8230; <span class=\"caps\">QED</span>!</p>\n\n<p style=\"font-size: 200%;\">2&#188; 3&#189; 4&#190;</p>"),
    ("|=. Testing colgroup and col syntax\n|:\\5. 80\n|a|b|c|d|e|\n\n|=. Testing colgroup and col syntax|\n|:\\5. 80|\n|a|b|c|d|e|",
     "\t<table>\n\t<caption>Testing colgroup and col syntax</caption>\n\t<colgroup span=\"5\" width=\"80\">\n\t</colgroup>\n\t\t<tr>\n\t\t\t<td>a</td>\n\t\t\t<td>b</td>\n\t\t\t<td>c</td>\n\t\t\t<td>d</td>\n\t\t\t<td>e</td>\n\t\t</tr>\n\t</table>\n\n\t<table>\n\t<caption>Testing colgroup and col syntax</caption>\n\t<colgroup span=\"5\" width=\"80\">\n\t</colgroup>\n\t\t<tr>\n\t\t\t<td>a</td>\n\t\t\t<td>b</td>\n\t\t\t<td>c</td>\n\t\t\t<td>d</td>\n\t\t\t<td>e</td>\n\t\t</tr>\n\t</table>"),
    ("table(#dvds){border-collapse:collapse}. Great films on DVD employing Textile summary, caption, thead, tfoot, two tbody elements and colgroups\n|={font-size:140%;margin-bottom:15px}. DVDs with two Textiled tbody elements\n|:\\3. 100 |{background:#ddd}|250||50|300|\n|^(header).\n|_. Title |_. Starring |_. Director |_. Writer |_. Notes |\n|~(footer).\n|\\5=. This is the tfoot, centred |\n|-(toplist){background:#c5f7f6}.\n| _The Usual Suspects_ | Benicio Del Toro, Gabriel Byrne, Stephen Baldwin, Kevin Spacey | Bryan Singer | Chris McQaurrie | One of the finest films ever made |\n| _Se7en_ | Morgan Freeman, Brad Pitt, Kevin Spacey | David Fincher | Andrew Kevin Walker | Great psychological thriller |\n| _Primer_ | David Sullivan, Shane Carruth | Shane Carruth | Shane Carruth | Amazing insight into trust and human psychology <br />rather than science fiction. Terrific! |\n| _District 9_ | Sharlto Copley, Jason Cope | Neill Blomkamp | Neill Blomkamp, Terri Tatchell | Social commentary layered on thick,\nbut boy is it done well |\n|-(medlist){background:#e7e895;}.\n| _Arlington Road_ | Tim Robbins, Jeff Bridges | Mark Pellington | Ehren Kruger | Awesome study in neighbourly relations |\n| _Phone Booth_ | Colin Farrell, Kiefer Sutherland, Forest Whitaker | Joel Schumacher | Larry Cohen | Edge-of-the-seat stuff in this\nshort but brilliantly executed thriller |",
     "\t<table id=\"dvds\" style=\"border-collapse:collapse;\" summary=\"Great films on DVD employing Textile summary, caption, thead, tfoot, two tbody elements and colgroups\">\n\t<caption style=\"font-size:140%; margin-bottom:15px;\"><span class=\"caps\">DVD</span>s with two Textiled tbody elements</caption>\n\t<colgroup span=\"3\" width=\"100\">\n\t<col style=\"background:#ddd;\" />\n\t<col width=\"250\" />\n\t<col />\n\t<col width=\"50\" />\n\t<col width=\"300\" />\n\t</colgroup>\n\t<thead class=\"header\">\n\t\t<tr>\n\t\t\t<th>Title </th>\n\t\t\t<th>Starring </th>\n\t\t\t<th>Director </th>\n\t\t\t<th>Writer </th>\n\t\t\t<th>Notes </th>\n\t\t</tr>\n\t</thead>\n\t<tfoot class=\"footer\">\n\t\t<tr>\n\t\t\t<td colspan=\"5\" style=\"text-align:center;\">This is the tfoot, centred </td>\n\t\t</tr>\n\t</tfoot>\n\t<tbody class=\"toplist\" style=\"background:#c5f7f6;\">\n\t\t<tr>\n\t\t\t<td> <em>The Usual Suspects</em> </td>\n\t\t\t<td> Benicio Del Toro, Gabriel Byrne, Stephen Baldwin, Kevin Spacey </td>\n\t\t\t<td> Bryan Singer </td>\n\t\t\t<td> Chris McQaurrie </td>\n\t\t\t<td> One of the finest films ever made </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> <em>Se7en</em> </td>\n\t\t\t<td> Morgan Freeman, Brad Pitt, Kevin Spacey </td>\n\t\t\t<td> David Fincher </td>\n\t\t\t<td> Andrew Kevin Walker </td>\n\t\t\t<td> Great psychological thriller </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> <em>Primer</em> </td>\n\t\t\t<td> David Sullivan, Shane Carruth </td>\n\t\t\t<td> Shane Carruth </td>\n\t\t\t<td> Shane Carruth </td>\n\t\t\t<td> Amazing insight into trust and human psychology <br />\nrather than science fiction. Terrific! </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> <em>District 9</em> </td>\n\t\t\t<td> Sharlto Copley, Jason Cope </td>\n\t\t\t<td> Neill Blomkamp </td>\n\t\t\t<td> Neill Blomkamp, Terri Tatchell </td>\n\t\t\t<td> Social commentary layered on thick,<br />\nbut boy is it done well </td>\n\t\t</tr>\n\t</tbody>\n\t<tbody class=\"medlist\" style=\"background:#e7e895;\">\n\t\t<tr>\n\t\t\t<td> <em>Arlington Road</em> </td>\n\t\t\t<td> Tim Robbins, Jeff Bridges </td>\n\t\t\t<td> Mark Pellington </td>\n\t\t\t<td> Ehren Kruger </td>\n\t\t\t<td> Awesome study in neighbourly relations </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> <em>Phone Booth</em> </td>\n\t\t\t<td> Colin Farrell, Kiefer Sutherland, Forest Whitaker </td>\n\t\t\t<td> Joel Schumacher </td>\n\t\t\t<td> Larry Cohen </td>\n\t\t\t<td> Edge-of-the-seat stuff in this<br />\nshort but brilliantly executed thriller </td>\n\t\t</tr>\n\t</tbody>\n\t</table>"),
    ("-(hot) *coffee* := Hot _and_ black\n-(hot#tea) tea := Also hot, but a little less black\n-(cold) milk := Nourishing beverage for baby cows.\nCold drink that goes great with cookies. =:\n\n-(hot) coffee := Hot and black\n-(hot#tea) tea := Also hot, but a little less black\n-(cold) milk :=\nNourishing beverage for baby cows.\nCold drink that goes great with cookies. =:",
     "<dl>\n\t<dt class=\"hot\"><strong>coffee</strong></dt>\n\t<dd>Hot <em>and</em> black</dd>\n\t<dt class=\"hot\" id=\"tea\">tea</dt>\n\t<dd>Also hot, but a little less black</dd>\n\t<dt class=\"cold\">milk</dt>\n\t<dd>Nourishing beverage for baby cows.<br />\nCold drink that goes great with cookies.</dd>\n</dl>\n\n<dl>\n\t<dt class=\"hot\">coffee</dt>\n\t<dd>Hot and black</dd>\n\t<dt class=\"hot\" id=\"tea\">tea</dt>\n\t<dd>Also hot, but a little less black</dd>\n\t<dt class=\"cold\">milk</dt>\n\t<dd><p>Nourishing beverage for baby cows.<br />\nCold drink that goes great with cookies.</p></dd>\n</dl>"),
    (";(class#id) Term 1\n: Def 1\n: Def 2\n: Def 3",
     "<dl class=\"class\" id=\"id\">\n\t<dt>Term 1</dt>\n\t<dd>Def 1</dd>\n\t<dd>Def 2</dd>\n\t<dd>Def 3</dd>\n</dl>"),
    ("*Here is a comment*\n\nHere is *(class)a comment*\n\n*(class)Here is a class* that is a little extended and is\n*followed* by a strong word!\n\nbc. ; Content-type: text/javascript\n; Cache-Control: no-store, no-cache, must-revalidate, pre-check=0, post-check=0, max-age=0\n; Expires: Sat, 24 Jul 2003 05:00:00 GMT\n; Last-Modified: Wed, 1 Jan 2025 05:00:00 GMT\n; Pragma: no-cache\n\n*123 test*\n\n*test 123*\n\n**123 test**\n\n**test 123**",
     "<p><strong>Here is a comment</strong></p>\n\n<p>Here is <strong class=\"class\">a comment</strong></p>\n\n<p><strong class=\"class\">Here is a class</strong> that is a little extended and is<br />\n<strong>followed</strong> by a strong word!</p>\n\n<pre><code>; Content-type: text/javascript\n; Cache-Control: no-store, no-cache, must-revalidate, pre-check=0, post-check=0, max-age=0\n; Expires: Sat, 24 Jul 2003 05:00:00 GMT\n; Last-Modified: Wed, 1 Jan 2025 05:00:00 GMT\n; Pragma: no-cache</code></pre>\n\n<p><strong>123 test</strong></p>\n\n<p><strong>test 123</strong></p>\n\n<p><b>123 test</b></p>\n\n<p><b>test 123</b></p>"),
    ("#_(first#list) one\n# two\n# three\n\ntest\n\n#(ordered#list2).\n# one\n# two\n# three\n\ntest\n\n#_(class_4).\n# four\n# five\n# six\n\ntest\n\n#_ seven\n# eight\n# nine\n\ntest\n\n# one\n# two\n# three\n\ntest\n\n#22 22\n# 23\n# 24",
     "<ol class=\"first\" id=\"list\" start=\"1\">\n\t<li>one</li>\n\t<li>two</li>\n\t<li>three</li>\n</ol>\n\n<p>test</p>\n\n<ol class=\"ordered\" id=\"list2\">\n\t<li>one</li>\n\t<li>two</li>\n\t<li>three</li>\n</ol>\n\n<p>test</p>\n\n<ol class=\"class_4\" start=\"4\">\n\t<li>four</li>\n\t<li>five</li>\n\t<li>six</li>\n</ol>\n\n<p>test</p>\n\n<ol start=\"7\">\n\t<li>seven</li>\n\t<li>eight</li>\n\t<li>nine</li>\n</ol>\n\n<p>test</p>\n\n<ol>\n\t<li>one</li>\n\t<li>two</li>\n\t<li>three</li>\n</ol>\n\n<p>test</p>\n\n<ol start=\"22\">\n\t<li>22</li>\n\t<li>23</li>\n\t<li>24</li>\n</ol>"),
    ("# one\n##3 one.three\n## one.four\n## one.five\n# two\n\ntest\n\n#_(continuation#section2).\n# three\n# four\n##_ four.six\n## four.seven\n# five\n\ntest\n\n#21 twenty-one\n# twenty-two",
     "<ol>\n\t<li>one\n\t<ol start=\"3\">\n\t\t<li>one.three</li>\n\t\t<li>one.four</li>\n\t\t<li>one.five</li>\n\t</ol></li>\n\t<li>two</li>\n</ol>\n\n<p>test</p>\n\n<ol class=\"continuation\" id=\"section2\" start=\"3\">\n\t<li>three</li>\n\t<li>four\n\t<ol start=\"6\">\n\t\t<li>four.six</li>\n\t\t<li>four.seven</li>\n\t</ol></li>\n\t<li>five</li>\n</ol>\n\n<p>test</p>\n\n<ol start=\"21\">\n\t<li>twenty-one</li>\n\t<li>twenty-two</li>\n</ol>"),
    ("|* Foo[^2^]\n* _bar_\n* ~baz~ |\n|#4 *Four*\n# __Five__ |\n|-(hot) coffee := Hot and black\n-(hot#tea) tea := Also hot, but a little less black\n-(cold) milk :=\nNourishing beverage for baby cows.\nCold drink that goes great with cookies. =:\n|",
     "\t<table>\n\t\t<tr>\n\t\t\t<td><ul>\n\t<li>Foo<sup>2</sup></li>\n\t<li><em>bar</em></li>\n\t<li><sub>baz</sub></li>\n</ul></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td><ol start=\"4\">\n\t<li><strong>Four</strong></li>\n\t<li><i>Five</i></li>\n</ol></td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td><dl>\n\t<dt class=\"hot\">coffee</dt>\n\t<dd>Hot and black</dd>\n\t<dt class=\"hot\" id=\"tea\">tea</dt>\n\t<dd>Also hot, but a little less black</dd>\n\t<dt class=\"cold\">milk</dt>\n\t<dd><p>Nourishing beverage for baby cows.<br />\nCold drink that goes great with cookies.</p></dd>\n</dl></td>\n\t\t</tr>\n\t</table>"),
    ("h4. A more complicated table\n\ntable(tableclass#tableid){color:blue}.\n|_. table |_. more |_. badass |\n|\\3. Horizontal span of 3|\n(firstrow). |first|HAL(open the pod bay doors)|1|\n|some|{color:green}. styled|content|\n|/2. spans 2 rows|this is|quite a|\n| deep test | don't you think?|\n(lastrow). |fifth|I'm a lumberjack|5|\n|sixth| _*bold italics*_ |6|",
     "<h4>A more complicated table</h4>\n\n\t<table class=\"tableclass\" id=\"tableid\" style=\"color:blue;\">\n\t\t<tr>\n\t\t\t<th>table </th>\n\t\t\t<th>more </th>\n\t\t\t<th>badass </th>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td colspan=\"3\">Horizontal span of 3</td>\n\t\t</tr>\n\t\t<tr class=\"firstrow\">\n\t\t\t<td>first</td>\n\t\t\t<td><acronym title=\"open the pod bay doors\"><span class=\"caps\">HAL</span></acronym></td>\n\t\t\t<td>1</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td>some</td>\n\t\t\t<td style=\"color:green;\">styled</td>\n\t\t\t<td>content</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td rowspan=\"2\">spans 2 rows</td>\n\t\t\t<td>this is</td>\n\t\t\t<td>quite a</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td> deep test </td>\n\t\t\t<td> don&#8217;t you think?</td>\n\t\t</tr>\n\t\t<tr class=\"lastrow\">\n\t\t\t<td>fifth</td>\n\t\t\t<td>I&#8217;m a lumberjack</td>\n\t\t\t<td>5</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td>sixth</td>\n\t\t\t<td> <em><strong>bold italics</strong></em> </td>\n\t\t\t<td>6</td>\n\t\t</tr>\n\t</table>"),
    ("| *strong* |\n\n| _em_ |\n\n| Inter-word -dashes- | ZIP-codes are 5- or 9-digit codes |",
     "\t<table>\n\t\t<tr>\n\t\t\t<td> <strong>strong</strong> </td>\n\t\t</tr>\n\t</table>\n\n\t<table>\n\t\t<tr>\n\t\t\t<td> <em>em</em> </td>\n\t\t</tr>\n\t</table>\n\n\t<table>\n\t\t<tr>\n\t\t\t<td> Inter-word <del>dashes</del> </td>\n\t\t\t<td> <span class=\"caps\">ZIP</span>-codes are 5- or 9-digit codes </td>\n\t\t</tr>\n\t</table>"),
    ("|_. attribute list |\n|<. align left |\n|>. align right|\n|=. center |\n|<>. justify me|\n|^. valign top |\n|~. bottom |",
     "\t<table>\n\t\t<tr>\n\t\t\t<th>attribute list </th>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"text-align:left;\">align left </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"text-align:right;\">align right</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"text-align:center;\">center </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"text-align:justify;\">justify me</td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"vertical-align:top;\">valign top </td>\n\t\t</tr>\n\t\t<tr>\n\t\t\t<td style=\"vertical-align:bottom;\">bottom </td>\n\t\t</tr>\n\t</table>"),
    ("h2. A definition list\n\n;(class#id) Term 1\n: Def 1\n: Def 2\n: Def 3\n;; Center\n;; NATO(Why Em Cee Ayy)\n:: Subdef 1\n:: Subdef 2\n;;; SubSub Term\n::: SubSub Def 1\n::: SubSub Def 2\n::: Subsub Def 3\nWith newline\n::: Subsub Def 4\n:: Subdef 3\n: DEF 4\n; Term 2\n: Another def\n: And another\n: One more\n:: A def without a term\n:: More defness\n; Third term for good measure\n: My definition of a boombastic jazz",
     "<h2>A definition list</h2>\n\n<dl class=\"class\" id=\"id\">\n\t<dt>Term 1</dt>\n\t<dd>Def 1</dd>\n\t<dd>Def 2</dd>\n\t<dd>Def 3\n\t<dl>\n\t\t<dt>Center</dt>\n\t\t<dt><acronym title=\"Why Em Cee Ayy\"><span class=\"caps\">NATO</span></acronym></dt>\n\t\t<dd>Subdef 1</dd>\n\t\t<dd>Subdef 2\n\t\t<dl>\n\t\t\t<dt>SubSub Term</dt>\n\t\t\t<dd>SubSub Def 1</dd>\n\t\t\t<dd>SubSub Def 2</dd>\n\t\t\t<dd>Subsub Def 3<br />\nWith newline</dd>\n\t\t\t<dd>Subsub Def 4</dd>\n\t\t</dl></dd>\n\t\t<dd>Subdef 3</dd>\n\t</dl></dd>\n\t<dd><span class=\"caps\">DEF</span> 4</dd>\n\t<dt>Term 2</dt>\n\t<dd>Another def</dd>\n\t<dd>And another</dd>\n\t<dd>One more\n\t<dl>\n\t\t<dd>A def without a term</dd>\n\t\t<dd>More defness</dd>\n\t</dl></dd>\n\t<dt>Third term for good measure</dt>\n\t<dd>My definition of a boombastic jazz</dd>\n</dl>"),
    ("###. Here's a comment.\n\nh3. Hello\n\n###. And\nanother\none.\n\nGoodbye.", "<h3>Hello</h3>\n\n<p>Goodbye.</p>"),
    ("h2. A Definition list which covers the instance where a new definition list is created with a term without a definition\n\n- term :=\n- term2 := def",
     "<h2>A Definition list which covers the instance where a new definition list is created with a term without a definition</h2>\n\n<dl>\n\t<dt>term</dt>\n\t<dt>term2</dt>\n\t<dd>def</dd>\n</dl>"),
    ("!{height:20px;width:20px;}https://1.gravatar.com/avatar/!",
     "<p><img alt=\"\" src=\"https://1.gravatar.com/avatar/\" style=\"height:20px; width:20px;\" /></p>"),
    ("& test", "<p>&amp; test</p>"),
];


#[test]
fn test_known_values_xhtml() {
    let textile = rustextile::Textile::default()
        .set_html_kind(rustextile::HtmlKind::XHTML);
    for (text, expected) in XHTML_KNOWN_VALUES {
        let processed = textile.parse(text);
        assert_str_eq!(processed, expected);
    }
}
