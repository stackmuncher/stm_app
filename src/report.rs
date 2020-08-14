use anyhow::{anyhow, Error};
use chrono;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use tracing::{error, info, warn};

#[derive(Debug, Serialize, Deserialize, Eq, Clone)]
pub struct KeywordCounter {
    /// keyword
    pub k: String,
    /// array of free text after the keyword
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<HashSet<String>>,
    /// count
    pub c: usize,
}

#[derive(Serialize, Deserialize, Debug, Eq, Clone)]
#[serde(rename = "tech")]
pub struct Tech {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technology: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "tech")]
pub struct Report {
    pub tech: HashSet<Tech>,
    pub timestamp: String,
    pub unprocessed_file_names: HashSet<String>,
    pub unknown_file_types: HashSet<KeywordCounter>,
    pub user_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub repo_name: String,
    /// A UUID of the report
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report_id: String,
    /// A unique name containing user name and project name
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report_name: String,
    /// s3 keys of the reports merged into this one
    pub reports_included: HashSet<String>,
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
    /// .report
    pub const REPORT_FILE_NAME_SUFFIX: &'static str = ".report";

    /// Adds up `tech` totals from `other_report` into `self`, clears unprocessed files and unknown extensions.
    pub fn merge(merge_into: Option<Self>, other_report: Self) -> Option<Self> {
        let mut merge_into = merge_into;

        // the very first report is added with minimal changes
        if merge_into.is_none() {
            info!("Adding 1st report");
            let mut other_report = other_report;
            other_report.unprocessed_file_names.clear();
            merge_into = Some(other_report);
        } else {
            // additional reports are merged
            info!("Merging reports");
            let merge_into_inner = merge_into.as_mut().unwrap();

            // merge all tech records
            for tech in other_report.tech {
                merge_into_inner.add_tech_record(tech);
            }

            // collect names of sub-reports in an array for easy retrieval
            merge_into_inner.reports_included.insert(other_report.report_name);
        }

        merge_into
    }

    /// Add a new Tech record merging with the existing records.
    pub(crate) fn add_tech_record(&mut self, tech: Tech) {
        // add totals to the existing record, if any
        if let Some(mut master) = self.tech.take(&tech) {
            // add up numeric values
            master.docs_comments += tech.docs_comments;
            master.files += tech.files;
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

    /// Generates a new report name in a consistent way.
    pub fn generate_report_name(user_name: &String, repo_name: &String) -> String {
        [
            user_name,
            "/",
            repo_name,
            ".",
            timestamp_as_s3_name().as_str(),
            Report::REPORT_FILE_NAME_SUFFIX,
        ]
        .concat()
    }

    /// Returns true if `name` looks like a report's name.
    /// E.g. AceofGrades/ProceduralMazes.20200811T064638.report
    /// Any leading part is ignored. It only looks at the ending.
    pub fn is_report_name(name: &String) -> bool {
        if name.ends_with(Report::REPORT_FILE_NAME_SUFFIX) {
            true
        } else {
            false
        }
    }

    /// Extracts repo name and date parts of the report name as a tuple.
    /// E.g. AceofGrades/ProceduralMazes.20200811T064638.report
    /// Returns an error if any of the parts cannot be extracted.
    pub fn into_parts(name: &String) -> Result<(String, String), Error> {
        // check if the name is long enough
        if name.len() < 26 {
            error!("Invalid report name {}", name);
            return Err(anyhow!(""));
        }

        // get the start idx of the repo name
        let repo_name_start = match name.rfind("/") {
            None => {
                error!("Invalid report name {}", name);
                return Err(anyhow!(""));
            }
            Some(v) => v,
        };

        let date = &name.as_bytes()[name.len() - 23..name.len() - Report::REPORT_FILE_NAME_SUFFIX.len()];
        let date = match String::from_utf8(date.to_vec()) {
            Err(_e) => {
                error!("Cannot extract date from report name {}", name);
                return Err(anyhow!(""));
            }
            Ok(v) => v,
        };

        let repo_name = &name.as_bytes()[repo_name_start..name.len() - 23];
        let repo_name = match String::from_utf8(repo_name.to_vec()) {
            Err(_e) => {
                error!("Cannot extract repo name from report name {}", name);
                return Err(anyhow!(""));
            }
            Ok(v) => v,
        };

        Ok((repo_name, date))
    }

    /// Create a blank report with the current timestamp.
    pub(crate) fn new(user_name: String, repo_name: String) -> Self {
        let report_name = Report::generate_report_name(&user_name, &repo_name);
        let mut reports_included: HashSet<String> = HashSet::new();
        reports_included.insert(report_name.clone());

        Report {
            tech: HashSet::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            unprocessed_file_names: HashSet::new(),
            unknown_file_types: HashSet::new(),
            user_name: user_name.clone(),
            repo_name: repo_name.clone(),
            report_name,
            report_id: uuid::Uuid::new_v4().to_string(),
            reports_included,
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
                    t: None,
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
        // this should not happen, but handling it just in case
        if new_kw_counter.c == 0 {
            warn!("Empty keywod counter.");
            return;
        }

        // increment if the record exists
        if let Some(mut existing_kw_counter) = hashset.take(&new_kw_counter) {
            existing_kw_counter.c += new_kw_counter.c;

            // additional parts of the keyword need to be added to the set
            if let Some(new_t) = new_kw_counter.t {
                if existing_kw_counter.t.is_none() {
                    existing_kw_counter.t = Some(new_t);
                } else {
                    if let Some(s) = new_t.iter().next().to_owned() {
                        existing_kw_counter.t.as_mut().unwrap().insert(s.to_owned());
                    }
                }
            };

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

impl KeywordCounter {
    /// Returns Self with `t` as `None`. Panics if `keyword` is empty.
    pub(crate) fn new_keyword(keyword: String, count: usize) -> Self {
        if keyword.is_empty() {
            error!("Empty keyword for KeywordCounter in new_keyword");
            panic!();
        }

        Self {
            k: keyword,
            t: None,
            c: count,
        }
    }

    /// Splits `keyword` into `k` and `t`. Panics if `keyword` is empty.
    pub(crate) fn new_ref(keyword: String, count: usize) -> Self {
        if keyword.is_empty() {
            error!("Empty keyword for KeywordCounter in new_ref");
            panic!();
        }

        // output collector
        let mut kwc = Self {
            k: keyword,
            t: None,
            c: count,
        };

        // loop through the characters to find the first boundary
        for (i, c) in kwc.k.as_bytes().iter().enumerate() {
            // keep iterating until the first separator (not ._"')
            if c.is_ascii_alphanumeric() || *c == 46u8 || *c == 95u8 {
                continue;
            }

            // the very first character is a boundary - return as-is
            if i == 0 {
                warn!("Invalid ref: {}", kwc.k);
                return kwc;
            }

            // split the keyword at the boundary
            let (k, t) = kwc.k.split_at(i);
            let mut ths: HashSet<String> = HashSet::new();
            ths.insert(t.to_string());
            kwc.t = Some(ths);
            kwc.k = k.to_string();

            return kwc;
        }

        // return as-is if the keyword is taking the entire length
        // or starts with a boundary
        kwc
    }
}

/// Returns a timestamp as `20200101T163957`
pub fn timestamp_as_s3_name() -> String {
    let ts = chrono::Utc::now().to_rfc3339().into_bytes();

    String::from(String::from_utf8_lossy(&[
        ts[0], ts[1], ts[2], ts[3], ts[5], ts[6], ts[8], ts[9], ts[10], ts[11], ts[12], ts[14], ts[15], ts[17], ts[18],
    ]))
}
