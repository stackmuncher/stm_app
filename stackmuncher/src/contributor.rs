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
    pub git_identity: String,
    /// A list of possible identities as name/email pairs for extracting contact details and de-duplication.
    /// E.g. `Author: rimutaka <max@onebro.me> would be `rimutaka`/`max@onebro.me`.
    pub name_email_pairs: HashSet<(String, String)>,
    /// The full SHA1 of the very last commit by this contributor. This bit should be retained for matching repositories on STM server.
    pub last_commit_sha1: String,
    /// The timestamp as EPOCH of the very last commit by this contributor.
    pub last_commit_epoch: i64,
    /// The timestamp of the last commit by this contributor formatted as RFC-3339.
    pub last_commit_date: String,
    /// The list of files touched by this contributor
    pub touched_files: HashSet<String>,
}

impl Contributor {
    /// De-dupes and normalizes the list of contributors from the provided commit history.
    ///
    /// The same contributor can come under different names, but there is often a link either via
    /// the name or email. E.g. rimutaka/max@onebro.me or maxv/max@onebro.me. They can be merged and de-duped
    /// to some extent, but the process is prone to errors. E.g. common user names such as `admin` or `ubuntu`
    /// can be pointing at completely different people.
    pub fn from_commit_history(commits: Vec<GitLogEntry>) -> Vec<Contributor> {
        // the output collector
        let mut contributors: HashMap<String, Contributor> = HashMap::new();

        for commit in commits {
            // skip commits with no author details
            if commit.author_name_email.0.is_empty() && commit.author_name_email.1.is_empty() {
                continue;
            }

            // choose the preferred identity for this contributor
            let git_identity = if commit.author_name_email.1.is_empty() {
                commit.author_name_email.0.clone()
            } else {
                commit.author_name_email.1.clone()
            };

            // check if it's a known contributor
            if let Some(contributor) = contributors.get_mut(&git_identity) {
                // this is a known contributor - merge with the existing one
                contributor
                    .name_email_pairs
                    .insert((commit.author_name_email.0, commit.author_name_email.1));

                for file in commit.files {
                    contributor.touched_files.insert(file);
                }
            } else {
                // it's a new contributor - add as-is

                // add the identities as name/email pairs
                let mut name_email_pairs: HashSet<(String, String)> = HashSet::new();
                name_email_pairs.insert((commit.author_name_email.0, commit.author_name_email.1));

                // init the contributor
                let contributor = Contributor {
                    git_identity: git_identity.clone(),
                    name_email_pairs,
                    last_commit_sha1: commit.sha1,
                    last_commit_epoch: commit.date_epoch,
                    last_commit_date: commit.date,
                    touched_files: commit.files,
                };

                contributors.insert(git_identity, contributor);
            }
        }

        contributors.into_iter().map(|(_, v)| v).collect::<Vec<Contributor>>()
    }
}
