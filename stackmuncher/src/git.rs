use std::path::Path;
use tokio::process::Command;
use tracing::{debug, error, info};

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
        error!("Git command failed. Status: {}. Stderr: {}", status, std_err);
        return Err(());
    }

    // stdout is Vec<u8>
    Ok(git_output.stdout)
}

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_all_tree_files(dir: &Path) -> Result<Vec<String>, ()> {
    let all_objects = execute_git_command(
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

/// Get the list of files from the current GIT tree (HEAD) relative to the current directory
pub async fn get_last_commit_files(dir: &Path) -> Result<Vec<String>, ()> {
    let all_objects = execute_git_command(
        vec![
            "log".into(),
            "--name-only".into(),
            "--oneline".into(),
            "--no-decorate".into(),
            "-1".into(),
        ],
        &dir.to_string_lossy().to_string(),
    )
    .await?;
    let all_objects = String::from_utf8_lossy(&all_objects);

    let files = all_objects
        .lines()
        .skip(1)
        .map(|v| v.to_owned())
        .collect::<Vec<String>>();
    info!("Objects in the last commit: {}", files.len());

    Ok(files)
}
