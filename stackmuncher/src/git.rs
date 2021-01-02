use std::collections::{HashMap, HashSet};
use tokio::process::Command;
use tracing::{debug, error, info, trace};

pub type FilePath = String;
pub type BlobSHA1 = String;
/// #### An alias for HashMap<FilePath, BlobSHA1>.
/// git ls-tree and some other commands provide blob hash and the file name.
/// E.g. `037498fba1ca5b3662963c848158b7b678adbbf3    .gitignore`.
pub type ListOfBlobs = HashMap<FilePath, BlobSHA1>;

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
pub async fn get_all_tree_files(dir: &String) -> Result<ListOfBlobs, ()> {
    let all_objects = execute_git_command(
        vec!["ls-tree".into(), "-r".into(), "--full-tree".into(), "HEAD".into()],
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
pub async fn get_blob_contents(dir: &String, blob_sha1: &String) -> Result<Vec<u8>, ()> {
    let blob_contents = execute_git_command(vec!["cat-file".into(), "-p".into(), blob_sha1.into()], dir).await?;

    Ok(blob_contents)
}
