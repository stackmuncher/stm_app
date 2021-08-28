use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    /// All reports are placed in a centralized location, but this can be overridden by CLI params.
    /// Set it to None if reports are not stored locally at all. Points at the folder for report files
    /// for the current project.
    pub project_report_dir: Option<PathBuf>,
    pub log_level: tracing::Level,
    /// Absolute or relative path to the project directory with the files to analyze.
    pub project_dir: PathBuf,
    /// GitHub user name (the validity is not enforced at the moment as it's not pushed anywhere)
    /// Used only for repos downloaded from GitHub
    pub user_name: String,
    /// GitHub repo name. Must be unique per user. Reports are attached to `user/repo` ID.
    /// Used only for repos downloaded from GitHub
    pub repo_name: String,
    /// List of contributors to generate reports for. Defaults to Git user, author and committer .email values.
    /// Can be overridden by CLI params. The first value in the list is the preferred user contact.
    pub git_identities: Vec<String>,
}

impl Config {
    pub const PROJECT_REPORT_FILE_NAME: &'static str = "project_report";
    /// The prefix of the file name followed by the contributor SHA1 hash.
    pub const CONTRIBUTOR_REPORT_FILE_NAME: &'static str = "contributor_";
    pub const CONTRIBUTOR_REPORT_COMBINED_FILE_NAME: &'static str = "combined_report";
    pub const CONTRIBUTOR_REPORT_SANITIZED_FILE_NAME: &'static str = "submission";
    pub const REPORT_FILE_EXTENSION: &'static str = ".json";
    pub const GIT_FOLDER_NAME: &'static str = ".git";

    /// Returns a minimal version of Self with no validation and default values.
    /// It compiles some regex and should be cached
    pub fn new(user_name: String, repo_name: String) -> Self {
        Config {
            log_level: tracing::Level::INFO,
            project_report_dir: None,
            project_dir: PathBuf::default(),
            user_name,
            repo_name,
            git_identities: Vec::new(),
        }
    }

    /// Returns a minimal version of Self with default values.
    /// The rules and munchers are expected to be in the current folder.
    /// It compiles some regex and should be cached.
    pub fn new_with_defaults(log_level: &tracing::Level) -> Self {
        Config {
            log_level: log_level.clone(),
            project_report_dir: None,
            project_dir: PathBuf::default(),
            user_name: String::new(),
            repo_name: String::new(),
            git_identities: Vec::new(),
        }
    }
}
