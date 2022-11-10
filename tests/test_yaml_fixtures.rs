use std::collections::BTreeMap;
use std::borrow::Cow;
use fancy_regex::{Regex, Captures};

use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use pretty_assertions::assert_str_eq;

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize, Debug)]
struct ParserSettings {
    setRestricted: Option<bool>,
    setLite: Option<bool>,
    setImages: Option<bool>,
    setLinkRelationShip: Option<String>,
    setUid: Option<String>,
    setGettingImageSize: Option<bool>,
    setHtmlType: Option<String>,
    setBlockTags: Option<bool>,
}

impl ParserSettings {
    fn apply(&self, mut parser: rustextile::Textile) -> rustextile::Textile {
        if let Some(value) = self.setRestricted {
            parser = parser.set_restricted(value);
        }
        if let Some(value) = self.setLite {
            parser = parser.set_lite(value);
        }
        if let Some(value) = self.setImages {
            parser = parser.set_images(value);
        }
        if let Some(ref value) = self.setLinkRelationShip {
            parser = parser.set_rel(Some(value.clone()));
        }
        if let Some(ref value) = self.setUid {
            parser = parser.set_uid(value);
        }
        if let Some(value) = self.setGettingImageSize {
            parser = parser.set_getting_image_size(value);
        }
        if let Some(ref value) = self.setHtmlType {
            parser = parser.set_html_kind(match value.as_str() {
                "xhtml" | "XHTML" => rustextile::HtmlKind::XHTML,
                "html5" | "HTML5" => rustextile::HtmlKind::HTML5,
                _ => panic!("Unsupported type of HTML: {}", value)
            });
        }
        if let Some(value) = self.setBlockTags {
            parser = parser.set_block_tags(value);
        }
        parser
    }
}

/// YAML contains chunks like "\x20" which, although totally valid,
/// for some reason are not recognized by serde_yaml at the moment,
/// and have to be converted into their respective characters by this function.
fn replace_xcodes(text: &str) -> Cow<str> {
    lazy_static! {
        static ref XCODE: Regex = Regex::new(r"\\x(\d{2})").unwrap();
    }
    XCODE.replace_all(text, |cap: &Captures| -> String {
        let char_code_str = &cap[1];
        match u32::from_str_radix(char_code_str, 16) {
            Ok(code) => char::from_u32(code)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| String::from(char_code_str)),
            Err(_) => String::from(char_code_str),
        }
    })
}

#[derive(Deserialize, Serialize, Debug)]
struct Fixture {
    input: String,
    expect: String,
    setup: Option<ParserSettings>,
    notes: Option<String>,
    assert: Option<String>,
}

fn normalize_newlines(text: &str) -> String {
    text.trim().replace('\t', "").lines().map(|l| l.trim()).collect()
}

impl Fixture {
    fn build_parser(&self) -> rustextile::Textile {
        let parser = rustextile::Textile::default().set_uid("xyz");
        if let Some(ref settings) = self.setup {
            settings.apply(parser)
        } else {
            parser
        }
    }

    fn run(&self, fixture_path: &std::path::Path, fixture_name: &str) {
        if self.assert.as_ref().map(|v| v == "skip") == Some(true) {
            println!("\tSkipping fixture {fixture_name:#?}");
            return;
        } else {
            println!("\tRunning fixture {fixture_name:#?}");
        }
        let parser = self.build_parser();
        let input_textile = replace_xcodes(self.input.trim());
        let result = parser.parse(&input_textile);
        let trimmed_result: String = normalize_newlines(&result);
        let trimmed_expectation: String = normalize_newlines(&self.expect);
        let notes = self.notes.as_deref().unwrap_or_default();
        assert_str_eq!(
            trimmed_result,
            trimmed_expectation,
            concat!("\nFailed on fixture \"{}\" from {:#?}\n",
                    "Fixture note: \"{}\"\n",
                    "Input Textile: {:#?}"),
            fixture_name,
            fixture_path,
            notes,
            input_textile
        );
    }
}

#[test]
fn test_xcode_replacer() {
    let result = replace_xcodes("-b-\\x20<br />");
    assert_eq!(result, "-b- <br />")
}

fn run_yaml_fixtures(names: &[&str]) {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").unwrap());
    for fixture_name in names {
        let fixture_path = manifest_dir.join(format!("tests/fixtures/{}.yaml", *fixture_name));
        let fixture_file = std::fs::File::open(&fixture_path).unwrap();
        let fixture_data_result: serde_yaml::Result<BTreeMap<String, Fixture>> = serde_yaml::from_reader(fixture_file);
        match fixture_data_result {
            Ok(fixture_data) => {
                println!("Running fixtures from {}", fixture_path.to_string_lossy());
                for (fixture_name, fixture) in fixture_data.iter() {
                    fixture.run(fixture_path.as_path(), fixture_name);
                }
            },
            Err(e) => {
                panic!("Unable to read fixture {}: {}", fixture_path.to_string_lossy(), e)
            }
        }
    }
}

#[test]
fn test_yaml_fixtures() {
    run_yaml_fixtures(&[
        "limits",
        "basic",
        "codeblocks",
        "images",
        "links",
        "dividers",
        "inline-code",
        "span-wrappers",
        "issue-22",
        "issue-24",
        "issue-40",
        "issue-63",
        "issue-65",
        "issue-106",
        "issue-120",
        "issue-123",
        "issue-128",
        "issue-129",
        "issue-131",
        "issue-132",
        "issue-135",
        "issue-141",
        "issue-142",
        "issue-143",
        "issue-144",
        "issue-145",
        "issue-158",
        "issue-164",
        "issue-168",
        "issue-172",
        "issue-189",
        "issue-198",
        "issue-202",
    ]);
}
