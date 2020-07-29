use crate::config;
use crate::report;
use content_inspector::ContentType;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};
use tracing::{error, trace};

pub(crate) fn process_file(file_path: &String, rules: &config::FileRules) -> Result<report::Tech, String> {
    trace!("\n");
    trace!("{}: {}", rules.name, file_path);

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
        trace!("Empty or binary file - not processing.");
        return Ok(tech);
    }

    // get total lines
    tech.total_lines = lines.len();

    // evaluate every line
    for line in lines {
        trace!("{}", line);
        // check for non-code parts
        if rules.doc_comments_regex.is_some() && rules.doc_comments_regex.as_ref().unwrap().is_match(&line) {
            tech.docs_comments += 1;
            trace!("doc_comments");
            continue;
        }

        if rules.line_comments_regex.is_some() && rules.line_comments_regex.as_ref().unwrap().is_match(&line) {
            tech.line_comments += 1;
            trace!("line_comments");
            continue;
        }

        if rules.bracket_only_regex.is_some() && rules.bracket_only_regex.as_ref().unwrap().is_match(&line) {
            tech.bracket_only_lines += 1;
            trace!("bracket_only");
            continue;
        }

        if rules.blank_line_regex.is_some() && rules.blank_line_regex.as_ref().unwrap().is_match(&line) {
            trace!("blank_line");
            continue;
        }

        if rules.inline_comments_regex.is_some() && rules.inline_comments_regex.as_ref().unwrap().is_match(&line) {
            tech.inline_comments += 1;
            trace!("inline_comments");
            continue;
        }

        // this is a code line of sorts
        tech.code_lines += 1;
        trace!("code_lines");

        // get the dependency, if any
        if rules.use_dependency_regex.is_some() {
            if let Some(caps) = rules.use_dependency_regex.as_ref().unwrap().captures(&line) {
                if caps.len() == 2 {
                    let cap = caps[1].to_string();
                    trace!("dependency: {}", cap);
                    tech.use_dependencies.insert(cap);
                    continue;
                } else {
                    error!("Failed dependency capture");
                }
            }
        }

        // try to extract the keywords from the line
        if rules.keywords_regex.is_some() {
            for (kw, re) in rules.keywords_regex.as_ref().unwrap() {
                //trace!("-{}", kw);
                if re.is_match(&line) {
                    report::Report::increment_hashmap_counter(&mut tech.keywords, kw.clone(), 1);
                    trace!("kw: {}", kw);
                }
            }
        }
    }

    Ok(tech)
}

/// Returns multiple lines from a text file. It does not check if the file
/// is text or binary. Will return an empty array if the file is empty or
/// is not text. Panics if the file cannot be open,
fn get_file_lines(asset_path: &String) -> Vec<String> {
    // read the file
    let file = fs::File::open(asset_path.as_str()).expect("Cannot open the file");
    let reader = io::BufReader::new(file);

    // check if the content is text and can be split into lines
    let content_type = content_inspector::inspect(&reader.buffer());
    match content_type {
        ContentType::UTF_8 | ContentType::UTF_8_BOM => {
            // success - we will process the lines later
        }
        _ => {
            // the content is not text - return an empty array
            trace!("Binary file: {}", content_type);
            return Vec::new();
        }
    };

    // convert the file into a collection of lines
    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(l) => lines.push(l),
            Err(e) => trace!("Unreadable line: {:?}", e),
        }
    }

    lines
}
