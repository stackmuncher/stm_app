use crate::{app_args::AppArgCommands, app_args::AppArgs, help};
use path_absolutize::{self, Absolutize};
use regex::Regex;
use ring::signature::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use serde_json;
use stackmuncher_lib::{config::Config, git::check_git_version, git::get_local_identities, utils::hash_str_sha1};
use std::path::{Path, PathBuf};
use std::process::exit;
use tracing::debug;

/// Name of the file stored in a predefined folder: config.json
const APP_CONFIG_FILE_NAME: &str = "config.json";

/// See HELP module for explanation of what different config flags and params do.
pub(crate) struct AppConfig {
    pub command: AppArgCommands,
    pub no_update: bool,
    pub primary_email: Option<String>,
    pub public_name: Option<String>,
    pub public_contact: Option<String>,
    /// Core config from stackmuncher_lib
    pub lib_config: Config,
    /// Extracted from the key file stored next to the config file
    pub user_key_pair: Ed25519KeyPair,
    /// The full path to the config file.
    pub config_file_path: PathBuf,
}

/// A container for storing some config info locally as a file.
#[derive(Serialize, Deserialize, PartialEq)]
struct AppConfigCache {
    pub primary_email: Option<String>,
    pub public_name: Option<String>,
    pub public_contact: Option<String>,
    pub git_identities: Vec<String>,
}

impl AppConfig {
    /// Inits values from ENV vars and the command line arguments
    pub(crate) async fn new() -> AppConfig {
        // -------------------------------------------------------------------------------------------------------------
        // The sequence of the calls is very important. Some of them read or create resources needed in subsequent steps
        // even if it may not be apparent from the function name. Follow the comments.
        // -------------------------------------------------------------------------------------------------------------

        // used in user messages
        let exe_suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };

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

        // try to read CLI params provided by the user with defaults where no user params were supplied - may panic
        let app_args = AppArgs::read_params();

        // init the subscriber now if the logging level is known from the CLI param
        if let Some(log_level) = &app_args.log {
            tracing_subscriber::fmt()
                .with_max_level(log_level.clone())
                .with_ansi(false)
                .init();
        }

        // get config defaults from the environment - may panic
        let (mut config, keys_dir) = new_config_with_defaults(current_dir).await;

        // if the logging level was provided in a CLI param then the logging was already initialized
        if let Some(log_level) = app_args.log {
            // assign the level, but not no re-initialize
            // it can be initialized only once in the app's lifetime
            config.log_level = log_level;
        } else {
            // using the default logging level - initialize for the first time
            tracing_subscriber::fmt()
                .with_max_level(config.log_level.clone())
                .with_ansi(false)
                .init();
        };

        // rules and project folders are being validated only - not much difference if it's done now or later
        // replace default config with user values from the CLI
        if let Some(rules) = app_args.rules {
            validate_rules_dir(&rules);
            config.code_rules_dir = rules;
        };

        if let Some(project) = app_args.project {
            validate_project_dir(&project);
            config.project_dir = project;
        };

        // reports folder may need to be created in the default or specified location
        config.report_dir = match app_args.reports {
            Some(v) => Some(validate_or_create_report_dir(&config.project_dir, &v)),
            None => Some(validate_or_create_report_dir(
                &config.project_dir,
                config
                    .report_dir
                    .as_ref()
                    .expect("Cannot unwrap config.report_dir. It's a bug."),
            )),
        };

        // keys folder is needed to read or generate a user key-pair and allow caching of some config values in the same folder
        let keys_dir = if let Some(keys) = app_args.keys { keys } else { keys_dir };

        // get existing or generate new key pair
        // it will create STMKEYa directory needed for storing the config cache
        let user_key_pair = crate::signing::get_key_pair(&keys_dir);

        // this step must be done after the keys folder was validated / created
        // it will check the git identities cached in a local file and merge them with what is in git config at the moment
        let config_file_path = keys_dir.join(APP_CONFIG_FILE_NAME);
        let app_config_cache = AppConfigCache::read_from_disk(&config_file_path);

