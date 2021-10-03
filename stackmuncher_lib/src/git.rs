use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, error, info, trace, warn};

/// An alias for String used for file paths to help with visual type identification.
/// It is not enforced by the compiler and is ignored by the IDE.
pub type FilePath = String;

/// Contains details about a file extracted from GIT
#[derive(Clone, Debug)]
pub struct GitBlob {
    /// SHA1 of the blob itself
    pub sha1: String,
    /// SHA1 of the commit the blob belongs to
    pub commit_sha1: String,
    /// Date of the commit the blob belongs to
    pub commit_date_epoch: i64,
    /// Date of the commit the blob belongs to
    pub commit_date_iso: String,
}

pub type BlobSHA1 = String;
/// #### An alias for HashMap<FilePath, (BlobSHA1, CommitSHA1, CommitDateEpoch, CommitDateIso)>.
/// git ls-tree and some other commands provide blob hash and the file name.
/// E.g. `037498fba1ca5b3662963c848158b7b678adbbf3    .gitignore`.
pub type ListOfBlobs = HashMap<FilePath, GitBlob>;

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
#[derive(Clone)]
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

    /// Returns a concatenated commit hash with the timestamp as a string.
    /// E.g. `e29d17e6_1627380297`. The commit hash is shortened to 8 characters.
    pub(crate) fn join_commit_with_ts(&self) -> String {
        [&self.sha1[..8], "_", self.date_epoch.to_string().as_str()]
            .concat()
            .to_owned()
    }
}

