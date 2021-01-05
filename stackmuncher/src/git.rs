use crate::utils;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use tokio::process::Command;
use tracing::{debug, error, info, trace, warn};

pub type FilePath = String;
pub type BlobSHA1 = String;
/// #### An alias for HashMap<FilePath, BlobSHA1>.
/// git ls-tree and some other commands provide blob hash and the file name.
/// E.g. `037498fba1ca5b3662963c848158b7b678adbbf3    .gitignore`.
pub type ListOfBlobs = HashMap<FilePath, BlobSHA1>;

/// A a structured representation of `git log` output. E.g.
/// ```
/// commit f527864cc944d52887d7cc26e79781ac1b01abc2
/// Author: rimutaka <max@onebro.me>
/// Date:   Sat Jan 2 22:33:34 2021 +0000
///
///     Switched from analyzing local files to GIT blobs
///
/// stackmuncher/src/git.rs
/// stackmuncher/src/lib.rs
/// stackmuncher/src/processors/mod.rs
/// stackmuncher/src/report.rs
/// stmapp/src/main.rs
/// ```
pub struct GitLogEntry {
    pub sha1: String,
    pub date_epoch: i64,
    pub date: String,
    pub msg: String,
    pub author_name_email: (String, String),
    pub files: HashSet<String>,
}

impl GitLogEntry {
    /// Returns a blank self
    pub fn new() -> Self {
        Self {
            sha1: String::new(),
            date_epoch: 0,
            date: String::new(),
            msg: String::new(),
            author_name_email: (String::new(), String::new()),
            files: HashSet::new(),
        }
    }
}

