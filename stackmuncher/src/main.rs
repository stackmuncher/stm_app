use stackmuncher_lib::{
    code_rules::CodeRules, config::Config, git::get_local_identities, report::Report, utils::hash_str_sha1,
};
use std::path::Path;
use tracing::{debug, info, warn};

mod config;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // get input params
    let config = config::new_config();

    tracing_subscriber::fmt()
        .with_max_level(config.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("StackMuncher started in {}", config.project_dir_path);

    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = CodeRules::new(&config.code_rules_dir);

    // Reports are grouped per project with a canonical project name as the last subfolder
    let report_dir = Path::new(
        config
            .report_dir
            .as_ref()
            .expect("Cannot unwrap config.report_dir. it's a bug."),
    );
    warn!("Reports folder: {}", report_dir.to_string_lossy());

    // load a previously generated report if it exists
    let project_report_filename = report_dir
        .join([Config::PROJECT_REPORT_FILE_NAME, Config::REPORT_FILE_EXTENSION].concat())
        .as_os_str()
        .to_string_lossy()
        .to_string();
    let cached_project_report = Report::from_disk(&project_report_filename);

    let project_report = match Report::process_project(
        &mut code_rules,
        &config.project_dir_path,
        &cached_project_report,
        &config.git_remote_url_regex,
        None,
    )
    .await?
    {
        None => {
            // there were no changes since the previous report - it can be reused as-is
            info!("Done in {}ms", instant.elapsed().as_millis());
            return Ok(());
        }
        Some(v) => v,
    };

    project_report.save_as_local_file(&project_report_filename);
    info!("Project report done in {}ms", instant.elapsed().as_millis());

    // check if there are multiple contributors and generate individual reports
    if let Some(contributors) = &project_report.contributors {
        let last_commit_author = project_report.last_commit_author.as_ref().unwrap().clone();

        // get the list of user identities for processing their contributions individually
        let git_identities = get_local_identities(&config.project_dir_path).await?;
        if git_identities.is_empty() {
            warn!("No git identity found. Individual contributions will not be processed. Use `git config --global user.email you@example.com` before the next run.");
            eprintln!(
            "Git user details (name/email) are not set in gitconfig. Use `git config --global user.email you@example.com` before the next run."
        );
            return Err(());
        }

        // a container for the combined contributor report if there are multiple identities
        // we save all identities (for a single contributor) separate and then combine them into a single report
        let mut contributor_reports: Vec<(Report, String)> = Vec::new();

        for contributor in contributors {
            // only process known local identities
            if !git_identities.contains(&contributor.git_id.trim().to_lowercase()) {
                debug!("Contributor {} skipped / unknown identity", contributor.git_id);
                continue;
            }

            let contributor_instant = std::time::Instant::now();
            // load the previous contributor report, if any
            let contributor_hash = hash_str_sha1(contributor.git_id.as_str());
            let contributor_report_filename = report_dir
                .join(
                    [
                        Config::CONTRIBUTOR_REPORT_FILE_NAME,
                        contributor_hash.as_str(),
                        Config::REPORT_FILE_EXTENSION,
                    ]
                    .concat(),
                )
                .as_os_str()
                .to_string_lossy()
                .to_string();

            let cached_contributor_report = Report::from_disk(&contributor_report_filename);

            // only process a single contributor of the latest commit if it's a single commit report update
            if project_report.is_single_commit && contributor.git_id != last_commit_author {
                if let Some(cached_contributor_report) = cached_contributor_report {
                    debug!(
                        "Used cached report for contributor {} / single commit",
                        contributor.git_id
                    );
                    contributor_reports.push((cached_contributor_report, contributor.git_id.clone()));
                    continue;
                }
                debug!(
                    "Missing cached report for contributor {} / single commit",
                    contributor.git_id
                );
            }

            let contributor_report = project_report
                .process_contributor(
                    &mut code_rules,
                    &config.project_dir_path,
                    &cached_contributor_report,
                    contributor,
                    project_report.tree_files.as_ref(),
                )
                .await?;

            contributor_report.save_as_local_file(&contributor_report_filename);

            info!(
                "Contributor report for {} done in {}ms",
                contributor.git_id,
                contributor_instant.elapsed().as_millis()
            );

            // push the contributor report into a container to combine later
            contributor_reports.push((contributor_report, contributor.git_id.clone()));
        }

        // combine multiple contributor reports from different identities
        debug!("Combining {} contributor reports", contributor_reports.len());
        if !contributor_reports.is_empty() {
            let (mut combined_report, contributor_git_id) = contributor_reports.pop().unwrap();
            combined_report.reset_combined_contributor_report(contributor_git_id);
            for (contributor_report, contributor_git_id) in contributor_reports.into_iter() {
                // this only adds per-file-tech and does not affect any other part of the report
                combined_report.merge_contributor_reports(contributor_report, contributor_git_id)
            }

            // combine all added per-file-tech into appropriate tech records
            combined_report.recompute_tech_section();

            // save the combined report
            combined_report.save_as_local_file(
                &report_dir
                    .join(
                        [
                            Config::COMBINED_CONTRIBUTOR_REPORT_FILE_NAME,
                            Config::REPORT_FILE_EXTENSION,
                        ]
                        .concat(),
                    )
                    .as_os_str()
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    info!("Repo processed in {}ms", instant.elapsed().as_millis());
    Ok(())
}