/// Executes a git command in the specified dir with a possible Error as a normal outcome.
/// E.g. some `git config` commands may return an error because there is no such setting, but we don't want to
/// log it as an error because it is an expected outcome. This function returns an error only if no errors are expected or there is an error message attached.
/// Set `expect_blank_err_msg` to `false` if any kind of error should be logged and returned as such.
pub async fn execute_git_command(
    args: Vec<String>,
    repo_dir: &Path,
    expect_blank_err_msg: bool,
) -> Result<Vec<u8>, ()> {
    // build `git ...` command
    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.current_dir(repo_dir);

    // try to run the command - it should never fail at this point unless there is a process failure
    let git_output = match cmd.output().await {
        Err(e) => {
            error!("Git command failed with {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    // status check
    let status = git_output.status.to_string();
    debug!("Status: {}, stdout len: {}", status, git_output.stdout.len());

    // the exit code must be 0 or there was a problem
    if git_output.status.code().is_none() || git_output.status.code() != Some(0) {
        // there may be some useful info in stderr
        let std_err = match String::from_utf8(git_output.stderr) {
            Err(e) => {
                error!("Git command failed. Could not unwrap stderr with {}.", e);
                return Err(());
            }
            Ok(v) => v,
        };
        // ignore errors if they are expected
        if expect_blank_err_msg && std_err.is_empty() {
            debug!("Git command returned blank stderr. Status: {}. Command: {:?}", status, cmd);
            return Ok(vec![]);
        }
        // the command failed and it was not expected
        // keep the logging level at warn because it fails on trivial errors like an empty repo
        warn!("Git command failed. Status: {}. Stderr: {}. Command: {:?}", status, std_err, cmd);
        return Err(());
    }

    // stdout is Vec<u8>
    Ok(git_output.stdout)
}

/// Returns the current git version installed on the machine
pub async fn check_git_version(dir: &Path) -> Result<String, ()> {
    let version = execute_git_command(vec!["--version".into()], dir, false).await?;
    let version = String::from_utf8_lossy(&version).to_string();

    // this is likely to go nowhere if the function is called before the logging was initialized
    info!("{}", version);

    Ok(version)
}

/// Populates blob's sha1 property at the point of the given commit.
/// Only one `git ls-tree` call is used to get the data.
/// * `blobs` param: Must be a ListOfBlobs with commit details populated per file. This function only adds the blob SHA1.
/// The commit details can be taken from `git log` or contributor section of the report.
/// * `commit_sha1` param: either specify a commit SHA1 or None for HEAD.
///
/// The raw git output looks like this:
/// ```
/// 100644 blob a28b99eae8417ac31293a332ef1a125b8772032d    Cargo.toml
/// 100644 blob f288702d2fa16d3cdf0035b15a9fcbc552cd88e7    LICENSE
/// 100644 blob 9da69050aa4d1f6488a258a221217a4dd9e73b71    assets/file-types/cs.json
/// ```
pub(crate) async fn populate_blob_sha1(
    dir: &Path,
    blobs: ListOfBlobs,
    commit_sha1: Option<String>,
) -> Result<ListOfBlobs, ()> {
    // use HEAD if no commit was specified
    let commit_sha1 = match commit_sha1 {
        Some(commit_sha1) => commit_sha1,
        None => "HEAD".into(),
    };

    let all_objects =
        execute_git_command(vec!["ls-tree".into(), "-r".into(), "--full-tree".into(), commit_sha1.clone()], dir, false)
            .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    trace!("{:?}", blobs);

    let updated_blobs = all_objects
        .lines()
        .filter_map(|v| {
            trace! {"get_all_tree_files: {}", v};
            if &v[7..11] == "blob" {
                let file_name = v[53..].to_owned();
                // cloning everything here seems to be inefficient
                if let Some(blob) = blobs.get(&file_name) {
                    Some((
                        file_name,
                        GitBlob {
                            sha1: v[12..52].to_owned(),
                            commit_sha1: blob.commit_sha1.clone(),
                            commit_date_epoch: blob.commit_date_epoch.clone(),
                            commit_date_iso: blob.commit_date_iso.clone(),
                        },
                    ))
                } else {
                    trace!("Ignored {}, in the tree, not requested", file_name);
                    None
                }
            } else {
                None
            }
        })
        .collect::<ListOfBlobs>();
    debug!(
        "Setting blob SHA1's for commit {}. Requested: {}, set: {}",
        commit_sha1,
        blobs.len(),
        updated_blobs.len()
    );

    Ok(updated_blobs)
}

/// Get the list of files from the current GIT tree for a given commit relative to the current directory.
/// Use HEAD if no commit was specified.
/// The raw git output looks like this:
/// ```
/// 100644 blob a28b99eae8417ac31293a332ef1a125b8772032d    Cargo.toml
/// 100644 blob f288702d2fa16d3cdf0035b15a9fcbc552cd88e7    LICENSE
/// 100644 blob 9da69050aa4d1f6488a258a221217a4dd9e73b71    assets/file-types/cs.json
/// ```
pub(crate) async fn get_all_tree_files(
    dir: &Path,
    commit_sha1: Option<String>,
    ignore_paths: &Vec<Regex>,
) -> Result<HashSet<String>, ()> {
    // use HEAD by default
    let commit_sha1 = commit_sha1.unwrap_or("HEAD".to_owned());

    let all_objects =
        execute_git_command(vec!["ls-tree".into(), "-r".into(), "--full-tree".into(), commit_sha1], dir, false).await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let files = all_objects
        .lines()
        .filter_map(|v| {
            trace! {"get_all_tree_files: {}", v};
            if &v[7..11] == "blob" {
                Some(v[53..].to_owned())
            } else {
                None
            }
        })
        .collect::<HashSet<String>>();
    let tree_all = files.len();

    // remove ignored files
    let files = files
        .into_iter()
        .filter_map(|file_path| {
            if is_in_ignore_list(ignore_paths, &file_path) {
                None
            } else {
                Some(file_path)
            }
        })
        .collect::<HashSet<String>>();

    info!(
        "Objects in the GIT tree: {}, ignored: {}, processing: {}",
        tree_all,
        tree_all - files.len(),
        files.len(),
    );

    Ok(files)
}

/// Returns TRUE if the file matches any of the ignore regex rules from `ignore_paths` module.
#[inline]
fn is_in_ignore_list(ignore_paths: &Vec<Regex>, file_path: &str) -> bool {
    // check if the path is in the ignore list
    for ignore_regex in ignore_paths {
        if ignore_regex.is_match(file_path) {
            debug!("Path ignored: {}", file_path);
            return true;
        }
    }

    false
}

/// Get the contents of the Git blob as text.
pub(crate) async fn get_blob_contents(dir: &Path, blob_sha1: &String) -> Result<Vec<u8>, ()> {
    let blob_contents = execute_git_command(vec!["cat-file".into(), "-p".into(), blob_sha1.into()], dir, false).await?;

    Ok(blob_contents)
}

/// Extracts and parses GIT log into who, what, when. Removes ignored files. No de-duping or optimisation is done. All log data is copied into the structs as-is.
/// Merge commits are excluded.
pub async fn get_log(
    repo_dir: &Path,
    contributor_git_identity: Option<&String>,
    ignore_paths: &Vec<Regex>,
) -> Result<Vec<GitLogEntry>, ()> {
    debug!("Extracting git log");

    // prepare the command that may optionally include the author name to limit commits just to that contributor
    let mut git_args = vec![
        "log".into(),
        "--no-decorate".into(),
        "--name-only".into(),
        "--encoding=utf-8".into(),
    ];
    if let Some(author) = contributor_git_identity {
        git_args.push([r#"--author=""#, author, r#"""#].concat());
    };

    // this trace may be needed for unusual `author` values
    trace!("GIT LOG: {:?}", git_args);

    // get the raw stdout output from GIT
    let git_output = execute_git_command(git_args, repo_dir, false).await?;

    // try to convert the commits into a list of lines
    let mut log_entries: Vec<GitLogEntry> = Vec::new();
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
        } else if line.starts_with("Merge:") {
            // We don't use merge info for any particular purpose at the moment
            // potentially, the committer of the merge should get at least some credit for it
            continue;
        } else if line.starts_with("commit ") {
            // commit d5e742de653954bfae88f0e5f6c8f0a7a5f6c437
            // save the previous commit details and start a new one
            // the very first entry will be always blank, it is remove outside the loop
            if current_log_entry.files.len() > 0 {
                // do not add a commit if a commit consists entirely of ignored files or has no files for another reason
                log_entries.push(current_log_entry);
            }
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
            warn!("Split failed on {}", line);
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
                warn!("Encountered a commit with no date: {}", line);
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
            if !is_in_ignore_list(ignore_paths, line) {
                trace!("Added as a file");
                current_log_entry.files.insert(line.into());
            } else {
                trace!("Ignored");
            }
        }
    }

    // the very last commit has to be pushed outside the loop
    log_entries.push(current_log_entry);

    debug!("Found {} commits of interest", log_entries.len());
    Ok(log_entries)
}

/// Extracts all contributor commits from the full log. `git_identities` should be lowercase.
pub fn get_contributor_commits_from_log(git_log: &Vec<GitLogEntry>, git_identities: &Vec<String>) -> Vec<GitLogEntry> {
    git_log
        .iter()
        .filter_map(|entry| {
            if git_identities.contains(&entry.author_name_email.1.to_lowercase()) {
                Some(entry.clone())
            } else {
                None
            }
        })
        .collect::<Vec<GitLogEntry>>()
}

/// Returns a list of possible git identities from user, author and committer settings.
/// The email part of the identity is preferred. The name part is only used if the email is blank.
/// The values are converted to lower case.
pub async fn get_local_identities(repo_dir: &Path) -> Result<Vec<String>, ()> {
    debug!("Extracting git identities");

    let mut git_identities: Vec<String> = Vec::new();

    // git supports 3 types of identities
    // the main one is user, the other 2 will be unused for majority of users
    // they are processed in the order or precedence
    for var_name in ["user", "author", "committer"].iter() {
        let key = [var_name.to_string(), ".email".to_string()].concat();
        // we need to check the email first and if that is blank check the name
        let git_args = vec!["config".into(), key.clone()];
        // git returns an empty error stream if the requested setting does not exist
        // It's possible there was some other problem. The only way to find out is to check the log.
        let git_output = execute_git_command(git_args, repo_dir, true).await?;
        let git_output = String::from_utf8_lossy(&git_output);
        if !git_output.trim().is_empty() {
            trace!("Git ID value for {}: {}", key, git_output);
            // normally this identity should already be known from the additional list because it was stored there
            // during the previous commit and they don't change that often
            let git_output = git_output.trim().to_lowercase();
            if !git_identities.contains(&git_output) {
                git_identities.push(git_output.trim().to_lowercase())
            }
            // it will exit on EMAIL section if the value was found or try NAME section otherwise
            break;
        }
    }

    debug!("Found {} identities", git_identities.len());
    trace!("{:?}", git_identities);
    Ok(git_identities)
}

/// Extracts the list of unique file names from the log with the latest commit/date per file. Ideally, this function should return the blob SHA1 as well,
/// but that info is not available from the log. It loops through all the files listed in `git log` and picks the latest revision per file.
/// Getting just all the tree files seems like a simpler option, but we need commit info, which is only present in `git log` output.
pub(crate) fn log_entries_to_list_of_blobs(git_log: &Vec<GitLogEntry>) -> ListOfBlobs {
    // output container
    let mut blobs: ListOfBlobs = ListOfBlobs::new();

    // go through all log entries, most recent first
    for log_entry in git_log {
        // grab the list of files per commit
        for file in &log_entry.files {
            if let Some(blob) = blobs.get_mut(file) {
                // update the file commit info if encountered a newer file in the source
                if blob.commit_date_epoch < log_entry.date_epoch {
                    warn!(
                        "Wrong commit order for {}. Existing commit: {}, newer: {}",
                        file, blob.commit_sha1, log_entry.sha1
                    );
                    blob.commit_sha1 = log_entry.sha1.clone();
                    blob.commit_date_epoch = log_entry.date_epoch;
                    blob.commit_date_iso = log_entry.date.clone();
                }
            } else {
                // in theory, the commits should be sorted in the chronological order, latest first
                // so we should be OK just adding the file to the collection on first encounter - it should be the latest revision
                let blob = GitBlob {
                    sha1: String::new(),
                    commit_sha1: log_entry.sha1.clone(),
                    commit_date_epoch: log_entry.date_epoch,
                    commit_date_iso: log_entry.date.clone(),
                };
                blobs.insert(file.clone(), blob);
            }
        }
    }
    debug!("list_of_files_with_commits_from_git_log collected {} files from git log", blobs.len());
    blobs
}
