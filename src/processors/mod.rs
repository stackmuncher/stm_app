use super::code_rules;
use encoding_rs as _;
use encoding_rs_io::DecodeReaderBytes;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use tracing::{warn, trace};
use super::tech::Tech;

pub(crate) fn process_file(file_path: &String, rules: &code_rules::FileRules) -> Result<Tech, String> {
    let file_rule_name = rules.file_names.join(", ");

    trace!("\n");
    trace!("{}: {}", file_rule_name, file_path);

    // prepare the blank structure
    let mut tech = Tech {
        language: rules.language.clone(),
        technology: rules.technology.clone(),
        name: file_rule_name,
        files: 1,
        total_lines: 0,
        code_lines: 0,
        line_comments: 0,
        block_comments: 0,
        docs_comments: 0,
        inline_comments: 0,
        blank_lines: 0,
        bracket_only_lines: 0,
        keywords: HashSet::new(), // this is wasteful
        refs: HashSet::new(),     // they should be Option<>
        refs_kw: None,
    };

    // get file contents
    let lines = get_file_lines(&file_path);
    if lines.len() == 0 {
        // exit now if the file is either empty or binary
        trace!("Empty or binary file - not processing.");
        return Ok(tech);
    }

    // get total lines
    tech.total_lines = lines.len();

    // set to true when the line is inside a block comment
    let mut inside_block_comment = false;

    // evaluate every line
    for line in lines {
        trace!("{}", line);
        // check for non-code parts

        // check if it's inside a block comment
        if inside_block_comment {
            tech.block_comments += 1;
            trace!("block_comments");
            // is it a closing block?
            if match_line(&rules.block_comments_end_regex, &line) {
                inside_block_comment = false;
            }
            continue;
        }

        if match_line(&rules.block_comments_start_regex, &line) {
            tech.block_comments += 1;
            trace!("block_comments");

            // mark it as the start of the block if there is no closing part on the same line
            if !match_line(&rules.block_comments_end_regex, &line) {
                inside_block_comment = true;
            }

            continue;

            // It is possible that some code may have multiple opening / closing comments on the same page.
            // That would probably be just messy code that can be ignored.
            // Those comments may also be inside string literals, e.g. "some text like this /*".
            // The same applies to other types of comments - they can be inside " ... "
        }

        if match_line(&rules.doc_comments_regex, &line) {
            tech.docs_comments += 1;
            trace!("doc_comments");
            continue;
        }

        if match_line(&rules.line_comments_regex, &line) {
            tech.line_comments += 1;
            trace!("line_comments");
            continue;
        }

        if match_line(&rules.inline_comments_regex, &line) {
            tech.inline_comments += 1;
            trace!("inline_comments");
            continue;
        }

        if match_line(&rules.bracket_only_regex, &line) {
            tech.bracket_only_lines += 1;
            trace!("bracket_only_lines");
            continue;
        }

        if match_line(&rules.blank_line_regex, &line) {
            tech.blank_lines += 1;
            trace!("blank_lines");
            continue;
        }

        // this is a code line of sorts
        tech.code_lines += 1;
        trace!("code_lines");

        // get the dependency, if any
        tech.count_refs(&rules.refs_regex, &line);
        tech.count_keywords(&rules.keywords_regex, &line);
    }

    Ok(tech)
}

/// Returns multiple lines from a text file. It does not check if the file
/// is text or binary. Will return an empty array if the file is empty or
/// is not text. Panics if the file cannot be open,
fn get_file_lines(asset_path: &String) -> Vec<String> {
    // read the file
    let file = fs::File::open(asset_path.as_str()).expect("Cannot open the file");
    // this decoder is required to read non-UTF-8 files
    let mut decoder = DecodeReaderBytes::new(file);

    // output collector
    let mut lines: Vec<String> = Vec::new();

    // try to read the file
    let mut utf8_string = String::new();
    if let Err(e) = decoder.read_to_string(&mut utf8_string) {
        // just skip if it cannot be read
        warn!("Cannot decode {} due to {}", asset_path, e);
        return lines;
    }

    // convert the file into a collection of lines
    for line in utf8_string.as_str().lines() {
        lines.push(line.into());
    }

    lines
}

/// Returns true if there is a regex and it matches the line.
fn match_line(regex: &Option<Vec<Regex>>, line: &String) -> bool {
    if let Some(v) = regex {
        for r in v {
            if r.is_match(&line) {
                trace!("{}", r);
                return true;
            }
        }
    }

    // no match found
    false
}
