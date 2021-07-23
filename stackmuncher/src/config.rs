use crate::{app_args::AppArgCommands, app_args::AppArgs, help};
use path_absolutize::{self, Absolutize};
use regex::Regex;
use stackmuncher_lib::{config::Config, git::check_git_version, git::get_local_identities, utils::hash_str_sha1};
use std::path::{Path, PathBuf};
use std::process::exit;

pub(crate) struct AppConfig {
    pub command: AppArgCommands,
    pub no_update: bool,
    pub primary_email: Option<String>,
    pub public_name: Option<String>,
    pub public_contact: Option<String>,
    pub lib_config: Config,
}

impl AppConfig {
    /// Inits values from ENV vars and the command line arguments
    pub(crate) async fn new() -> AppConfig {
        // assume that the project_dir is the current working folder
        let current_dir = match std::env::current_dir() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("STACKMUNCHER CONFIG ERROR: Cannot get the name of the current directory due to {}", e);
                help::emit_usage_msg();
                exit(1);
            }
        };

        // check if GIT is installed
        // this check will change to using the git supplied as part of STM package
        if let Err(_e) = check_git_version(&current_dir).await {
            eprintln!(
                "STACKMUNCHER CONFIG ERROR: Cannot launch Git from {} folder. Is it installed on this machine?",
                current_dir.to_string_lossy()
            );
            help::emit_usage_msg();
            exit(1);
        };

        // try to read CLI params provided by the user with defaults where no user params were supplied
        let app_args = AppArgs::read_params();

        // get config defaults from the environment
        let mut config = new_config_with_defaults(current_dir).await;

        // replace default config with user values from the CLI
        if let Some(rules) = app_args.rules {
            validate_rules_dir(&rules);
            config.code_rules_dir = rules;
        };

        if let Some(project) = app_args.project {
            validate_project_dir(&project);
            config.project_dir = project;
        };

        config.report_dir = match app_args.reports {
            Some(v) => Some(generate_report_dir(&config.project_dir, &v)),
            None => Some(generate_report_dir(
                &config.project_dir,
                config
                    .report_dir
                    .as_ref()
                    .expect("Cannot unwrap config.report_dir. It's a bug."),
            )),
        };

        if let Some(log_level) = app_args.log {
            config.log_level = log_level;
        };

        if let Some(emails) = app_args.emails {
            if emails.is_empty() {
                println!("Found empty `--emails` CLI param. Will generate a project report only.")
            }
            config.git_identities = emails;
        }else {
            if config.git_identities.is_empty() {
                println!("This app looks for commits with an email from `git configure user.email` or multiple emails from `--emails` CLI param.");
                println!("Both are empty. Will generate a project report only.");
                println!();
                println!("    1. Add your email with `git configure --global user.email me@gmail.com` to identify your future commits.");
                println!("    2. Run `git shortlog -s -e --all` to check if you made commits under other email addresses.");
                println!("    3. Use `--emails \"me@gmail.com me@example.com\"` param to include contributions from multiple addresses and ignore git `user.email` setting.");
                println!();
            }
        };

        let primary_email = match app_args.primary_email {
            Some(v) => Some(v),
            None => match config.git_identities.len() {
                0 => None,
                _ => Some(config.git_identities[0].clone()),
            },
        };

        AppConfig {
            command: app_args.command,
            no_update: app_args.no_update,
            primary_email,
            public_name: app_args.public_name,
            public_contact: app_args.public_contact,
            lib_config: config,
        }
    }
}

