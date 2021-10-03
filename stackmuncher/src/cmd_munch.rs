use crate::config::AppConfig;
use crate::help;
use crate::signing::ReportSignature;
use crate::submission::submit_report;
use futures::stream::{FuturesUnordered, StreamExt};
use stackmuncher_lib::contributor::Contributor;
use stackmuncher_lib::report_brief::TechOverview;
use stackmuncher_lib::{code_rules::CodeRules, config::Config, git, report::Report, utils::hash_str_sha1};
use std::path::Path;
use tracing::{debug, info, warn};

pub(crate) async fn run(config: AppConfig) -> Result<(), ()> {
    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = CodeRules::new();

    // Reports are grouped per project with a canonical project name as the last subfolder
    let report_dir = Path::new(
        config
            .lib_config
            .project_report_dir
            .as_ref()
            .expect("Cannot unwrap config.report_dir. It's a bug."),
    );
    warn!("Reports folder: {}", report_dir.to_string_lossy());

    // load a previously generated report if it exists
    let project_report_filename =
        report_dir.join([Config::PROJECT_REPORT_FILE_NAME, Config::REPORT_FILE_EXTENSION].concat());
    let cached_project_report = Report::from_disk(&project_report_filename);

    // get and retain a copy of the full git lot to re-use in multiple places
    let git_log = git::get_log(&config.lib_config.project_dir, None, &code_rules.ignore_paths).await?;

    let project_report = match Report::process_project(
        &mut code_rules,
        &config.lib_config.project_dir,
        &cached_project_report,
        Some(git_log.clone()),
    )
    .await?
    {
        None => {
            // there were no changes since the previous report - it can be reused as-is
            info!("Done in {}ms", instant.elapsed().as_millis());
            println!("    No new commits since the last run.");
            cached_project_report.expect("Cannot unwrap cached report. It's a bug.")
        }
        Some(v) => {
            let _ = v.save_as_local_file(&project_report_filename, true);
            info!("Project stack analyzed in {}ms", instant.elapsed().as_millis());
            v
        }
    };

    info!("Contributor reports requested for: {:?}", config.lib_config.git_identities);

    // check if there are multiple contributors and generate individual reports
    if let Some(contributors) = &project_report.contributors {
        let last_commit_author = project_report.last_commit_author.as_ref().unwrap().clone();

        // prepare a combined list of commit IDs from all known identities
        let list_of_commits = git::get_contributor_commits_from_log(&git_log, &config.lib_config.git_identities);

        // prepare a container for async submission jobs
        let mut submission_jobs = FuturesUnordered::new();

        // a container for the combined contributor report if there are multiple identities
        // we save all identities (for a single contributor) separate and then combine them into a single report
        let mut contributor_reports: Vec<(Report, String)> = Vec::new();

        for contributor in contributors {
            // only process known local identities
            if !config
                .lib_config
                .git_identities
                .contains(&contributor.git_id.trim().to_lowercase())
            {
                debug!("Contributor {} skipped / unknown identity", contributor.git_id);
                continue;
            }

            let contributor_instant = std::time::Instant::now();
            // load the previous contributor report, if any
            let contributor_hash = hash_str_sha1(contributor.git_id.as_str());
            let contributor_report_filename = report_dir.join(
                [
                    Config::CONTRIBUTOR_REPORT_FILE_NAME,
                    contributor_hash.as_str(),
                    Config::REPORT_FILE_EXTENSION,
                ]
                .concat(),
            );

            let cached_contributor_report = Report::from_disk(&contributor_report_filename);

            // if this is a single-commit update then use cached reports for all contributors other than the author of the commit
            if project_report.is_single_commit && contributor.git_id != last_commit_author {
                if let Some(cached_contributor_report) = cached_contributor_report {
                    debug!("Used cached report for contributor {} / single commit", contributor.git_id);
                    contributor_reports.push((cached_contributor_report, contributor.git_id.clone()));
                    continue;
                }
                debug!("Missing cached report for contributor {} / single commit", contributor.git_id);
            }

            let contributor_report = project_report
                .process_contributor(
                    &mut code_rules,
                    &config.lib_config.project_dir,
                    &cached_contributor_report,
                    contributor,
                    project_report.tree_files.as_ref(),
                )
                .await?;

            contributor_report.save_as_local_file(&contributor_report_filename, false);

            info!(
                "Contributor stack for {} analyzed in {}ms",
                contributor.git_id,
                contributor_instant.elapsed().as_millis()
            );

            // push the contributor report into a container to combine later
            contributor_reports.push((contributor_report, contributor.git_id.clone()));
        }

        // combine multiple contributor reports from different identities
        debug!("Combining {} contributor reports", contributor_reports.len());
        if contributor_reports.is_empty() {
            // there were no matching contributors
            print_no_contributions_msg(&config.lib_config.git_identities, contributors);
        } else {
            // seed the combined report from the 1st contributor report in the list of all contributor reports
            let (mut combined_report, contributor_git_id) = contributor_reports.pop().unwrap();
            combined_report.reset_combined_contributor_report(contributor_git_id, &list_of_commits, &project_report);
            for (contributor_report, contributor_git_id) in contributor_reports.into_iter() {
                // this only adds per-file-tech and does not affect any other part of the report
                combined_report.merge_same_project_contributor_reports(contributor_report, contributor_git_id);
            }

            // combine all added per-file-tech into appropriate tech records
            combined_report.recompute_tech_section();

            // add any personal details supplied via CLI or taken from the environment
            combined_report.primary_email = config.primary_email.clone();
            combined_report.gh_validation_id = config.gh_validation_id.clone();

            // check if there is a already a cached contributor report
            // it would have to be a dry run (no submission) if it's the first time STM is run on this repo
            let combined_report_file_name = report_dir.join(
                [
                    Config::CONTRIBUTOR_REPORT_COMBINED_FILE_NAME,
                    Config::REPORT_FILE_EXTENSION,
                ]
                .concat(),
            );
            let first_run = !combined_report_file_name.exists();

            // save the combine report for inspection by the user
            combined_report.save_as_local_file(&combined_report_file_name, true);

            // produce a sanitized version of the combined report, save and submit it if needed
            if let Ok(combined_report) = combined_report.sanitize(ReportSignature::get_salt(&config.user_key_pair)) {
                // prepare the file name of the sanitized report
                let sanitized_report_file_name = &report_dir.join(
                    [
                        Config::CONTRIBUTOR_REPORT_SANITIZED_FILE_NAME,
                        Config::REPORT_FILE_EXTENSION,
                    ]
                    .concat(),
                );

                // save the sanitized report
                combined_report.save_as_local_file(sanitized_report_file_name, true);

                print_combined_stats(&combined_report);

                // check if the submission to the directory should go ahead
                if config.dryrun {
                    // a dry-run was requested by the user
                    println!("    Profile update:      skipped with `--dryrun` flag");
                } else {
                    if first_run {
                        info!("No report submission on the first run");
                        help::emit_dryrun_msg(&sanitized_report_file_name.to_string_lossy());
                    } else {
                        submission_jobs.push(submit_report(combined_report.clone(), &config));
                    }
                }
            }
        }

        // there should be only a single submission of the combined report
        match submission_jobs.next().await {
            Some(_) => {
                debug!("Combined contributor report submitted");
            }
            None => {
                debug!("No combined contributor report was submitted");
            }
        }
    }

    // print the location of the reports
    println!("    Stack reports:       {}", report_dir.to_string_lossy());
    info!("Repo processed in {}ms", instant.elapsed().as_millis());

    Ok(())
}