        // primary_email, public_name and public_contact may come from the cache, CLI or git IDs
        let primary_email = if let Some(prim_email_arg) = app_args.primary_email {
            if prim_email_arg.is_empty() {
                // reset the value to NULL if `--primary_email ""`
                debug!("Resetting primary_email to an empty string");
                println!("Your primary email address for notifications from the Directory was removed. Your profile will no longer be updated. You can still generate and view stack reports locally.");
                println!();
                Some(String::new())
            } else {
                // some new value from the CLI
                println!(
                    "{} will be used for notifications about your Directory Profile views and employer interest.",
                    prim_email_arg
                );
                Some(prim_email_arg)
            }
        } else if app_config_cache.primary_email.is_some() {
            // setting the email from cache - no need to print anything for the user
            app_config_cache.primary_email.clone()
        } else if !config.git_identities.is_empty() {
            // setting the email from GIT IDs
            println!("{} is your default Git commit email and will be used for notifications about your Directory Profile views and employer interest.",config.git_identities[0]);
            println!();
            println!(
                "    Run `stackmuncher{} --primary_email me@example.com` to set your preferred contact email. It will not be published or shared with anyone.",
                exe_suffix
            );
            println!();
            Some(config.git_identities[0].clone())
        } else {
            println!("Missing preferred contact email. Your profile will not be updated. You can still generate and view your stack reports locally.");
            println!();
            println!(
                "    Run `stackmuncher{} --primary_email me@example.com` to start updating your Directory profile.",
                exe_suffix
            );
            println!();
            None
        };

        let public_name = if let Some(pub_name) = &app_args.public_name {
            if pub_name.is_empty() {
                // empty public name - make anon
                println!("Your Directory Profile name was removed. Your profile will be anonymous.");
                println!();
                println!(
                    "    Run `stackmuncher{} --public_name \"My Name or Nickname\"` to make it public.",
                    exe_suffix
                );
                println!();
            } else {
                // a new public name was supplied
                println!(
                    "Your new Directory Profile name: {}. It is visible to anyone, including search engines.",
                    pub_name
                );
                println!();
                println!(
                    "    Run `stackmuncher{} --public_name \"\"` to remove the name and make your profile anonymous.",
                    exe_suffix
                );
                println!();
            }
            app_args.public_name
        } else if app_config_cache.public_name.is_some() {
            app_config_cache.public_name.clone()
        } else {
            None
        };

        let public_contact = if let Some(pub_contact) = &app_args.public_contact {
            if pub_contact.is_empty() {
                // no public contact details
                if primary_email.is_some() && !primary_email.as_ref().unwrap().is_empty() {
                    println!("Your Directory Profile contact details were removed. Employers will be able to express their interest via Directory notifications sent to {}.", primary_email.as_ref().unwrap());
                } else {
                    println!("Your Directory Profile contact details were removed. Since your primary email address is blank as well your profile will be hidden.");
                }

                println!();
                println!(
                    "    Run `stackmuncher{} --public_contact \"Your email, website or any other contact details\"` for employers to contact you directly.",
                    exe_suffix
                );
                println!();
            } else {
                // new public contact details
                println!(
                    "Your new Directory Profile contact: {}. It is visible to anyone, including search engines.",
                    pub_contact
                );
                println!();
                println!("    Run `stackmuncher{} --public_contact \"\"` to remove it.", exe_suffix);
                println!();
            }

            app_args.public_contact
        } else if app_config_cache.public_contact.is_some() {
            app_config_cache.public_contact.clone()
        } else {
            None
        };

        // print a message about multiple git IDs on the first run
        if config.git_identities.len() > 0 && app_args.emails.is_none() && app_config_cache.git_identities.is_empty() {
            println!("Only commits from {} will be analyzed. Did you use any other email addresses for Git commits in the past?",config.git_identities[0]);
            println!();
            println!("    1. Run `git shortlog -s -e --all` to check if you made commits under other email addresses.");
            println!("    2. Run `stackmuncher{} --emails \"me@example.com, old@example.com\"` once to add more of your emails for this and future runs.", exe_suffix);
            println!();
        }

        // merge all known git identities in a single unique list (git config + --emails + cached config)
        if let Some(git_ids) = app_args.emails {
            debug!("Adding {} git IDs from CLI", git_ids.len());
            config.git_identities.extend(git_ids);
        }
        config.git_identities.extend(app_config_cache.git_identities.clone());
        config.git_identities.sort();
        config.git_identities.dedup();
        debug!("Valid Git IDs: {}", config.git_identities.len());

        // warn the user if there are no identities to work with
        if config.git_identities.is_empty() {
            println!("Cannot identify which commits are yours without knowing your email address.");
            println!();
            println!("    1. Add your email with `git configure --global user.email me@example.com` to identify your future commits.");
            println!("    2. Run `git shortlog -s -e --all` to check if you made commits under other email addresses.");
            println!("    3. Run `stackmuncher{} --emails \"me@example.com, old@example.com\"` once to add more of your emails for this and future runs.", exe_suffix);
            println!();
        }

        let app_config = AppConfig {
            command: app_args.command,
            no_update: app_args.no_update,
            primary_email,
            public_name,
            public_contact,
            lib_config: config,
            user_key_pair,
            config_file_path,
        };

