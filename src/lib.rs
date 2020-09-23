use anyhow::Error;
use regex::Regex;
use std::fs;
use std::path::Path;
use tracing::{error, info};

#[path = "code_rules.rs"]
pub mod code_rules;
#[path = "file_type.rs"]
pub mod file_type;
#[path = "kwc.rs"]
pub mod kwc;
#[path = "muncher.rs"]
pub mod muncher;
#[path = "processors/mod.rs"]
pub mod processors;
#[path = "report.rs"]
pub mod report;
#[path = "tech.rs"]
pub mod tech;

pub async fn process_project(
    conf: &mut code_rules::CodeRules,
    project_dir: &String,
    user_name: &String,
    repo_name: &String,
) -> Result<report::Report, Error> {
    info!("Analyzing code from {}", project_dir);

    // get list of files
    let mut files = get_file_names_recursively(Path::new(project_dir));

    // remove .git/ files from the list
    let re = Regex::new(r"\.git/").unwrap();
    files.retain(|f| !re.is_match(f.as_str()));

    // result collectors
    let mut processed_files: Vec<String> = Vec::new();
    let mut report = report::Report::new(user_name.clone(), repo_name.clone());

    // loop through all the files and process them one by one
    for file_path in &files {
        // fetch the right muncher
        if let Some(muncher) = conf.get_muncher(file_path) {
            // process the file with the rules from the muncher
            if let Ok(tech) = processors::process_file(&file_path, muncher) {
                processed_files.push(file_path.clone());
                report.add_tech_record(tech);
            }
        }
    }

    // add commit details
    report.extract_commit_info(&project_dir).await;

    // discard processed files
    info!("Adding un-processed files");
    files.retain(|f| !processed_files.contains(&f));

    // log unprocessed files in the report
    for f in &files {
        report.add_unprocessed_file(f, project_dir);
    }

    info!("Analysis finished");
    Ok(report)
}

fn get_file_names_recursively(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let mut f = get_file_names_recursively(&path);
                files.append(&mut f);
            } else if path.is_file() {
                // some files cropped up with None after the conversion, so unwrapping safely
                if let Some(f_n) = entry.path().to_str() {
                    files.push(f_n.to_owned());
                }
            }
        }
    } else {
        error!(
            "get_file_names_recursively: {} is not a dir",
            dir.to_str().unwrap().to_owned()
        );
    }

    files
}
#[derive(Debug)]
pub struct Params {
    /// Full path to the config file. Absolute or relative to the working dir.
    pub config_file_path: String,
    pub log_level: tracing::Level,
    /// Absolute or relative path to the project directory with the files to analyze.
    pub project_dir_path: String,
    /// File name only. Reports are always saved in the current dir
    pub report_file_name: String,
    /// registered user name (the validity is not enforced at the moment as it's not pushed anywhere)
    pub user_name: String,
    /// Repo name. Must be unique per user. Reports are attached to `user/repo` ID.
    pub repo_name: String,
}

pub const ENV_CONF_PATH: &'static str = "STACK_MUNCHER_CODERULES_PATH";

impl Params {
    /// Returns a minimal version of Self with no validation.
    pub fn from_ext_config(code_rules_file_location: String, user_name: String, repo_name: String) -> Self {
        Params {
            log_level: tracing::Level::INFO,
            config_file_path: code_rules_file_location,
            project_dir_path: String::new(),
            report_file_name: String::new(),
            user_name,
            repo_name,
        }
    }
}
