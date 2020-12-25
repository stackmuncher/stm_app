#[derive(Debug)]
pub struct Config {
    /// Full path to the dir with code rules. Absolute or relative to the working dir.
    pub code_rules_dir: String,
    pub log_level: tracing::Level,
    /// Absolute or relative path to the project directory with the files to analyze.
    pub project_dir_path: String,
    /// Absolute or relative path for the project report produced by the app.
    pub report_file_name: String,
    /// Registered user name (the validity is not enforced at the moment as it's not pushed anywhere)
    pub user_name: String,
    /// Repo name. Must be unique per user. Reports are attached to `user/repo` ID.
    pub repo_name: String,
}

impl Config {
    /// Returns a minimal version of Self with no validation.
    pub fn from_ext_config(code_rules_dir: String, user_name: String, repo_name: String) -> Self {
        Config {
            log_level: tracing::Level::INFO,
            code_rules_dir,
            project_dir_path: String::new(),
            report_file_name: String::new(),
            user_name,
            repo_name,
        }
    }
}
