use chrono;
use regex::Regex;
use serde::Serialize;
use serde_json;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use tracing::{error, info, trace};

#[derive(Serialize, Debug)]
#[serde(rename = "tech")]
pub struct Tech {
    pub name: String,
    pub files: usize,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub bracket_only_lines: usize,
    pub code_lines: usize,
    pub inline_comments: usize,
    pub line_comments: usize,
    pub block_comments: usize,
    pub docs_comments: usize,
    pub keywords: HashMap<String, usize>, // has to be Option<>
    pub refs: HashMap<String, usize>,     // has to be Option<>
}

#[derive(Serialize, Debug)]
#[serde(rename = "tech")]
pub struct Report {
    pub tech: HashMap<String, Tech>,
    pub timestamp: String,
    pub unprocessed_file_names: HashSet<String>,
    pub unknown_file_types: HashMap<String, usize>,
}

impl Report {
    /// Add a new Tech record merging with the existing records.
    pub(crate) fn add_tech_record(&mut self, tech: Tech) {
        // add it to the hashmap if there no matching tech record
        if !self.tech.contains_key(&tech.name) {
            trace!("Inserting report for {}", tech.name);
            self.tech.insert(tech.name.clone(), tech);
            return;
        }

        // add totals to the existing record
        if let Some(master) = self.tech.get_mut(&tech.name) {
            trace!("Adding totals for {}", tech.name);
            // add up numeric values
            master.docs_comments += tech.docs_comments;
            master.files += 1;
            master.inline_comments += tech.inline_comments;
            master.line_comments += tech.line_comments;
            master.total_lines += tech.total_lines;
            master.blank_lines += tech.blank_lines;
            master.block_comments += tech.block_comments;
            master.bracket_only_lines += tech.bracket_only_lines;
            master.code_lines += tech.code_lines;

            // add keyword counts
            for (kw, cnt) in tech.keywords {
                Report::increment_hashmap_counter(&mut master.keywords, kw, cnt);
            }

            // add dependencies
            for (kw, cnt) in tech.refs {
                Report::increment_hashmap_counter(&mut master.refs, kw, cnt);
            }
        }
    }

    /// Create a blank report with the current timestamp.
    pub(crate) fn new() -> Self {
        Report {
            tech: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            unprocessed_file_names: HashSet::new(),
            unknown_file_types: HashMap::new(),
        }
    }

    /// Add a file that won't be processed because it is of unknown type.
    pub(crate) fn add_unprocessed_file(&mut self, file_name: &String, project_dir_path: &String) {
        // remove the project path from the file name
        let mut file_name = file_name.clone(); // I don't like `mut` in the function signature
        file_name.drain(..project_dir_path.len()); // remove the path
        if file_name.starts_with("/") || file_name.starts_with("\\") {
            file_name.drain(..1); // remove the leading / or \, if any
        }

        // add the file name to the list
        self.unprocessed_file_names.insert(file_name.clone());

        // check if this particular extension was encountered
        if let Some(ext) = Regex::new(r"\.[\w\d_]+$").unwrap().captures(&file_name) {
            if ext.len() == 1 {
                let ext = ext[0].to_string();
                Report::increment_hashmap_counter(&mut self.unknown_file_types, ext, 1);
            } else {
                println!("Extension regex failed on {}", file_name);
            }
        }
    }

    /// Insert a new record or increment the counter for the existing one
    pub(crate) fn increment_hashmap_counter(hashmap: &mut HashMap<String, usize>, key: String, value: usize) {
        match hashmap.get_mut(&key) {
            Some(cnt) => {
                *cnt = *cnt + value;
            }
            None => {
                hashmap.insert(key, value);
            }
        }
    }

    /// First it tries to save into the specified location. If that failed it saves into the local folder.
    pub fn save_as_local_file(&self, file_name: &String) {
        // try to create the file
        let mut file = match File::create(file_name) {
            Err(e) => {
                error!("Cannot save in {} due to {}", file_name, e);
                panic!();
            }
            Ok(f) => {
                info!("Saving into {}", file_name);
                f
            }
        };

        write!(file, "{}", self).expect("Failed to save in the specified location. ");
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(v) => {
                write!(f, "{}", v).expect("Invalid JSON string in report.");
            }
            Err(e) => {
                write!(f, "Cannot serialize Report {:?}", e).expect("Invalid error msg in report.");
            }
        }
        Ok(())
    }
}
