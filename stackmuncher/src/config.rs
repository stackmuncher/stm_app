use regex::Regex;

#[derive(Debug)]
pub struct Config {
    /// Full path to the dir with code rules. Absolute or relative to the working dir.
    pub code_rules_dir: String,
    pub log_level: tracing::Level,
    /// Absolute or relative path to the project directory with the files to analyze.
    pub project_dir_path: String,
    /// Registered user name (the validity is not enforced at the moment as it's not pushed anywhere)
    pub user_name: String,
    /// Repo name. Must be unique per user. Reports are attached to `user/repo` ID.
    pub repo_name: String,
    /// A compiled regex for extracting remote URLs from `git remote -v` command
    pub git_remote_url_regex: Regex,
}

impl Config {
    pub const PROJECT_REPORT_FILE_NAME: &'static str = "project_report";
    pub const CONTRIBUTOR_REPORT_FILE_NAME: &'static str = "contributor_report_";
    pub const REPORT_FILE_EXTENSION: &'static str = ".json";
    pub const REPORT_FOLDER_NAME: &'static str = "stm_reports";
    pub const GIT_FOLDER_NAME: &'static str = ".git";

    /// Returns a minimal version of Self with no validation and default values.
    pub fn new(code_rules_dir: String, user_name: String, repo_name: String) -> Self {
        Config {
            log_level: tracing::Level::INFO,
            code_rules_dir,
            project_dir_path: String::new(),
            user_name,
            repo_name,
            git_remote_url_regex: Regex::new(r#"(?i)\s(http.*)\("#).unwrap(),
        }
    }
}
