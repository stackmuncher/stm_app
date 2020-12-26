use report::Report;
use std::path::Path;
use tracing::{info, trace};

pub mod code_rules;
pub mod config;
pub mod file_type;
pub mod kwc;
pub mod muncher;
pub mod processors;
pub mod report;
pub mod tech;

pub async fn process_project(
    conf: &mut code_rules::CodeRules,
    project_dir: &String,
    user_name: &String,
    repo_name: &String,
    old_report: Option<report::Report>,
) -> Result<report::Report, ()> {
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

    // result collectors
    let mut processed_files: Vec<String> = Vec::new();
    let mut report = report::Report::new(user_name.clone(), repo_name.clone());
    let mut per_file_tech: Vec<String> = Vec::new();

    // get the list of files to process (all files in the tree)
    let files = get_file_names_recursively(Path::new(project_dir)).await?;

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
                report.per_file_tech.insert(tech.clone());
                per_file_tech.push(file_path.clone());
                report.merge_tech_record(tech);
            }
        }
    }

    // copy some parts from the old report, if any
    if let Some(old_report) = old_report {
        // copy tech reports for unchanged munchers from the old report, if any
        for tech in old_report.tech {
            if tech.muncher_hash > 0 && unchanged_munchers.contains(&tech.muncher_hash) {
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
    let mut files = files;
    files.retain(|f| !processed_files.contains(&f));

    // log unprocessed files in the report
    for f in &files {
        report.add_unprocessed_file(f);
    }

    report.tree_files = Some(files);

    info!("Analysis finished");
    Ok(report)
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
async fn get_file_names_recursively(dir: &Path) -> Result<Vec<String>, ()> {
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
