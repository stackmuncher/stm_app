use crate::config;
use crate::report;
use content_inspector::ContentType;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use tracing::trace;

pub(crate) fn process_file(file_path: &String, rules: &config::FileRules) -> Result<report::Tech, String> {
    let file_rule_name = rules.file_names.join(", ");

    trace!("\n");
    trace!("{}: {}", file_rule_name, file_path);

    // prepare the blank structure
    let mut tech = report::Tech {
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
        keywords: HashMap::new(), // this is wasteful
        refs: HashMap::new(),     // they should be Option<>
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
        count_matches(&rules.refs_regex, &line, &mut tech.refs);
        count_matches(&rules.keywords_regex, &line, &mut tech.keywords);
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

/// Returns true if there is a regex and it matches the line.
fn count_matches(regex: &Option<Vec<Regex>>, line: &String, hashmap: &mut HashMap<String, usize>) {
    // the output collector
    //let mut hashmap: HashMap<String, usize> = HashMap::new();

    // process if there is a regex
    if let Some(v) = regex {
        for r in v {
            if let Some(groups) = r.captures(line) {
                // The regex may or may not have capture groups. The counts depend on that.
                // We'll assume that if there is only capture[0], which is the whole string,
                // then it's one match. If there is > 1, then it's .len()-1, because capture[0]
                // is always present as the full string match.

                // grab the exact match, if any, otherwise grab the whole string match
                let (cap, group_len) = if groups.len() > 1 {
                    // for g in groups.iter().skip(1) {
                    //     g.unwrap_or_default().as_str()
                    // }

                    let gr_ar: Vec<&str> = groups.iter().skip(1).map(|g| g.unwrap().as_str()).collect();
                    (gr_ar.join(" "), groups.len() - 1)
                //(groups[1].to_string(), groups.len() - 1)
                } else {
                    (groups[0].to_string(), 1)
                };

                trace!("{} x {} for {}", cap, groups.len(), r);

                report::Report::increment_hashmap_counter(hashmap, cap, group_len);
            }
        }
    }
}
