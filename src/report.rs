use anyhow::{anyhow, Error};
use chrono;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use tracing::{error, info};
use super::kwc::{KeywordCounter, KeywordCounterSet};
use super::tech::Tech;

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


impl Report {
    /// .report
    pub const REPORT_FILE_NAME_SUFFIX: &'static str = ".report";

    /// Adds up `tech` totals from `other_report` into `self`, clears unprocessed files and unknown extensions.
    pub fn merge(merge_into: Option<Self>, other_report: Self) -> Option<Self> {
        let mut merge_into = merge_into;
        let mut other_report = other_report;

        // update keyword summaries in all tech records
        let mut new_rep_tech: HashSet<Tech> = HashSet::new();
        for mut tech in other_report.tech.drain() {
            tech.refs_kw = tech.new_kw_summary();
            new_rep_tech.insert(tech);
        }
        other_report.tech = new_rep_tech;

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
                master.keywords.increment_counters(kw);
            }

            // add dependencies
            for kw in tech.refs {
                master.refs.increment_counters(kw);
            }

            // add unique words from dependencies
            if tech.refs_kw.is_some() {
                // init the field if None
                if master.refs_kw.is_none() {
                    master.refs_kw = Some(HashSet::new());
                }

                let refs_kw = master.refs_kw.as_mut().unwrap();
                for kw in tech.refs_kw.unwrap() {
                    refs_kw.increment_counters(kw);
                }
            }
            // re-insert the master record
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
                self.unknown_file_types.increment_counters(ext);
            } else {
                println!("Extension regex failed on {}", file_name);
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

/// Returns a timestamp as `20200101T163957`
pub fn timestamp_as_s3_name() -> String {
    let ts = chrono::Utc::now().to_rfc3339().into_bytes();

    String::from(String::from_utf8_lossy(&[
        ts[0], ts[1], ts[2], ts[3], ts[5], ts[6], ts[8], ts[9], ts[10], ts[11], ts[12], ts[14], ts[15], ts[17], ts[18],
    ]))
}
