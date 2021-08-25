use crate::{app_args::AppArgCommands, app_args::AppArgs, help};
use path_absolutize::{self, Absolutize};
use regex::Regex;
use ring::signature::Ed25519KeyPair;
use serde::{Deserialize, Serialize};
use serde_json;
use stackmuncher_lib::{config::Config, git::check_git_version, git::get_local_identities, utils::hash_str_sha1};
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use tracing::debug;

/// Name of the file stored in a predefined folder: config.json
const APP_CONFIG_FILE_NAME: &str = "config.json";

/// The location of user config and keys for signing STM Inbox messages: `.stm_config`
const CONFIG_FOLDER_NAME_DEBUG: &'static str = ".stm_config";
/// The location of user config and keys for signing STM Inbox messages: `/usr/share/stackmuncher/.stm_config`
const CONFIG_FOLDER_NAME_LINUX: &'static str = "/usr/share/stackmuncher/.stm_config";
/// This value is to be appended to the folder of the executable
const CONFIG_FOLDER_NAME_WIN: &'static str = ".stm_config";

/// See HELP module for explanation of what different config flags and params do.
pub(crate) struct AppConfig {
    pub command: AppArgCommands,
    pub dryrun: bool,
    // An empty string means NO CONTACT
    pub primary_email: Option<String>,
    /// A 32-byte long hex string of the Gist ID with the validation string for the user GH account
    /// E.g. `fb8fc0f87ee78231f064131022c8154a`
    /// It is validated on change and then cached in config.json
    pub gh_validation_id: Option<String>,
    /// GitHub login of the user. It is set after validating the a/c ownership and then cached in config.json
    pub gh_login: Option<String>,
    /// Core config from stackmuncher_lib
    pub lib_config: Config,
    /// Extracted from the key file stored next to the config file
    pub user_key_pair: Ed25519KeyPair,
    /// The full path to the config file.
    pub config_file_path: PathBuf,
    /// A stash for validation Gist details to avoid going to GitHub twice
    /// Not cached and can only be present if --gist param was used to link to a new github a/c
    pub gh_validation_gist: Option<crate::cmd_config::Gist>,
}

/// A container for storing some config info locally as a file.
#[derive(Serialize, Deserialize, PartialEq)]
struct AppConfigCache {
    // An empty string means NO CONTACT
    pub primary_email: Option<String>,
    pub gh_validation_id: Option<String>,
    // It is a derivitive value. Used for displaying a profile URL only.
    pub gh_login: Option<String>,
    pub git_identities: Vec<String>,
}

