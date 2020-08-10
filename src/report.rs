use chrono;
use regex::Regex;
use serde::Serialize;
use serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use tracing::{error, info, trace};

#[derive(Debug, Serialize, Eq)]
pub struct KeywordCounter {
    /// keyword
    pub k: String,
    /// count
    pub c: usize,
}

#[derive(Serialize, Debug, Eq)]
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
    pub keywords: HashSet<KeywordCounter>, // has to be Option<>
    pub refs: HashSet<KeywordCounter>,     // has to be Option<>
}

#[derive(Serialize, Debug)]
#[serde(rename = "tech")]
pub struct Report {
    pub tech: HashSet<Tech>,
    pub timestamp: String,
    pub unprocessed_file_names: HashSet<String>,
    pub unknown_file_types: HashSet<KeywordCounter>,
    pub user_name: String,
    pub repo_name: String,
    pub report_id: String,
    pub report_name: String,
}

impl std::hash::Hash for KeywordCounter {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.k.as_bytes());
        state.finish();
    }
}

impl PartialEq for KeywordCounter {
    fn eq(&self, other: &Self) -> bool {
        self.k == other.k
    }
}

impl std::hash::Hash for Tech {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.name.as_bytes());
        state.finish();
    }
}

impl PartialEq for Tech {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Report {
    /// Add a new Tech record merging with the existing records.
    pub(crate) fn add_tech_record(&mut self, tech: Tech) {
        // add totals to the existing record, if any
        if let Some(mut master) = self.tech.take(&tech) {
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
            for kw in tech.keywords {
                Report::increment_keyword_counter(&mut master.keywords, kw);
            }

            // add dependencies
            for kw in tech.refs {
                Report::increment_keyword_counter(&mut master.refs, kw);
            }

            // re-insert the master
            self.tech.insert(master);
        } else {
            // there no matching tech record - add it to the hashmap for the 1st time
            self.tech.insert(tech);
        }
    }

    /// Create a blank report with the current timestamp.
    pub(crate) fn new(user_name: String, repo_name: String) -> Self {
        Report {
            tech: HashSet::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            unprocessed_file_names: HashSet::new(),
            unknown_file_types: HashSet::new(),
            user_name: user_name.clone(),
            repo_name: repo_name.clone(),
            report_name: [
                user_name,
                "/".to_string(),
                repo_name,
                "report.".to_string(),
                timestamp_as_s3_name(),
            ]
            .concat(),
            report_id: uuid::Uuid::new_v4().to_string(),
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
                let ext = KeywordCounter {
                    k: ext[0].to_string(),
                    c: 1,
                };
                Report::increment_keyword_counter(&mut self.unknown_file_types, ext);
            } else {
                println!("Extension regex failed on {}", file_name);
            }
        }
    }

    /// Insert a new record or increment the counter for the existing one
    pub(crate) fn increment_keyword_counter(hashset: &mut HashSet<KeywordCounter>, new_kw_counter: KeywordCounter) {
        // increment if the record exists
        if let Some(mut existing_kw_counter) = hashset.take(&new_kw_counter) {
            existing_kw_counter.c += new_kw_counter.c;
            hashset.insert(existing_kw_counter);
        } else {
            // insert if it's a new one
            hashset.insert(new_kw_counter);
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

/// Returns a timestamp as `20200101T163957`
fn timestamp_as_s3_name() -> String {
    let ts = chrono::Utc::now().to_rfc3339().into_bytes();

    String::from(String::from_utf8_lossy(&[
        ts[0], ts[1], ts[2], ts[3], ts[5], ts[6], ts[8], ts[9], ts[10], ts[11], ts[12], ts[14], ts[15], ts[17], ts[18],
    ]))
}
