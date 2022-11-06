use std::borrow::Cow;

use fancy_regex::Regex;
use lazy_static::lazy_static;

use crate::block::{BlockAttributes, BlockHtmlAttributes};
use crate::regextra::{split_with_capture, fregex};
use crate::regex_snips::{ALIGN_RE_S, CLS_RE_S, VALIGN_RE_S, SNIP_SPACE, PNCT_RE_S};
use crate::htmltools::generate_tag;


const COLSPAN_RE_S: &str = r"(?:\\\d+)";
const ROWSPAN_RE_S: &str = r"(?:\/\d+)";
lazy_static! {
    pub static ref TABLE_SPAN_RE_S: String = format!(
        r"(?:{0}|{1})*", COLSPAN_RE_S, ROWSPAN_RE_S);
}

fn process_caption(capts: &str, cap: &str, restricted: bool) -> String {
    let html_attributes = BlockAttributes::parse(capts, None, true, restricted).html_attrs();
    let tag = generate_tag("caption", Some(cap.trim()), &html_attributes);
    format!("\t{0}\n", tag)
}

struct TableSection {
    tag: String,
    attributes: BlockAttributes,
    rows: Vec<String>
}

impl TableSection {
    fn new(tag: String, attributes: BlockAttributes) -> Self {
        TableSection { tag, attributes, rows: Default::default() }
    }

    fn process(self) -> String {
        let content = self.rows.join("") + "\n\t";
        generate_tag(&self.tag, Some(&content), &self.attributes.html_attrs())
    }
}

struct Row {
    cells: Vec<String>,
    attributes: BlockHtmlAttributes,
}

impl Row {
    fn new(attributes: BlockHtmlAttributes) -> Self {
        Self {
            attributes,
            cells: Default::default(),
        }
    }

    fn process(&self) -> String {
        let cell_data = self.cells.join("") + "\n\t\t";
        format!(
            "\n\t\t{}",
            generate_tag("tr", Some(&cell_data), &self.attributes))
    }
}