/// Executes a git command in the specified dir. Returns stdout or Err.
pub async fn execute_git_command(args: Vec<String>, repo_dir: &String) -> Result<Vec<u8>, ()> {
    // build `git ...` command
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.current_dir(&repo_dir);

    // run git reset
    let git_output = match cmd.output().await {
        Err(_e) => {
            error!("Git command failed");
            return Err(());
        }
        Ok(v) => v,
    };

    // check the status of the cloning
    let status = git_output.status.to_string();
    debug!("Status: {}, stdout len: {}", status, git_output.stdout.len());

    // the exit code must be 0 or there was a problem
    if git_output.status.code().is_none() || git_output.status.code() != Some(0) {
        let std_err = String::from_utf8(git_output.stderr).unwrap_or("Faulty stderr".into());
        error!(
            "Git command failed. Status: {}. Stderr: {}. Command: {:?}",
            status, std_err, cmd
        );
        return Err(());
    }

    // stdout is Vec<u8>
    Ok(git_output.stdout)
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_all_tree_files(dir: &String, commit_sha1: Option<String>) -> Result<ListOfBlobs, ()> {
    // use HEAD if no commit was specified
    let commit_sha1 = commit_sha1.unwrap_or("HEAD".into());

    let all_objects = execute_git_command(
        vec!["ls-tree".into(), "-r".into(), "--full-tree".into(), commit_sha1],
        dir,
    )
    .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let files = all_objects
        .lines()
        .filter_map(|v| {
            trace! {"get_all_tree_files: {}", v};
            if &v[7..11] == "blob" {
                Some((v[53..].to_owned(), v[12..52].to_owned()))
            } else {
                None
            }
        })
        .collect::<ListOfBlobs>();
    info!("Objects in the GIT tree: {}", files.len());

    Ok(files)
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_last_commit_files(dir: &String, all_files: &ListOfBlobs) -> Result<ListOfBlobs, ()> {
    let all_objects = execute_git_command(
        vec![
            "log".into(),
            "--name-only".into(),
            "--oneline".into(),
            "--no-decorate".into(),
            "-1".into(),
        ],
        dir,
    )
    .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let commit_files = all_objects
        .lines()
        .skip(1)
        .map(|v| v.to_owned())
        .collect::<HashSet<String>>();
    info!("Objects in the last commit: {}", commit_files.len());

    // convert vector
    let commit_blobs = all_files
        .iter()
        .filter_map(|(name, sha1)| {
            if commit_files.contains(name) {
                Some((name.clone(), sha1.clone()))
            } else {
                None
            }
        })
        .collect::<ListOfBlobs>();

    Ok(commit_blobs)
}

/// Get the contents of the Git blob as text.
pub(crate) async fn get_blob_contents(dir: &String, blob_sha1: &String) -> Result<Vec<u8>, ()> {
    let blob_contents = execute_git_command(vec!["cat-file".into(), "-p".into(), blob_sha1.into()], dir).await?;

    Ok(blob_contents)
}

/// Returns a list of hashes for all remote URLs for inclusion in the report instead of the URLs themselves for privacy.
/// E.g., `base    https://github.com/awslabs/aws-lambda-rust-runtime.git (fetch)` will get only `https://github.com/awslabs/aws-lambda-rust-runtime.git`
/// hashed as `&str`. The type must match exactly for the hash to be the same. See https://github.com/rust-lang/rust/issues/27108.
pub(crate) async fn get_hashed_remote_urls(dir: &String, git_remote_url_regex: &Regex) -> Result<HashSet<String>, ()> {
    // get the list of remotes, which may look like this
    /*
        base    https://github.com/awslabs/aws-lambda-rust-runtime.git (fetch)
        base    https://github.com/awslabs/aws-lambda-rust-runtime.git (push)
        origin  https://github.com/rimutaka/aws-lambda-rust-runtime.git (fetch)
        origin  https://github.com/rimutaka/aws-lambda-rust-runtime.git (push)
        test    http://local host (fetch)
        test    http://local host (push)
    */
    let all_remotes = execute_git_command(vec!["remote".into(), "-v".into()], dir).await?;
    let all_remotes = String::from_utf8_lossy(&all_remotes);

    debug!("Found {} remotes", all_remotes.lines().count());

    Ok(all_remotes
        .lines()
        .filter_map(|line| {
            trace!("Remote: {}", line);
            if let Some(captures) = git_remote_url_regex.captures(&line) {
                trace!("Captures: {}", captures.len());
                if captures.len() == 2 {
                    Some(utils::hash_str_sha1(captures[1].trim_end_matches("(").trim()))
                } else {
                    None
                }
            } else {
                warn!("No remotes found");
                None
            }
        })
        .collect::<HashSet<String>>())
}

/// Extracts and parses GIT log into who, what, when. No de-duping or optimisation is done. All log data is copied into the structs as-is.
/// Merge commits are excluded.
pub(crate) async fn get_log(repo_dir: &String) -> Result<Vec<GitLogEntry>, ()> {
    debug!("Extracting git log");

    // the output collector
    let mut log_entries: Vec<GitLogEntry> = Vec::new();

    // get the raw stdout output from GIT
    let git_output = execute_git_command(
        vec![
            "log".into(),
            "--no-decorate".into(),
            "--name-only".into(),
            "--no-merges".into(),
            "--encoding=utf-8".into(),
        ],
        repo_dir,
    )
    .await?;

    // try to convert the commits into a list of lines
    let git_output = String::from_utf8_lossy(&git_output);
    if git_output.len() == 0 {
        warn!("Zero-length git log");
        return Ok(log_entries);
    }

    let mut current_log_entry = GitLogEntry::new();

    for line in git_output.lines() {
        trace!("{}", line);
        if line.is_empty() {
            // one empty line is after DATE and one is before COMMIT
            continue;
        } else if line.starts_with("commit ") {
            // commit d5e742de653954bfae88f0e5f6c8f0a7a5f6c437
            // save the previous commit details and start a new one
            // the very first entry will be always blank, it is remove outside the loop
            log_entries.push(current_log_entry);
            current_log_entry = GitLogEntry::new();
            if line.len() > 8 {
                current_log_entry.sha1 = line[7..].to_owned();
            }
        } else if line.starts_with("Author: ") {
            // the author line looks something like this
            //Lorenzo Baboollie <lorenzo@xamsie.be>
            if line.len() < 9 {
                warn!("Corrupt Author line: {}", line);
                continue;
            }
            let author = line[8..].trim();
            if author.is_empty() {
                continue;
            }
            trace!("Author: {}", author);
            // try to split the author details into name and email
            if author.ends_with(">") {
                if let Some(idx) = author.rfind(" <") {
                    let (author_n, author_e) = author.split_at(idx);
                    let author_n = author_n.trim();
                    let author_e = author_e.trim().trim_end_matches(">").trim_start_matches("<");
                    debug!("Author split: {}|{}", author_n, author_e);
                    current_log_entry.author_name_email = (author_n.to_owned(), author_e.to_owned());
                    continue;
                };
            }
            // name/email split failed - add the entire line
            current_log_entry.author_name_email = (author.to_owned(), String::new());
            error!("Split failed on {}", line);
        } else if line.starts_with("Date: ") {
            // Date:   Tue Dec 22 17:43:07 2020 +0000
            if line.len() < 9 {
                warn!("Corrupt Date line: {}", line);
                continue;
            }
            let date = line[6..].trim();
            trace!("Date: {}", date);
            // go to the next line if there is no date (impossible?)
            if date.is_empty() {
                error!("Encountered a commit with no date: {}", line);
                continue;
            }

            // Formatter: https://docs.rs/chrono/0.4.15/chrono/format/strftime/index.html
            // Example: Mon Aug 10 22:47:56 2020 +0200
            if let Ok(d) = chrono::DateTime::parse_from_str(date, "%a %b %d %H:%M:%S %Y %z") {
                trace!("Parsed as: {}", d.to_rfc3339());
                current_log_entry.date = d.to_rfc3339();
                current_log_entry.date_epoch = d.timestamp();
                continue;
            } else {
                error! {"Invalid commit date format: {}", date};
            };
        } else if line.starts_with("    ") {
            // log messages are indented with 4 spaces, including blank lines
            if line.len() < 4 {
                warn!("Corrupt comment line: {}", line);
                continue;
            }
            current_log_entry.msg = [current_log_entry.msg, line[3..].to_owned()].join("\n");
        } else {
            // the only remaining type of data should be the list of files
            // they are not tagged or indented - the entire line is the file name with the relative path
            // file names are displayed only with --name-only option
            trace!("Added as a file");
            current_log_entry.files.insert(line.into());
        }
    }

    log_entries.remove(0);

    debug!("Found {} commits", log_entries.len());
    Ok(log_entries)
}
