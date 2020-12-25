use anyhow::Error;
use regex::Regex;
use std::fs;
use std::path::Path;
use tracing::{error, info, trace};

pub mod code_rules;
pub mod file_type;
pub mod kwc;
pub mod muncher;
pub mod processors;
pub mod report;
pub mod tech;
pub mod config;

pub async fn process_project(
    conf: &mut code_rules::CodeRules,
    project_dir: &String,
    user_name: &String,
    repo_name: &String,
    old_report: Option<report::Report>,
) -> Result<report::Report, Error> {
    info!("Analyzing code from {}", project_dir);

    // collects hashes of munchers that should be ignored for this project because they have
    // not changed since the last processing of the repo
    // collect all hashes from the old report as this stage and then
    // remove them from the list as mucnhers are loaded
    let mut old_munchers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut unchanged_munchers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    if let Some(old_report) = old_report.as_ref() {
        for tech in &old_report.tech {
            if tech.muncher_hash > 0 {
                old_munchers.insert(tech.muncher_hash);
            }
        }
        info!("Encountered an old report with {} tech sections", old_munchers.len());
    }

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
            // check if the old report was processed by the same muncher and can be skipped
            if old_munchers.contains(&muncher.muncher_hash) {
                unchanged_munchers.insert(muncher.muncher_hash);
                processed_files.push(file_path.clone());
                trace!("Unchanged muncher for {}", file_path);
                continue;
            }

            // process the file with the rules from the muncher
            if let Ok(tech) = processors::process_file(&file_path, muncher) {
                processed_files.push(file_path.clone());
                report.add_tech_record(tech);
            }
        }
    }

    // copy tech reports for unchanged munchers from the old report, if any
    if let Some(old_report) = old_report {
        for tech in old_report.tech {
            if tech.muncher_hash > 0 && unchanged_munchers.contains(&tech.muncher_hash) {
                info!(
                    "Copied {}/{}/{} tech section from the old report",
                    tech.language, tech.muncher_name, tech.muncher_hash
                );
                report.add_tech_record(tech);
            }
        }

        // copy the commit info because the repo has not changed
        // if the repo changed there would be no old report
        report.collaborators = old_report.collaborators;
        report.date_head = old_report.date_head;
        report.date_init = old_report.date_init;
        info!("Copied commit info from the old report");
    } else {
        report.extract_commit_info(&project_dir).await;
    }

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