/// Prints a one-line summary of the report for the user to get an idea and not need to look up the report file
/// E.g. `Summary (LoC/libs):  Rust 12656/26, Markdown 587, PowerShell 169`
fn print_combined_stats(report: &Report) {
    let report = report.get_overview();

    // get a summary and sort the stack by LoC
    let mut tech = report.tech.iter().collect::<Vec<&TechOverview>>();
    tech.sort_unstable_by(|a, b| b.loc.cmp(&a.loc));

    // prepare a single line of per-tech stats
    let per_tech_stats = tech
        .iter()
        .map(|t| {
            // only include libs count if there are any
            let libs = if t.libs > 0 {
                ["/", t.libs.to_string().as_str()].concat()
            } else {
                String::new()
            };

            [t.language.as_str(), " ", t.loc.to_string().as_str(), libs.as_str()].concat()
        })
        .collect::<Vec<String>>();
    let per_tech_stats = per_tech_stats.as_slice().join(", ");
    println!("    Summary (LoC/libs):  {}", per_tech_stats);
}

/// Prints a list of contributors and git identities to help find user git identities
fn print_no_contributions_msg(git_identities: &Vec<String>, contributors: &Vec<Contributor>) {
    // is this repo empty?
    if contributors.is_empty() {
        println!("    This repository has no commits with identifiable committers.");
        return;
    }

    match git_identities.len() {
        0 => {
            println!();
            println!("    No commits were selected for analysis.");
            println!("    Configure `user.email` Git setting or use `--email` CLI params to add committer emails.");
            println!();
        }
        1 => {
            println!();
            println!(
                "Found no commits from {}. Did you make commits with a different email?",
                git_identities[0]
            );
            println!("    Run `git shortlog -s -e --all` to see all committer emails in this repo.");
            println!("    Add more of your committer emails with `stackmuncher config --emails \"me1@gmail.com,me2@gmail.com\"");
            println!();
        }
        _ => {
            println!();
            println!("    Found no commits from any of: {}.", git_identities.join(", "));
            println!("    Run `git shortlog -s -e --all` to see all committer emails in this repo.");
            println!("    Add more of your committer emails with `stackmuncher config --emails \"me1@gmail.com,me2@gmail.com\"");
            println!();
        }
    }
}
