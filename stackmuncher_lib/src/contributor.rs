use super::git::GitLogEntry;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A GIT author or committer. E.g. `Author: rimutaka <max@onebro.me>` from `git log`.
/// It contains extended info like what was committed, when, contact details.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contributor {
    /// Email is the preferred ID, but it can be just the name if the email is missing, e.g. `max@onebro.me` for `Author: rimutaka <max@onebro.me>`
    ///
    /// A contributor with name, but no email should only match if the other record has no email either. It's easier to merge fragmented reports
    /// than separating wrong attribution.
    pub git_id: String,
    /// A list of possible identities as name/email pairs for extracting contact details and de-duplication.
    /// E.g. `Author: rimutaka <max@onebro.me> would be `rimutaka`/`max@onebro.me`.
    pub name_email_pairs: HashSet<(String, String)>,
    /// The full SHA1 of the very last commit by this contributor. This bit should be retained for matching repositories on STM server.
    pub last_commit_sha1: String,
    /// The timestamp as EPOCH of the very last commit by this contributor.
    pub last_commit_epoch: i64,
    /// The timestamp of the last commit by this contributor formatted as RFC-3339.
    pub last_commit_date: String,
    /// The list of files touched by this contributor as FileName/CommitSHA1 tuple
    pub touched_files: HashSet<ContributorFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub struct ContributorFile {
    /// The file name extracted from GIT, including the relative path, e.g. `myproject/src/main.rs`
    pub name: String,
    /// SHA1 of the very last commit that affected this file
    pub commit: String,
    /// Date and time of the commit as EPOCH
    pub date_epoch: i64,
    /// Date and time of the commit as a human readable ISO date
    pub date_iso: String,
}

impl std::hash::Hash for ContributorFile {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.name.as_bytes());
        state.finish();
    }
}

impl PartialEq for ContributorFile {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Contributor {
    /// De-dupes and normalizes the list of contributors from the provided commit history.
    ///
    /// The same contributor can come under different names, but there is often a link either via
    /// the name or email. E.g. rimutaka/max@onebro.me or maxv/max@onebro.me. They can be merged and de-duped
    /// to some extent, but the process is prone to errors. E.g. common user names such as `admin` or `ubuntu`
    /// can be pointing at completely different people.
    pub(crate) fn from_commit_history(commits: Vec<GitLogEntry>) -> Vec<Contributor> {
        // the output collector: a map of Contributors with the contributor git identity as the key
        // each contributor has a hashmap with file as the key and commit/date/timestamp tuple that gets converted into an Vec for touched_files property
        let mut contributors: HashMap<String, (Contributor, HashMap<String, (String, String, i64)>)> = HashMap::new();

        for commit in commits {
            // skip commits with no author details
            if commit.author_name_email.0.is_empty() && commit.author_name_email.1.is_empty() {
                continue;
            }

            // choose the preferred identity for this contributor
            let git_identity = Self::git_identity_from_name_email_pair(&commit.author_name_email);

            // check if the contributor is already in the output collector
            if let Some((contributor, touched_files)) = contributors.get_mut(&git_identity) {
                // this is a known contributor - merge with the existing one
                contributor
                    .name_email_pairs
                    .insert((commit.author_name_email.0, commit.author_name_email.1));

                // only the latest version of the file is of interest
                for file in commit.files {
                    if !touched_files.contains_key(&file) {
                        touched_files.insert(file, (commit.sha1.clone(), commit.date.clone(), commit.date_epoch));
                    }
                }
            } else {
                // it's a new contributor - add as-is

                // add the identities as name/email pairs
                let mut name_email_pairs: HashSet<(String, String)> = HashSet::new();
                name_email_pairs.insert((commit.author_name_email.0, commit.author_name_email.1));

                // collect the list of touched files with the commit SHA1
                let mut touched_files: HashMap<String, (String, String, i64)> = HashMap::new();
                for file in commit.files {
                    if !touched_files.contains_key(&file) {
                        touched_files.insert(file, (commit.sha1.clone(), commit.date.clone(), commit.date_epoch));
                    }
                }

                // init the contributor
                let contributor = Contributor {
                    git_id: git_identity.clone(),
                    name_email_pairs,
                    last_commit_sha1: commit.sha1,
                    last_commit_epoch: commit.date_epoch,
                    last_commit_date: commit.date,
                    touched_files: HashSet::new(),
                };

                contributors.insert(git_identity, (contributor, touched_files));
            }
        }

        // convert hashmap of file/sha1 into tuples, assign them to the contributors and return the entire collection as a Vec
        // this is done because hashmaps do not look nice in json
        let mut output_collector: Vec<Contributor> = Vec::new();
        for (_, (mut contributor, touched_files_map)) in contributors {
            // flatten the file list and assign to the contributor
            contributor.touched_files = touched_files_map
                .into_iter()
                .map(|(name, (sha1, date_iso, date_epoch))| ContributorFile {
                    name,
                    commit: sha1,
                    date_epoch,
                    date_iso,
                })
                .collect::<HashSet<ContributorFile>>();
            output_collector.push(contributor);
        }

        output_collector
    }

    /// Converts name email pairs, e.g. rimutaka|max@onebro.me into a git identity in a consistent way across the project
    pub(crate) fn git_identity_from_name_email_pair(author_name_email: &(String, String)) -> String {
        if !author_name_email.1.is_empty() {
            author_name_email.1.clone()
        } else {
            author_name_email.0.clone()
        }
    }
}