/// Generate a new Config struct with the default values from the environment.
pub(crate) async fn new_config_with_defaults(current_dir: PathBuf) -> Config {
    // look for the rules in the current working dir if in debug mode
    // otherwise default to a platform-specific location
    // this can be overridden by `--rules` CLI param
    let (code_rules_dir, report_dir, log_level) = if cfg!(debug_assertions) {
        (
            Path::new(Config::RULES_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_DEBUG).to_path_buf(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        (
            Path::new(Config::RULES_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_LINUX).to_path_buf(),
            tracing::Level::ERROR,
        )
    } else if cfg!(target_os = "windows") {
        // the easiest way to store the rules on Win is next to the executable
        let exec_dir = match std::env::current_exe() {
            Err(e) => {
                // in theory, this should never happen
                panic!(
                    "No current exe path: {}. This is a bug. The app should at least see the path to its own executable.",
                    e
                );
            }
            Ok(v) => v
                .parent()
                .expect(&format!("Cannot determine the location of the exe file from: {}", v.to_string_lossy()))
                .to_path_buf(),
        };

        // apps should store their data in the user profile and the exact location is obtained via an env var
        let local_appdata_dir = std::env::var("LOCALAPPDATA").expect("%LOCALAPPDATA% env variable not found");
        let local_appdata_dir = Path::new(&local_appdata_dir);
        (
            exec_dir.join(Config::RULES_FOLDER_NAME_WIN),
            local_appdata_dir.join(Config::REPORT_FOLDER_NAME_WIN),
            tracing::Level::ERROR,
        )
    } else {
        unimplemented!("Only Linux and Windows are supported at the moment");
    };

    // find out what email addresses are known from Git for processing contributors individually as the default option
    let git_identities = match get_local_identities(&current_dir).await {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };

    let config = Config {
        log_level,
        code_rules_dir,
        report_dir: Some(report_dir),
        project_dir: current_dir,
        user_name: String::new(),
        repo_name: String::new(),
        git_remote_url_regex: Regex::new(Config::GIT_REMOTE_URL_REGEX).unwrap(),
        git_identities,
    };

    config
}

/// Shortens a potentially long folder name like home_mx_projects_stm_stm_apps_stm_28642a39
/// to a reasonable length of about 250 bytes, which can be 250 ASCII chars or much fewer for UTF-8.
/// The trimming is done by cutting off segments at the _ from the start.
/// The worst case scenario it will be just the hash left.
fn trim_canonical_project_name(name: String) -> String {
    let mut name = name;

    // windows - 260, linux - 255, mac - 255, but that is in chars
    // it gets tricky with UTF-8 because some chars are multi-byte, so technically it is even shorter
    // having a very long name is kind of pointless - most useful info is at the end,
    // anything over 100 glyphs would be hard to read
    while name.as_bytes().len() > 250 {
        // cut off the first segment_
        // there should be at least one _ under the 255 limit because the hash is at the end and it's only 8 chars long
        let cut_off_idx = name
            .find("_")
            .expect("Failed to trim a canonical project name. It's a bug")
            + 1;
        name = name[cut_off_idx..].to_string();
        // keep cutting until it is within the acceptable limit
        continue;
    }

    name
}

/// Validates the value for config.code_rules_dir and does process::exit(1) on error.
/// Prints error messages.
fn validate_rules_dir(rules: &PathBuf) {
    // this checks if the rules dir is present, but not its contents
    // incomplete, may fall over later
    if !rules.exists() {
        eprintln!("STACKMUNCHER CONFIG ERROR: Cannot find StackMuncher code parsing rules.");
        help::emit_code_rules_msg();
        exit(1);
    }

    // check if the sub-folders of stm_rules are present
    let file_type_dir = rules.join(Config::RULES_SUBFOLDER_FILE_TYPES);
    if !file_type_dir.exists() {
        let file_type_dir = file_type_dir
            .absolutize()
            .expect("Cannot convert rules / file_types dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find file type rules folder {}",
            file_type_dir.to_string_lossy()
        );
        help::emit_code_rules_msg();
        std::process::exit(1);
    }

    // check if the munchers sub-folder is present
    let muncher_dir = rules.join(Config::RULES_SUBFOLDER_MUNCHERS);
    if !muncher_dir.exists() {
        let muncher_dir = muncher_dir
            .absolutize()
            .expect("Cannot convert rules / munchers dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find rules directory for munchers in {}",
            muncher_dir.to_string_lossy()
        );
        help::emit_code_rules_msg();
        std::process::exit(1);
    }
}

/// Validates config.project_dir
fn validate_project_dir(project: &PathBuf) {
    // the project dir at this point is either a tested param from the CLI or the current dir
    // a full-trust app is guaranteed access to the current dir
    // a restricted app would need to test if the dir is actually accessible, but it may fail over even earlier when it tried to get the current dir name

    // check if there is .git subfolder in the project dir
    let git_path = project.join(".git");
    if !git_path.is_dir() {
        eprintln!(
            "STACKMUNCHER CONFIG ERROR: No Git repository found in the project folder {}",
            project.to_string_lossy()
        );
        help::emit_usage_msg();
        exit(1);
    }
}

/// Validates the value for the reports dir, adds the project component to it and creates the directory if needed.
/// Prints error messages and exits on error.
fn generate_report_dir(project: &PathBuf, report: &PathBuf) -> PathBuf {
    // individual project reports are grouped in their own folders - build that path here
    // this can be relative or absolute, which should be converted into absolute in a canonical form as a single folder name
    // e.g. /var/tmp/stackmuncher/reports/home_ubuntu_projects_some_project_name_1_6bdf08b3 were the last part is a canonical project name built
    // out of the absolute project path and its own hash
    // the hash is included in the path for ease of search and matching with the report contents because the report itself does not contain any project or user
    // identifiable info
    let absolute_project_path = if project.is_absolute() {
        project.to_string_lossy().to_string()
    } else {
        // join the current working folder with the relative path to the project
        std::env::current_dir()
            .expect("Cannot get the current dir. It's a bug.")
            .join(project)
            .to_string_lossy()
            .to_string()
    };

    // convert the absolute project path to its canonical name
    let canonical_project_name = Regex::new(r#"\W+"#)
        .expect("Invalid canonical report path regex. It's a bug.")
        .replace_all(&absolute_project_path, "_")
        .trim_matches('_')
        .to_lowercase();

    // append its own hash at the end
    let canonical_project_name_hash = hash_str_sha1(&canonical_project_name)[0..8].to_string();
    let canonical_project_name = [canonical_project_name, canonical_project_name_hash].join("_");
    let canonical_project_name = trim_canonical_project_name(canonical_project_name);

    // append the project report subfolder name to the reports root folder
    let report_dir = report.join(canonical_project_name);

    // check if the project report folder exists or create it if possible
    if !report_dir.is_dir() {
        if report_dir.exists() {
            // the path exists as something else
            eprintln!(
                "STACKMUNCHER CONFIG ERROR. The path to report directory exists, but it is not a directory: {}",
                report_dir.to_string_lossy()
            );
        }
        // create it
        if let Err(e) = std::fs::create_dir_all(report_dir.clone()) {
            eprintln!(
                "STACKMUNCHER CONFIG ERROR. Cannot create reports directory at {} due to {}",
                report_dir.to_string_lossy(),
                e
            );
        };
        println!("StackMuncher reports folder: {}", report_dir.to_string_lossy());
    }

    // save the project report path in config as String
    report_dir
}