        app_config_cache.save(&app_config);

        app_config
    }
}

/// Generate a new Config struct with the default values from the environment. May panic if the environment is not accessible.
pub(crate) async fn new_config_with_defaults(current_dir: PathBuf) -> (Config, PathBuf) {
    // look for the rules in the current working dir if in debug mode
    // otherwise default to a platform-specific location
    // this can be overridden by `--rules` CLI param
    let (code_rules_dir, report_dir, keys_dir, log_level) = if cfg!(debug_assertions) {
        (
            Path::new(Config::RULES_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(Config::KEYS_FOLDER_NAME_DEBUG).to_path_buf(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        (
            Path::new(Config::RULES_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(Config::KEYS_FOLDER_NAME_LINUX).to_path_buf(),
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
            local_appdata_dir.join(Config::KEYS_FOLDER_NAME_WIN),
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

    (config, keys_dir)
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
fn validate_or_create_report_dir(project: &PathBuf, report: &PathBuf) -> PathBuf {
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
    }

    // save the project report path in config as String
    report_dir
}

impl AppConfigCache {
    /// Reads cached config settings from `.stm_config` folder or returns a blank sruct if no cached config found
    fn read_from_disk(config_file_path: &PathBuf) -> Self {
        // create a blank dummy to return in case of a problem
        let app_config_cache = AppConfigCache {
            primary_email: None,
            public_name: None,
            public_contact: None,
            git_identities: Vec::new(),
        };

        // check if the file exists
        if !config_file_path.exists() {
            debug!("Config cache file not found");
            return app_config_cache;
        }

        // read the contents
        let cached_file = match std::fs::read(config_file_path.clone()) {
            Err(e) => {
                eprintln!(
                "STACKMUNCHER ERROR: failed to read a cached config file from {}.\n\n    Reason: {}\n\n    Will proceed anyway.",
                config_file_path.absolutize().unwrap_or_default().to_string_lossy(),
                e
            );
                return app_config_cache;
            }
            Ok(v) => v,
        };

        // deserialize from JSON
        let app_config_cache = match serde_json::from_slice::<AppConfigCache>(&cached_file) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                "STACKMUNCHER ERROR: failed to de-serialize a cached config file from {}.\n\n    Reason: {}\n\n    Did you edit the file manually? It will be overwritten with default values.",
                config_file_path.absolutize().unwrap_or_default().to_string_lossy(),
                e
            );
                // return the blank version
                return app_config_cache;
            }
        };

        debug!(
            "Config cache loaded: {}",
            config_file_path.absolutize().unwrap_or_default().to_string_lossy()
        );

        // returns the version with cached contents
        app_config_cache
    }

    /// Extracts persistent parts from `AppConfig` and saves them as a file in `.stm_config` folder.
    /// Does not panic. May print a message on error.
    fn save(self, app_config: &AppConfig) {
        // prepare the data to save
        let app_config_cache = AppConfigCache {
            primary_email: app_config.primary_email.clone(),
            public_name: app_config.public_name.clone(),
            public_contact: app_config.public_contact.clone(),
            git_identities: app_config.lib_config.git_identities.clone(),
        };

        // proceed only if there were any changes to the config or if the config file doesn't exist to create a stub the user can edit
        if app_config_cache == self && app_config.config_file_path.exists() {
            debug!("No config cache changes");
            return;
        }

        // try to serialize and save the config cache
        match serde_json::to_vec(&app_config_cache) {
            Ok(app_config_cache) => {
                if let Err(e) = std::fs::write(app_config.config_file_path.clone(), app_config_cache) {
                    eprintln!(
                "STACKMUNCHER ERROR: failed to save config cache in {}.\n\n    Reason: {}\n\n    It's a bug. Please, report it to https://github.com/stackmuncher/stm.",
                app_config.config_file_path.absolutize().unwrap_or_default().to_string_lossy(),
                e
            );
                } else {
                    debug!(
                        "Config cache saved in {}",
                        app_config
                            .config_file_path
                            .absolutize()
                            .unwrap_or_default()
                            .to_string_lossy()
                    );
                }
            }
            // serialization shouldn't fail
            Err(e) => {
                // nothing the user can do about it and it's not fatal, so inform and carry on
                eprintln!(
                "STACKMUNCHER ERROR: failed to save config cache in {}.\n\n    Reason: {}\n\n    It's a bug. Please, report it to https://github.com/stackmuncher/stm.",
                app_config.config_file_path.absolutize().unwrap_or_default().to_string_lossy(),
                e
            );
            }
        }
    }
}