impl AppConfig {
    /// Inits values from ENV vars and the command line arguments
    pub(crate) async fn new() -> AppConfig {
        // -------------------------------------------------------------------------------------------------------------
        // The sequence of the calls is very important. Some of them read or create resources needed in subsequent steps
        // even if it may not be apparent from the function name. Follow the comments.
        // -------------------------------------------------------------------------------------------------------------

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
        let (mut config, config_dir_default) = new_config_with_defaults(current_dir).await;

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

        // only validate project, rules and report if code analysis is to be done
        // config should be validated regardless because nothing functions without it
        if app_args.command == AppArgCommands::Munch {
            // only `project` folder is being validated - not much difference if it's done now or later
            // replace default config with user values from the CLI

            // check the project folder for existence and if it has .git in it
            config.project_dir = match app_args.project {
                Some(project) => {
                    // expand ~/somepath on Linux to /home/user/...
                    let project = tilde_expand(project);
                    validate_project_dir(project)
                }
                None => {
                    // validate the default value
                    validate_project_dir(config.project_dir)
                }
            };

            // reports folder may need to be created in the default or specified location
            config.report_dir = match app_args.reports {
                Some(v) => {
                    // expand ~/somepath on Linux to /home/user/...
                    let v = tilde_expand(v);

                    Some(validate_or_create_report_dir(&config.project_dir, &v))
                }
                None => Some(validate_or_create_report_dir(
                    &config.project_dir,
                    config
                        .report_dir
                        .as_ref()
                        .expect("Cannot unwrap config.report_dir. It's a bug."),
                )),
            };
        }

        // config folder is needed to read or generate a user key-pair and allow caching of some config values in the same folder
        let config_dir = if let Some(conf_dir_from_args) = app_args.config {
            // expand ~/somepath on Linux to /home/user/...
            tilde_expand(conf_dir_from_args)
        } else {
            config_dir_default
        };

        // get existing or generate new key pair
        // it will create STMKEYa directory needed for storing the config cache
        let user_key_pair = crate::signing::get_key_pair(&config_dir);

        // this step must be done after the config folder was validated / created
        // it will check the git identities cached in a local file and merge them with what is in git config at the moment
        let config_file_path = config_dir.join(APP_CONFIG_FILE_NAME);
        let app_config_cache = AppConfigCache::read_from_disk(&config_file_path);

        // primary_email, public_name and public_contact may come from the cache, CLI or git IDs
        let primary_email = if let Some(prim_email_arg) = app_args.primary_email {
            if prim_email_arg.is_empty() {
                // reset the value to NULL if `--primary_email ""`
                debug!("Resetting primary_email to an empty string");
                println!("Your primary email address for notifications from the Directory was removed.");
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
            println!();
            println!("{} is your default Git commit email and will be used for notifications about your Directory Profile views and employer interest.",config.git_identities[0]);
            println!(
                "    Run `stackmuncher{} --primary_email me@example.com` to set your preferred contact email. It will not be published or shared with anyone.",
                EXE_SUFFIX
            );
            println!();
            Some(config.git_identities[0].clone())
        } else {
            println!("Missing preferred contact email. Your profile will not be updated. You can still generate and view your stack reports locally.");
            println!();
            println!(
                "    Run `stackmuncher{} --primary_email me@example.com` to set your preferred contact email for notifications about profile views and employer interest.",
                EXE_SUFFIX
            );
            println!();
            None
        };

        // print a message about multiple git IDs on the first run
        if config.git_identities.len() > 0 && app_args.emails.is_none() && app_config_cache.git_identities.is_empty() {
            println!("Only commits from {} will be analyzed. Did you use any other email addresses for Git commits in the past?",config.git_identities[0]);
            println!("    1. Run `git shortlog -s -e --all` to check if you made commits under other email addresses.");
            println!("    2. Run `stackmuncher{} --emails \"me@example.com, old@example.com\"` once to add more of your emails for this and future runs.", EXE_SUFFIX);
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
            println!("    3. Run `stackmuncher{} --emails \"me@example.com, old@example.com\"` once to add more of your emails for this and future runs.", EXE_SUFFIX);
            println!();
        }

        // GitHub login validation - use the validated ID or None if --gist param was provided
        // It means that the user requested a change of sorts.
        // Otherwise use what is in the cache without any validation.
        let (gh_validation_id, gh_login, gh_validation_gist) = if app_args.gh_validation_id.is_some() {
            // --gist was present - so a change was requested by the user
            match crate::cmd_config::get_validated_gist(&app_args.gh_validation_id, &user_key_pair).await {
                // the gist struct will be needed to print config details later
                Some(gist) => (app_args.gh_validation_id.clone(), Some(gist.login.clone()), Some(gist)),
                None => (None, None, None),
            }
        } else {
            // --gist was not present - use what's in cache
            (app_config_cache.gh_validation_id.clone(), app_config_cache.gh_login.clone(), None)
        };

        let app_config = AppConfig {
            command: app_args.command,
            dryrun: app_args.dryrun,
            primary_email,
            gh_validation_id,
            lib_config: config,
            user_key_pair,
            config_file_path,
            gh_validation_gist,
            gh_login,
        };

        app_config_cache.save(&app_config);

        app_config
    }
}

/// Generate a new Config struct with the default values from the environment. May panic if the environment is not accessible.
pub(crate) async fn new_config_with_defaults(current_dir: PathBuf) -> (Config, PathBuf) {
    // check if the app was compiled for release, but is still sitting in target/release/ folder
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
    let is_local_release = exec_dir.ends_with("target/release") || exec_dir.ends_with("target\\release");

    // use the current working dir if in debug mode
    // otherwise default to a platform-specific location
    // this can be overridden by `--report` and `--config` CLI param
    let (report_dir, config_dir, log_level) = if is_local_release {
        // this branch activates when the app is called directly from `stm_app/target/release` folder, but all the config files are 2 levels up
        // go 2 steps up in the hierarchy to get to the root of stm_app project
        let mut exec_dir = exec_dir;
        exec_dir.pop();
        exec_dir.pop();
        (
            exec_dir.join(Config::REPORT_FOLDER_NAME_DEBUG),
            exec_dir.join(CONFIG_FOLDER_NAME_DEBUG),
            tracing::Level::ERROR,
        )
    } else if cfg!(debug_assertions) {
        // this branch activates when run as `cargo run`
        (
            Path::new(Config::REPORT_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(CONFIG_FOLDER_NAME_DEBUG).to_path_buf(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        (
            Path::new(Config::REPORT_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(CONFIG_FOLDER_NAME_LINUX).to_path_buf(),
            tracing::Level::ERROR,
        )
    } else if cfg!(target_os = "windows") {
        // apps should store their data in the user profile and the exact location is obtained via an env var
        let local_appdata_dir = std::env::var("LOCALAPPDATA").expect("%LOCALAPPDATA% env variable not found");
        let local_appdata_dir = Path::new(&local_appdata_dir);
        (
            local_appdata_dir.join(Config::REPORT_FOLDER_NAME_WIN),
            local_appdata_dir.join(CONFIG_FOLDER_NAME_WIN),
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
        report_dir: Some(report_dir),
        project_dir: current_dir,
        user_name: String::new(),
        repo_name: String::new(),
        git_identities,
    };

    (config, config_dir)
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

/// Returns a validated config.project_dir or exits with an error message
fn validate_project_dir(project: PathBuf) -> PathBuf {
    // the project dir at this point is either a tested param from the CLI or the current dir
    // a full-trust app is guaranteed access to the current dir
    // a restricted app would need to test if the dir is actually accessible, but it may fail over even earlier when it tried to get the current dir name

    if !project.is_dir() {
        eprintln!("STACKMUNCHER CONFIG ERROR: invalid project folder {}", project.to_string_lossy());
        help::emit_usage_msg();
        exit(1);
    }

    // check if there is .git subfolder in the project dir
    let git_path = project.join(".git");
    if !git_path.is_dir() {
        eprintln!("STACKMUNCHER CONFIG ERROR: No Git repository found in {}", git_path.to_string_lossy());
        eprintln!("    Try running the app from the root of a project with a .git subfolder.");
        help::emit_usage_msg();
        exit(1);
    }

    project
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
            gh_validation_id: None,
            gh_login: None,
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
            gh_validation_id: app_config.gh_validation_id.clone(),
            git_identities: app_config.lib_config.git_identities.clone(),
            gh_login: app_config.gh_login.clone(),
        };

        // proceed only if there were any changes to the config or if the config file doesn't exist to create a stub the user can edit
        if app_config_cache == self && app_config.config_file_path.exists() {
            debug!("No config cache changes");
            return;
        }

        // try to serialize and save the config cache
        match serde_json::to_vec_pretty(&app_config_cache) {
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

/// Replaces `~` in Linux paths with the full path to the home directory.
/// E.g. `~/rust/stm_app` -> `/home/ubuntu/rust/stm_app`
fn tilde_expand(path: PathBuf) -> PathBuf {
    // check if there is a ~ at all
    if !path.starts_with("~") {
        return path;
    }

    // is there a home directory?
    let home_dir = match std::env::var("HOME") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("STACKMUNCHER CONFIG ERROR: Cannot get the name of HOME directory due to {}", e);
            eprintln!();
            eprintln!("     Try replacing ~ with an absolute path.");
            eprintln!();
            help::emit_usage_msg();
            exit(1);
        }
    };

    debug!("Home dir: {}", home_dir);

    let home_dir = match PathBuf::from_str(&home_dir) {
        Err(_) => {
            eprintln!("STACKMUNCHER CONFIG ERROR: $HOME has invalid home directory path: {}", home_dir);
            eprintln!();
            eprintln!("     Try replacing ~ with an absolute path.");
            eprintln!();
            help::emit_usage_msg();
            exit(1);
        }
        Ok(v) => {
            if path.starts_with("~/") {
                v.join(&path.to_string_lossy()[2..])
            } else {
                eprintln!("STACKMUNCHER CONFIG ERROR: cannot expand ~ shortcut");
                eprintln!();
                eprintln!("     Try replacing ~ with an absolute path.");
                eprintln!();
                help::emit_usage_msg();
                exit(1);
            }
        }
    };

    debug!("Expanded {} -> {}", path.to_string_lossy(), home_dir.to_string_lossy());
    home_dir
}
