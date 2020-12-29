use report::Report;
use std::{collections::HashSet, path::Path};
use tracing::{info, trace};

pub mod code_rules;
pub mod config;
pub mod file_type;
pub mod kwc;
pub mod muncher;
pub mod processors;
pub mod report;
pub mod tech;

/// Processes the entire repo with or without a previous report. If the report is present and the munchers
/// have not changed the relevant sections are copied from the old report. Use this function when:
/// * it's a new repo
/// * the munchers changed and the entire repo needs to be reprocessed
pub async fn process_project(
    code_rules: &mut code_rules::CodeRules,
    project_dir: &String,
    user_name: &String,
    repo_name: &String,
    old_report: Option<report::Report>,
) -> Result<report::Report, ()> {
    // all files to be processed
    let files = get_last_commit_files(Path::new(project_dir)).await?;

    let files = match old_report.as_ref() {
        Some(v) => filter_out_files_with_unchanged_munchers(code_rules, v, files),
        None => files,
    };

    // just return the old report if there were no changes and the old report can be re-used
    if old_report.is_some() && files.is_empty() {
        return Ok(old_report.unwrap());
    }

    // generate the report
    let report = process_project_files(code_rules, project_dir, user_name, repo_name, old_report, &files).await?;

    // update the report with additional info
    let report = report.extract_commit_info(project_dir).await;
    let report = report.update_list_of_tree_files(files);

    Ok(report)
}

/// Processes specified files from the repo and returns a report with Tech and Tech per file sections.
pub async fn process_project_files(
    code_rules: &mut code_rules::CodeRules,
    project_dir: &String,
    user_name: &String,
    repo_name: &String,
    old_report: Option<report::Report>,
    files: &Vec<String>,
) -> Result<report::Report, ()> {
    info!("Analyzing code from {}", project_dir);

    // result collectors
    let mut report = report::Report::new(user_name.clone(), repo_name.clone());
    let mut per_file_tech: Vec<String> = Vec::new();
    let mut updated_tech: HashSet<u64> = HashSet::new();

    // loop through all the files supplied by the caller and process them one by one
    for file_path in files {
        // fetch the right muncher
        if let Some(muncher) = code_rules.get_muncher(file_path) {
            // process the file with the rules from the muncher
            if let Ok(tech) = processors::process_file(&file_path, muncher) {
                report.per_file_tech.insert(tech.clone());
                per_file_tech.push(file_path.clone());
                updated_tech.insert(tech.muncher_hash);
                report.merge_tech_record(tech);
            }
        }
    }

    // copy some parts from the old report to the new where no changes were made
    if let Some(old_report) = old_report {
        // copy unaffected tech records
        for tech in old_report.tech {
            if tech.muncher_hash > 0 && !updated_tech.contains(&tech.muncher_hash) {
                info!(
                    "Copied {}/{}/{} tech section from the old report",
                    tech.language, tech.muncher_name, tech.muncher_hash
                );
                report.merge_tech_record(tech);
            }
        }

        // copy per-file tech sections that are still present in the tree, but were not re-processed
        for tech in old_report.per_file_tech {
            if let Some(file_name) = &tech.file_name {
                if !per_file_tech.contains(file_name) && files.contains(file_name) {
                    info!("Copied {} file-tech section from the old report", file_name);
                    report.per_file_tech.insert(tech);
                }
            };
        }
    };

    info!("Analysis finished");
    Ok(report)
}

/// Returns the list of files containing only files with changed munchers.
pub fn filter_out_files_with_unchanged_munchers(
    code_rules: &mut code_rules::CodeRules,
    old_report: &report::Report,
    files: Vec<String>,
) -> Vec<String> {
    info!("Filtering out files with unchanged munchers");

    // collects hashes of munchers that should be ignored for this project because they have
    // not changed since the last processing of the repo
    let mut old_munchers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for tech in &old_report.tech {
        if tech.muncher_hash > 0 {
            old_munchers.insert(tech.muncher_hash);
        }
    }
    info!("Found {} muncher hashes in the report", old_munchers.len());

    // result collector
    let mut files_with_changed_munchers: Vec<String> = Vec::new();

    // loop through all the files supplied by the caller and process them one by one
    for file_path in files {
        // fetch the right muncher
        if let Some(muncher) = code_rules.get_muncher(&file_path) {
            // check if the file in the old report was processed by the same muncher and can be skipped
            if old_munchers.contains(&muncher.muncher_hash) {
                trace!("Unchanged muncher for {}", file_path);
                continue;
            }

            // the muncher was changed
            trace!("Retaining {}", file_path);
            files_with_changed_munchers.push(file_path);
        }
    }

    info!("Returning {} file names", files_with_changed_munchers.len());
    files_with_changed_munchers
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_all_tree_files(dir: &Path) -> Result<Vec<String>, ()> {
    let all_objects = Report::execute_git_command(
        vec![
            "ls-tree".into(),
            "-r".into(),
            "--full-tree".into(),
            "--name-only".into(),
            "HEAD".into(),
        ],
        &dir.to_string_lossy().to_string(),
    )
    .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let files = all_objects.lines().map(|v| v.to_owned()).collect::<Vec<String>>();
    info!("Objects in the GIT tree: {}", files.len());

    Ok(files)
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_last_commit_files(dir: &Path) -> Result<Vec<String>, ()> {
    let all_objects = Report::execute_git_command(
        vec![
            "log".into(),
            "--name-only".into(),
            "--oneline".into(),
            "--no-decorate".into(),
            "-1".into(),
        ],
        &dir.to_string_lossy().to_string(),
    )
    .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let files = all_objects
        .lines()
        .skip(1)
        .map(|v| v.to_owned())
        .collect::<Vec<String>>();
    info!("Objects in the last commit: {}", files.len());

    Ok(files)
}
