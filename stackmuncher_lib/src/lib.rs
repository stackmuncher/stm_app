use chrono::TimeZone;
use contributor::Contributor;
use git::{log_entries_to_list_of_blobs, GitBlob, GitLogEntry, ListOfBlobs};
use report::Report;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info, trace, warn};

pub mod code_rules;
pub mod config;
pub mod contributor;
pub mod file_type;
pub mod git;
mod ignore_paths;
pub mod muncher;
pub mod processors;
pub mod report;
pub mod utils;

impl Report {
    /// Processes the entire repo with or without a previous report. If the report is present and the munchers
    /// have not changed the relevant sections are copied from the old report. Use this function when:
    /// * it's a new repo
    /// * the munchers changed and the entire repo needs to be reprocessed
    /// * `git_log` must contain the entire log for the project or the function will get the log as needed if None
    /// ## Return values
    /// * `Err` - something went wrong, error details logged
    /// * `None` - no changes, use the cached report
    /// * `Some` - an updated report
    pub async fn process_project(
        code_rules: &mut code_rules::CodeRules,
        project_dir: &Path,
        old_report: &Option<report::Report>,
        git_log: Option<Vec<GitLogEntry>>,
    ) -> Result<Option<report::Report>, ()> {
        let report = report::Report::new();

        // get the full git log if none was supplied
        let git_log = match git_log {
            Some(v) => v,
            None => git::get_log(project_dir, None, &code_rules.ignore_paths).await?,
        };

        // get the list of files in the tree at HEAD
        let all_head_files = git::get_all_tree_files(project_dir, None, &code_rules.ignore_paths).await?;
        if all_head_files.len() > Report::MAX_FILES_PER_REPO {
            warn!("Repo ignored. Too many files: {}", all_head_files.len());
            return Err(());
        }

        // get the list of all files that ever existed in the repo, including renamed and deleted
        let all_project_blobs = log_entries_to_list_of_blobs(&git_log);
        debug!(
            "Found {} files in log and {} in the current tree",
            all_project_blobs.len(),
            all_head_files.len()
        );
        // filter out blobs that are no longer in the HEAD tree
        let all_project_blobs = all_project_blobs
            .into_iter()
            .filter_map(|(file_name, blob)| {
                if all_head_files.contains(&file_name) {
                    Some((file_name, blob))
                } else {
                    None
                }
            })
            .collect::<ListOfBlobs>();

        let report = report.set_single_commit_flag(&git_log, &old_report);
        let report = report.add_commits_history(git_log).await;

        // check if there were any contents or muncher changes since the last commit
        // this is the cheapest check we can do to determine if there were an changes that need to be reprocessed
        if !report.has_content_or_muncher_changes(code_rules, &old_report, &all_project_blobs) {
            return Ok(None);
        }

        // copy unchanged tech records from the old report, if any and get the list of files that dont need to be munched
        let (report, reused_per_file_tech) =
            report.copy_cached_data_from_another_report(code_rules, old_report.as_ref(), &all_project_blobs);

        // select blobs that could not be copied from the old report and need to be munched because either the blob or the muncher changed
        let blobs_to_munch = all_project_blobs
            .into_iter()
            .filter_map(|(file_name, blob)| {
                if !reused_per_file_tech.contains(&file_name) {
                    Some((file_name, blob))
                } else {
                    None
                }
            })
            .collect::<ListOfBlobs>();
        debug!("Blobs that could not be copied from cache: {}", blobs_to_munch.len());

        // remove blobs that have no munchers or should be ignored - there is no point even retrieving the contents
        let blobs_to_munch = blobs_to_munch
            .into_iter()
            .filter_map(|(file_path, blob)| {
                if code_rules.get_muncher(&file_path).is_some() {
                    Some((file_path, blob))
                } else {
                    None
                }
            })
            .collect::<ListOfBlobs>();
        debug!("Blobs to munch with matching munchers: {}", blobs_to_munch.len());

        // group contributor files by commit to get the blob IDs with min number of git requests later

        // populate blob sha1 from head commit for blobs that need to be munched
        let blobs_to_munch = git::populate_blob_sha1(project_dir, blobs_to_munch, None).await?;

        // generate the report
        let report = report
            .process_project_files(code_rules, project_dir, &blobs_to_munch, Some(&all_head_files))
            .await?;

        // update lists of files (unprocessed and project tree)
        let report = report.update_project_file_lists(all_head_files);

        // add various metadata based on the final report
        let report = report.with_summary();

        Ok(Some(report))
    }