pub(crate) fn process_table<'t>(
    parser: &mut crate::parser::ParserState,
    tatts: &'t str,
    rows_str: &'t str,
    summary: Option<&'t str>
) -> String
{
    lazy_static! {
        static ref COMPONENTS_RE: Regex = fregex!(
            &format!(r"(?m)\|{0}*?$", SNIP_SPACE));
        static ref CAPTION_RE: Regex = fregex!(
               &format!(
                   r"(?s)^\|\=(?P<capts>{s}{a}{c})\. (?P<cap>[^\n]*)(?P<row>.*)",
                   s=*TABLE_SPAN_RE_S, a=*ALIGN_RE_S, c=*CLS_RE_S));

        static ref GRPMATCH_RE: Regex = fregex!(
            &format!(
                concat!(r"(?ms)(:?^\|(?P<part>{v})(?P<rgrpatts>{s}{a}{c})",
                        r"\.{space}*$\n)?^(?P<row>.*)"),
                v=VALIGN_RE_S, s=*TABLE_SPAN_RE_S, a=*ALIGN_RE_S, c=*CLS_RE_S,
                space=SNIP_SPACE));


        static ref RMTCH_RE: Regex = fregex!(
            &format!(r"^(?P<ratts>{0}{1}\. )(?P<row>.*)",
                     *ALIGN_RE_S, *CLS_RE_S));

        static ref CMTCH_RE: Regex = fregex!(
            &format!(r"(?s)^(?P<catts>_?{0}{1}{2}\. )(?P<cell>.*)",
                     *TABLE_SPAN_RE_S, *ALIGN_RE_S, *CLS_RE_S));
        static ref CELL_A_PATTERN_RE: Regex = fregex!(
            &format!(r"(?s)(?P<space>{0}*)(?P<cell>.*)", SNIP_SPACE));
        static ref COLGROUP_RE: Regex = fregex!(
            &format!(r"(?m)^\|:(?P<cols>{s}{a}{c}\. .*)",
                     s=*TABLE_SPAN_RE_S, a=*ALIGN_RE_S, c=*CLS_RE_S));
        static ref HEADING_RE: Regex = fregex!(
            &format!(r"^_(?={0}|{1})", SNIP_SPACE, PNCT_RE_S));
    }
    let mut html_attrs = BlockAttributes::parse(tatts, Some("table"), true, parser.textile.restricted).html_attrs();

    if let Some(s) = summary {
        if !s.is_empty() {
            html_attrs.insert("summary", s.trim().to_owned());
        }
    }
    let mut caption = String::new();
    let mut colgroup = String::new();
    let mut content = Vec::<String>::new();
    let mut rgrp: Option<TableSection> = None;
    let mut groups = Vec::<String>::new();

    let non_empty_rows =
        split_with_capture(&COMPONENTS_RE, rows_str)
        .filter(|row| !row.is_empty());
    for (i, row) in non_empty_rows.enumerate() {
        let row = Cow::Borrowed(row.trim_start());

        // # Caption -- only occurs on row 1, otherwise treat '|=. foo |...'
        // # as a normal center-aligned cell.
        let row = if i == 0 {
            if let Ok(Some(cmtch)) = CAPTION_RE.captures(&row) {
                caption = format!(
                    "\n{}",
                    process_caption(
                        &cmtch["capts"],
                        &cmtch["cap"],
                        parser.textile.restricted));
                let new_row = cmtch["row"].trim_start();
                if new_row.is_empty() {continue} else {new_row.to_owned().into()}
            } else {
                row
            }
        } else {
            row
        };
        // Colgroup -- A colgroup row will not necessarily end with a |.
        // Hence it may include the next row of actual table data.
        let row = if let Ok(Some(gmtch)) = COLGROUP_RE.captures(&row) {
            // Is this colgroup def missing a closing pipe? If so, there
            // will be a newline in the middle of $row somewhere.
            let cols = &gmtch[1].replace('.', "");
            for (idx, col) in cols.split('|').enumerate() {
                let group_atts: String = BlockAttributes
                    ::parse(col.trim(), Some("col"), true, parser.textile.restricted)
                    .into();
                colgroup.push_str("\t<col");
                if idx == 0 {
                    colgroup.push_str("group");
                    colgroup.push_str(&group_atts);
                    colgroup.push('>');
                } else {
                    colgroup.push_str(&group_atts);
                    colgroup.push_str(" />");
                }
                colgroup.push('\n');
            }
            colgroup.push_str("\t</colgroup>");
            let row_newline = row.find('\n');
            if let Some(nl_index) = row_newline {
                Cow::Borrowed(row[nl_index..].trim_start())
            } else {
                continue
            }
        } else {
            row
        };
        // search the row for a table group - thead, tfoot, or tbody
        let grpmatch_cap = GRPMATCH_RE.captures(row.trim_start());
        let row = if let Ok(Some(ref grpmatch)) = grpmatch_cap {
            if let (Some(grpname), Some(rgrpatts))
                = (grpmatch.name("part"), grpmatch.name("rgrpatts")) {
                // we're about to start a new group, so process the current one
                // and add it to the output
                if let Some(rgrp_data) = rgrp {
                    groups.push(format!("\n\t{0}", rgrp_data.process()));
                }
                let section_tag = match grpname.as_str() {
                    "^" => "thead",
                    "~" => "tfoot",
                    "-" => "tbody",
                    _ => unreachable!()
                };
                rgrp = Some(
                    TableSection::new(
                        section_tag.to_owned(),
                        BlockAttributes::parse(
                            rgrpatts.as_str(),
                            None,
                            true,
                            parser.textile.restricted)));
            }
            Cow::Borrowed(&grpmatch["row"])
        } else {
            row
        };

        let rmtch_cap = RMTCH_RE.captures(row.trim_start());
        let (row, row_atts) = match rmtch_cap {
            Ok(Some(ref rmtch)) => (
                Cow::Borrowed(&rmtch["row"]),
                BlockAttributes::parse(&rmtch["ratts"], Some("tr"), true, parser.textile.restricted).html_attrs()
            ),
            _ => (row, BlockHtmlAttributes::default()),
        };

        // create a row to hold the cells.
        let mut r = Row::new(row_atts);
        for (_cellctr, cell) in row.split('|').skip(1).enumerate() {
            let ctag = match HEADING_RE.is_match(cell) {
                Ok(true) => "th",
                _ => "td"
            };

            let cmtch_cap = CMTCH_RE.captures(cell);
            let (cell, cell_atts) = match cmtch_cap {
                Ok(Some(ref cmtch)) => (
                    &cmtch["cell"],
                    BlockAttributes::parse(&cmtch["catts"], Some("td"), true, parser.textile.restricted).html_attrs()
                ),
                _ => (cell, BlockHtmlAttributes::default())
            };

            let cell = if !parser.textile.lite {
                Cow::Owned(
                    if let Ok(Some(a)) = CELL_A_PATTERN_RE.captures(cell) {
                        let cell = parser.redcloth_list(&a["cell"]);
                        let cell = parser.textile_lists(&cell);
                        a["space"].to_owned() + cell.as_ref()
                    } else {
                        String::new()
                    }
                )
            } else {
                Cow::Borrowed(cell)
            };

            // create a cell
            let c = generate_tag(ctag, Some(&cell), &cell_atts);
            let cline_tag = format!("\n\t\t\t{0}", c);
            // add the cell to the row
            r.cells.push(parser.do_tag_br(ctag, &cline_tag).into_owned());
        }
        // if we're in a group, add it to the group's rows, else add it
        // directly to the content
        if let Some(ref mut rgrp_data) = rgrp {
            rgrp_data.rows.push(r.process());
        } else {
            content.push(r.process());
        }
    }
    // if there's still an rgrp, process it and add it to the output
    if let Some(rgrp_data) = rgrp {
        groups.push(format!("\n\t{0}", rgrp_data.process()));
    }

    let tag_content = format!(
        "{0}{1}{2}{3}\n\t",
        caption, colgroup, groups.join(""), content.join(""));
    let tbl = generate_tag("table", Some(&tag_content), &html_attrs);
    format!("\t{0}\n\n", tbl)
}
