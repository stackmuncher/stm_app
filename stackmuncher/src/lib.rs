use contributor::Contributor;
use git::ListOfBlobs;
use regex::Regex;
use report::Report;
use std::collections::HashMap;
use tracing::{debug, info};

pub mod code_rules;
pub mod config;
pub mod contributor;
pub mod file_type;
pub mod git;
pub mod kwc;
pub mod muncher;
pub mod processors;
pub mod report;
pub mod tech;
pub mod utils;

impl Report {
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
        git_remote_url_regex: &Regex,
    ) -> Result<report::Report, ()> {
        // all files to be processed
        let all_files = git::get_all_tree_files(project_dir, None).await?;

        let files_with_changed_munchers = match old_report.as_ref() {
            Some(v) => filter_out_files_with_unchanged_munchers(code_rules, v, all_files.clone()),
            None => all_files.clone(),
        };

        // just return the old report if there were no changes and the old report can be re-used
        if old_report.is_some() && files_with_changed_munchers.is_empty() {
            return Ok(old_report.unwrap());
        }

        // generate the report
        let report = Report::process_project_files(
            code_rules,
            project_dir,
            user_name,
            repo_name,
            old_report,
            &files_with_changed_munchers,
            &all_files,
        )
        .await?;

        // update the report with additional info
        let report = report.extract_commit_history(project_dir, git_remote_url_regex).await;
        let report = report.update_list_of_tree_files(all_files);

        Ok(report)
    }

    /// Processes specified files from the repo and returns a report with Tech and Tech per file sections.
    pub async fn process_project_files(
        code_rules: &mut code_rules::CodeRules,
        project_dir: &String,
        user_name: &String,
        repo_name: &String,
        old_report: Option<report::Report>,
        files_to_process: &ListOfBlobs,
        all_tree_files: &ListOfBlobs,
    ) -> Result<report::Report, ()> {
        info!("Analyzing code from {}", project_dir);

        // result collectors
        let mut report = report::Report::new(user_name.clone(), repo_name.clone());
        let mut per_file_tech: Vec<String> = Vec::new();

        // loop through all the files supplied by the caller and process them one by one
        for blob in files_to_process {
            debug!("Blob {}/{}", blob.0, blob.1);
            // fetch the right muncher
            if let Some(muncher) = code_rules.get_muncher(blob.0) {
                // process the file with the rules from the muncher
                if let Ok(tech) = processors::process_file(blob.0, blob.1, muncher, project_dir).await {
                    report.per_file_tech.insert(tech.clone());
                    per_file_tech.push(blob.0.clone());
                    report.merge_tech_record(tech);
                }
            }
        }

        // copy all per-file tech sections that are still present in the tree, but were not re-processed
        if let Some(old_report) = old_report {
            for tech in old_report.per_file_tech {
                if let Some(file_name) = &tech.file_name {
                    if !per_file_tech.contains(file_name) && all_tree_files.contains_key(file_name) {
                        info!("Copied {} file-tech section from the old report", file_name);
                        report.per_file_tech.insert(tech.clone());
                        report.merge_tech_record(tech);
                    }
                };
            }
        };

        info!("Analysis finished");
        Ok(report)
    }

    /// Process only files touched by the contributor at the point of the contribution.
    pub async fn process_contributor(
        &self,
        code_rules: &mut code_rules::CodeRules,
        project_dir: &String,
        user_name: &String,
        old_report: Option<report::Report>,
        contributor: &Contributor,
    ) -> Result<report::Report, ()> {
        debug!("Processing contributor: {}", user_name);
        // files touched by the contributor with corresponding commit SHA1
        let mut touched_files: ListOfBlobs = ListOfBlobs::new();

        // arrange the files by commit
        let mut files_by_commit: HashMap<String, Vec<String>> = HashMap::new();
        for file in &contributor.touched_files {
            if let Some(file_list) = files_by_commit.get_mut(&file.commit) {
                file_list.push(file.name.clone());
            } else {
                files_by_commit.insert(file.commit.clone(), vec![file.name.clone()]);
            }
        }
        debug!(
            "Found {} contributor commits for looking up blobs",
            files_by_commit.len()
        );

        // request blobs from particular commits
        for (commit_sha1, commit_files) in files_by_commit {
            let commit_tree = git::get_all_tree_files(project_dir, Some(commit_sha1.clone())).await?;
            debug!("Commit {} has {} touched files", commit_sha1, commit_files.len());
            for commit_file in commit_files {
                if let Some(blob_sha1) = commit_tree.get(&commit_file) {
                    touched_files.insert(commit_file, blob_sha1.clone());
                } else {
                    // this normally happens when a file was deleted from the tree
                    // we may see it from the diff, but there is no point trying to look it up - if it's missing, it's missing
                    // it would be good to exclude them from the list of contributor files in the first place, but it would require an additional
                    // look up, which is expensive
                    // deleting a file is a contribution
                    debug!("Cannot find blob SHA1 for {} in commit {}", commit_file, commit_sha1);
                }
            }
        }

        // only files touched by the contributor where munchers changed need to be processed
        let files_to_process = match old_report.as_ref() {
            Some(v) => filter_out_files_with_unchanged_munchers(code_rules, v, touched_files.clone()),
            None => touched_files.clone(),
        };
        debug!(
            "Contributor files: {}, blobs found: {}, to process {}",
            contributor.touched_files.len(),
            touched_files.len(),
            files_to_process.len(),
        );

        // just return the old report if there were no changes and the old report can be re-used
        if old_report.is_some() && files_to_process.is_empty() {
            debug!("No changes. Reusing old report.");
            return Ok(old_report.unwrap());
        }

        // generate the report
        let report = Report::process_project_files(
            code_rules,
            project_dir,
            user_name,
            &self.repo_name,
            old_report,
            &files_to_process,
            &touched_files,
        )
        .await?;

        // update the report with additional info
        let report = report.update_list_of_tree_files(touched_files);

        Ok(report)
    }
}
/// Returns the list of files (blobs) containing only files with changed munchers.
fn filter_out_files_with_unchanged_munchers(
    code_rules: &mut code_rules::CodeRules,
    old_report: &report::Report,
    files: ListOfBlobs,
) -> ListOfBlobs {
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
    let mut files_with_changed_munchers: ListOfBlobs = ListOfBlobs::new();

    // loop through all the files supplied by the caller and process them one by one
    for blob in files {
        // fetch the right muncher
        if let Some(muncher) = code_rules.get_muncher(&blob.0) {
            // check if the file in the old report was processed by the same muncher and can be skipped
            if old_munchers.contains(&muncher.muncher_hash) {
                debug!("Unchanged muncher for {}", blob.0);
                continue;
            }

            // the muncher was changed
            debug!("Retaining {}", blob.0);
            files_with_changed_munchers.insert(blob.0, blob.1);
        }
    }

    info!("Returning {} file names", files_with_changed_munchers.len());
    files_with_changed_munchers
}