    /// Processes specified files from the repo and returns a report with Tech and Tech per file sections.
    /// * `project_dir` - needed for git
    /// * `blobs_to_process` - list of blobs that need to be processed, must have SHA1 set
    pub(crate) async fn process_project_files(
        self,
        code_rules: &mut code_rules::CodeRules,
        project_dir: &Path,
        blobs_to_process: &ListOfBlobs,
        all_tree_files: Option<&HashSet<String>>,
    ) -> Result<report::Report, ()> {
        info!("Processing individual project files from {}", project_dir.to_string_lossy());

        // result collectors
        let mut report = self;

        // loop through all the files supplied by the caller and process them one by one
        for (file_name, blob) in blobs_to_process {
            debug!("Blob {}/{}", file_name, blob.sha1);
            // fetch the right muncher
            if let Some(muncher) = code_rules.get_muncher(file_name) {
                // process the file with the rules from the muncher
                if let Ok(tech) = processors::process_file(
                    file_name,
                    &blob.sha1,
                    muncher,
                    project_dir,
                    &blob.commit_sha1,
                    blob.commit_date_epoch,
                    &blob.commit_date_iso,
                    all_tree_files,
                )
                .await
                {
                    report.per_file_tech.insert(tech.clone());
                    report.merge_tech_record(tech.reset_file_and_commit_info());
                }
            }
        }

        info!("Analysis finished");
        Ok(report)
    }

    /// Copies per-file tech sections for `blobs_to_process` that can be taken from the cached report without reprocessing.
    /// The records must match on the file name, commit SHA1 and muncher hash with the latest muncher from the rules collection.
    /// Returns an updated report and a list of cached files added to it.
    fn copy_cached_data_from_another_report(
        self,
        code_rules: &mut code_rules::CodeRules,
        other_report: Option<&report::Report>,
        all_project_blobs: &ListOfBlobs,
    ) -> (Self, HashSet<String>) {
        debug!("Copying cached data from another report");

        // exit if no other report was provided
        if other_report.is_none() {
            debug!("No cached report found");
            return (self, HashSet::new());
        }
        let other_report = other_report.unwrap();

        // result collector
        let mut report = self;

        // prepare a list of file names already in the target report, just copied or already existed or should not be copied
        let mut copied_per_file_techs: HashSet<String> = report
            .per_file_tech
            .iter()
            .filter_map(|tech| tech.file_name.clone())
            .collect();

        // loop thru all the cached per-file techs
        for tech in &other_report.per_file_tech {
            // unwrap the file name - there should always be one
            if let Some(file_tech_file_name) = tech.file_name.clone() {
                // skip the file if it's already present in the target report
                if copied_per_file_techs.contains(&file_tech_file_name) {
                    continue;
                }
                // check if there is a corresponding blob for that file name
                if let Some(blob) = all_project_blobs.get(&file_tech_file_name) {
                    // unwrap the commit SHA1 - there should always be one
                    if let Some(file_tech_commit_sha1) = tech.commit_sha1.clone() {
                        // if the commit SHA1s match ...
                        if blob.commit_sha1 == *file_tech_commit_sha1 {
                            // ... and the muncher hash matches the one in per_file_tech copy the record over
                            if let Some(muncher) = code_rules.get_muncher(&file_tech_file_name) {
                                if muncher.muncher_hash == tech.muncher_hash {
                                    debug!("Copied {} file-tech section from cached data", file_tech_file_name);
                                    // copy the record
                                    report.per_file_tech.insert(tech.clone());
                                    // merge it at the tech level
                                    report.merge_tech_record(tech.clone());
                                    // store the file name, so we know what was copied
                                    copied_per_file_techs.insert(file_tech_file_name.clone());
                                }
                            };
                        }
                    };
                };
            };
        }
        debug!("Copied {} per-file tech sections", copied_per_file_techs.len());
        (report, copied_per_file_techs)
    }

