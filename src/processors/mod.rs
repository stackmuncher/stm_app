use crate::config;
use crate::report;
use content_inspector::ContentType;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};

pub(crate) fn process_file(file_path: &String, rules: &config::FileRules) -> Result<report::Tech, String> {
    println!("{}: {}", rules.name, file_path);

    // prepare the blank structure
    let mut tech = report::Tech {
        name: rules.name.clone(),
        files: 0,
        total_lines: 0,
        code_lines: 0,
        line_comments: 0,
        block_comments: 0,
        docs_comments: 0,
        inline_comments: 0,
        blank_lines: 0,
        bracket_only_lines: 0,
        keywords: HashMap::new(),
        use_dependencies: HashSet::new(),
    };

    // get file contents
    let lines = get_file_lines(&file_path);
    if lines.len() == 0 {
        // exit now if the file is either empty or binary
        return Ok(tech);
    }

    // get total lines
    tech.total_lines = lines.len();

    // prepare the regex
    let bracket_only = Regex::new(rules.bracket_only.as_str()).expect("Bad regex for bracket_only");
    let line_comments = Regex::new(rules.line_comments.as_str()).expect("Bad regex for line_comments");
    let inline_comments = Regex::new(rules.inline_comments.as_str()).expect("Bad regex for inline_comments");
    let doc_comments = Regex::new(rules.doc_comments.as_str()).expect("Bad regex for doc_comments");
    //let block_comments = Regex::new(rules.block_comments.as_str());
    let use_dependency = Regex::new(rules.use_dependency.as_str()).expect("Bad regex for use_dependency");
    let blank_line = Regex::new(r"^\s*$").expect("Bad regex for blank_line - hardcoded");

    // evaluate every line
    for line in lines {
        // check for non-code parts
        if doc_comments.is_match(&line) {
            tech.docs_comments += 1;
            continue;
        }

        if line_comments.is_match(&line) {
            tech.line_comments += 1;
            continue;
        }

        if bracket_only.is_match(&line) {
            tech.bracket_only_lines += 1;
            continue;
        }

        if blank_line.is_match(&line) {
            continue;
        }

        if inline_comments.is_match(&line) {
            tech.inline_comments += 1;
            continue;
        }

        // this is a code line of sorts
        tech.code_lines += 1;

        // get the dependency, if any
        if let Some(caps) = use_dependency.captures(&line) {
            if caps.len() == 2 {
                tech.use_dependencies.insert(caps[1].to_string());
                continue;
            } else {
                println!("Failed dependency capture: {}", line);
            }
        }

        // try to extract the keywords from the line
        


    }

    //println!("\nTech:\n{:?}", tech);

    Ok(tech)
}

/// Returns multiple lines from a text file. It does not check if the file
/// is text or binary. Will return an empty array if the file is empty or
/// is not text. Panics if the file cannot be open,
fn get_file_lines(asset_path: &String) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    let file = fs::File::open(asset_path.as_str()).expect("Cannot open the file");
    let reader = io::BufReader::new(file);

    match content_inspector::inspect(&reader.buffer()) {
        ContentType::UTF_8 | ContentType::UTF_8_BOM => {
            // do nothing - we can process UTF-8 only
        }
        _ => {
            return lines;
        }
    };

    for line in reader.lines() {
        lines.push(line.unwrap());
    }

    lines
}