    /// Process only files touched by the contributor at the point of the contribution.
    pub async fn process_contributor(
        &self,
        code_rules: &mut code_rules::CodeRules,
        project_dir: &Path,
        old_contributor_report: &Option<report::Report>,
        contributor: &Contributor,
        all_tree_files: Option<&HashSet<String>>,
    ) -> Result<report::Report, ()> {
        debug!("Processing contributor: {}", contributor.git_id);

        let project_report = self;

        // collect all contributor blobs from the project report
        let mut last_contributor_commit_sha1 = String::new();
        let mut last_contributor_commit_date_epoch = 0i64;
        let mut last_contributor_commit_date_iso: Option<String> = None;
        let contributor_blobs = &contributor
            .touched_files
            .iter()
            .map(|file| {
                // find the latest commit SHA1 and date for this contributor while it's iterating through them all anyway
                if file.date_epoch > last_contributor_commit_date_epoch {
                    last_contributor_commit_sha1 = file.commit.clone();
                    last_contributor_commit_date_epoch = file.date_epoch;
                    last_contributor_commit_date_iso = Some(file.date_iso.clone());
                }
                (
                    file.name.clone(),
                    GitBlob {
                        sha1: String::new(),
                        commit_sha1: file.commit.clone(),
                        commit_date_epoch: file.date_epoch,
                        commit_date_iso: file.date_iso.clone(),
                    },
                )
            })
            .collect::<ListOfBlobs>();

        let report = report::Report::new();
        // copy cached data processed earlier
        // first from the old contributor report
        let (report, reused_per_file_tech_contributor) = report.copy_cached_data_from_another_report(
            code_rules,
            old_contributor_report.as_ref(),
            &contributor_blobs,
        );
        // then from the project report
        let (report, reused_per_file_tech_project) =
            report.copy_cached_data_from_another_report(code_rules, Some(project_report), &contributor_blobs);

        // get the list of contributor blobs that could not be copied and have to be processed
        let blobs_to_munch = contributor_blobs
            .iter()
            .filter_map(|(file_name, blob)| {
                if reused_per_file_tech_contributor.contains(file_name)
                    || reused_per_file_tech_project.contains(file_name)
                {
                    None
                } else {
                    Some((file_name.clone(), blob.clone()))
                }
            })
            .collect::<ListOfBlobs>();

        debug!(
            "Blobs to munch: {}, reused_per_file_tech_contributor: {}, reused_per_file_tech_project: {}",
            blobs_to_munch.len(),
            reused_per_file_tech_contributor.len(),
            reused_per_file_tech_project.len()
        );

        // remove blobs that have no munchers - there is no point in getting the contents
        let blobs_to_munch = blobs_to_munch
            .into_iter()
            .filter_map(|(file_name, blob)| {
                if code_rules.get_muncher(&file_name).is_some() {
                    Some((file_name, blob))
                } else {
                    None
                }
            })
            .collect::<ListOfBlobs>();
        debug!("Blobs to munch with matching munchers: {}", blobs_to_munch.len());

        // group contributor files by commit to get the blob IDs with min number of git requests later
        // blobs_to_munch -> blobs_by_commit
        let mut blobs_by_commit: HashMap<String, ListOfBlobs> = HashMap::new();
        for (file_name, blob) in blobs_to_munch {
            // separate file names by commit
            if let Some(blob_list) = blobs_by_commit.get_mut(&blob.commit_sha1) {
                // add the file name to the commit record in files_by_commit
                trace!("Inserting blob {} for existing commit {}", file_name, blob.commit_sha1);
                blob_list.insert(file_name, blob);
            } else {
                // create a new commit record in files_by_commit
                trace!("Inserting blob {} for new commit {}", file_name, blob.commit_sha1);
                let blob_commit_sha1 = blob.commit_sha1.clone();
                let mut commit_blobs: ListOfBlobs = ListOfBlobs::new();
                commit_blobs.insert(file_name, blob);
                blobs_by_commit.insert(blob_commit_sha1, commit_blobs);
            }
        }
        debug!("Found {} contributor commits for looking up blob SHA1s", blobs_by_commit.len());

        // loop through the commits and update blob SHA1s for commit-associated files
        let mut blobs_to_munch = ListOfBlobs::new();
        for (commit_sha1, commit_blobs) in blobs_by_commit {
            // populate blob sha1 from head commit for blobs that need to be munched
            let commit_blobs = git::populate_blob_sha1(project_dir, commit_blobs, Some(commit_sha1.clone())).await?;
            for (file_name, blob) in commit_blobs {
                if !blob.sha1.is_empty() {
                    // store the entire list of blobs for analyzing them later
                    blobs_to_munch.insert(file_name, blob);
                }
            }
        }

        debug!(
            "Contributor files: {}, blobs to munch: {}",
            contributor.touched_files.len(),
            blobs_to_munch.len(),
        );

        // generate the report
        let mut report = report
            .process_project_files(code_rules, project_dir, &blobs_to_munch, all_tree_files)
            .await?;

        // copy all contributor commits from the list of project commits by commit idx
        if let Some(project_commits) = &project_report.recent_project_commits {
            let contributor_commits = contributor
                .commits
                .iter()
                .filter_map(|idx| {
                    if project_commits.len() > *idx {
                        Some(project_commits[*idx].clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>();

            // add meta for the first commit
            if let Some(first_commit) = contributor_commits.iter().last() {
                if let Some((sha1, ts)) = first_commit.split_once("_") {
                    if let Ok(ts_num) = i64::from_str_radix(ts, 10) {
                        let ts_dt = chrono::Utc.timestamp(ts_num, 0);

                        report.first_contributor_commit_date_epoch = Some(ts_num);
                        report.first_contributor_commit_date_iso = Some(ts_dt.to_rfc3339());
                        report.first_contributor_commit_sha1 = Some(sha1.to_string());
                    }
                }
            }

            report.recent_project_commits = Some(contributor_commits);
        } else {
            warn!("No project commits to copy to contributor");
        }

        // check if the contributor commits meta was set correctly
        if report.first_contributor_commit_sha1.is_none() {
            warn!("Missing first contributor commit info");
        }

        // copy some meta from the project report
        report.report_commit_sha1 = project_report.report_commit_sha1.clone();
        report.log_hash = project_report.log_hash.clone();
        report.is_single_commit = project_report.is_single_commit;
        report.last_commit_author = project_report.last_commit_author.clone();
        report.git_ids_included.insert(contributor.git_id.clone());
        report.contributor_count = project_report.contributor_count.clone();
        report.last_contributor_commit_sha1 = Some(last_contributor_commit_sha1);
        report.last_contributor_commit_date_iso = last_contributor_commit_date_iso;
        report.last_contributor_commit_date_epoch = Some(last_contributor_commit_date_epoch);
        report.loc_project = project_report.loc_project.clone();
        report.libs_project = project_report.libs_project.clone();
        report.commit_count_project = project_report.commit_count_project.clone();
        report.commit_count_contributor = Some(contributor.commit_count.clone());

        Ok(report)
    }

    /// Returns `true` if either content (blobs) or relevant munchers (their hashes) have changed since the old report
    /// was generated. Otherwise returns true.
    fn has_content_or_muncher_changes(
        &self,
        code_rules: &mut code_rules::CodeRules,
        old_report: &Option<report::Report>,
        files: &ListOfBlobs,
    ) -> bool {
        debug!("Checking for content or muncher changes");

        // check for contents changes
        if old_report.is_none() {
            debug!("No cached report found");
            return true;
        }
        let old_report = old_report.as_ref().unwrap();

        // check if the report is in an older format and has to be reprocessed regardless
        if old_report.is_outdated_format() {
            warn!("Full reprocessing due to new report format: {}", Report::REPORT_FORMAT_VERSION);
            return true;
        };

        let report_commit_sha1 = self.report_commit_sha1.clone().unwrap_or_default();
        let old_report_commit_sha1 = old_report.report_commit_sha1.clone().unwrap_or_default();

        if report_commit_sha1.is_empty() || report_commit_sha1 != old_report_commit_sha1 {
            debug!(
                "Current report commit sha1: {}, cached: {}, mismatch",
                report_commit_sha1, old_report_commit_sha1
            );
            return true;
        }

        // collects hashes of munchers that should be ignored for this project because they have
        // not changed since the last processing of the repo
        let mut old_munchers: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for tech in &old_report.tech {
            if tech.muncher_hash > 0 {
                old_munchers.insert(tech.muncher_hash);
            }
        }
        debug!("Found {} muncher hashes in the old report", old_munchers.len());

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
                debug!("Found a changed muncher for blob {}", blob.0);
                return true;
            }
        }

        info!("No changes in content or munchers. Will re-use the cached report as-is.");
        false
    }

    /// Sets `is_single_commit` flag to `true` if there was only a single-commit change between the old and the current repos.
    /// It will be set to false in case of merge, rebase or any other history re-write. This function looks at commit SHA1s and
    /// ignores commit messages, dates or any other info.
    pub(crate) fn set_single_commit_flag(
        self,
        git_log: &Vec<GitLogEntry>,
        old_report: &Option<report::Report>,
    ) -> Self {
        let mut report = self;
        report.is_single_commit = false;

        // pre-requisites
        if old_report.is_none() || git_log.len() < 2 {
            debug!(
                "set_single_commit_flag -> false, commits: {}, cached report: {}",
                git_log.len(),
                old_report.is_some()
            );
            return report;
        }

        // compare the SHA1s of the 2nd commit and the old report
        let old_report_sha1 = old_report
            .as_ref()
            .unwrap()
            .report_commit_sha1
            .clone()
            .unwrap_or_default();
        let old_report_log_hash = old_report.as_ref().unwrap().log_hash.clone().unwrap_or_default();

        // heck if there are any history rewrites in the order of complexity check
        if !old_report_sha1.is_empty()
            && !old_report_log_hash.is_empty()
            && old_report_sha1 == git_log[1].sha1
            && old_report_log_hash
                == utils::hash_vec_sha1(
                    git_log
                        .iter()
                        .skip(1)
                        .map(|entry| entry.sha1.clone())
                        .collect::<Vec<String>>(),
                )
        {
            debug!("set_single_commit_flag -> true, commits: {}", git_log.len());
            report.is_single_commit = true;
        }

        report
    }
}
