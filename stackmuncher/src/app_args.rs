use crate::help;
use pico_args;
use regex::Regex;
use std::env::consts::EXE_SUFFIX;
use std::str::FromStr;
use std::{path::PathBuf, process::exit};

pub(crate) const GIST_ID_REGEX: &str = "[a-f0-9]{32}";

/// List of valid app commands
#[derive(PartialEq)]
pub(crate) enum AppArgCommands {
    /// The default value
    Munch,
    /// Display a detailed usage message
    Help,
    /// Display details of the current config (folders, git ids)
    ViewConfig,
    /// Remove name and contact details from the directory making the profile anonymous
    MakeAnon,
    /// Completely delete the member profile from the directory
    DeleteProfile,
    /// Configure Github validation page
    GitGHubConfig,
}

/// A container for user-provided CLI commands and params. The names of the members correspond
/// to the names of CLI args. E.g. --emails -> emails
pub(crate) struct AppArgs {
    pub command: AppArgCommands,
    pub dryrun: bool,
    pub primary_email: Option<String>,
    pub emails: Option<Vec<String>>,
    /// A 32-byte long hex string of the Gist ID with the validation string for the user's GH account
    /// E.g. `fb8fc0f87ee78231f064131022c8154a`
    pub gh_validation_id: Option<String>,
    pub project: Option<PathBuf>,
    pub reports: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub log: Option<tracing::Level>,
}

impl FromStr for AppArgCommands {
    type Err = ();
    /// Returns a parsed value or prints an error message and exits.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let command = s.trim().to_lowercase();
        let command = match command.as_str() {
            "help" => Self::Help,
            "" | "munch" => Self::Munch,
            "config" => Self::ViewConfig,
            "makeanon" | "make-anon" | "make_anon" => Self::MakeAnon,
            "deleteprofile" | "delete-profile" | "delete_profile" | "delete" => Self::DeleteProfile,
            "github" => Self::GitGHubConfig,
            _ => {
                eprintln!("STACKMUNCHER CONFIG ERROR: invalid command `{}`", command);
                help::emit_usage_msg();
                exit(1);
            }
        };

        Ok(command)
    }
}

impl AppArgs {
    /// Read the CLI params from the environment and place them in `self`.
    /// Uses None for omitted params.
    pub(crate) fn read_params() -> Self {
        let mut app_args = AppArgs {
            command: AppArgCommands::Munch,
            dryrun: false,
            primary_email: None,
            emails: None,
            gh_validation_id: None,
            project: None,
            reports: None,
            config: None,
            log: None,
        };

        // read the params into a parser
        let mut pargs = pico_args::Arguments::from_env();

        // process sub-command
        match pargs.subcommand() {
            Ok(v) => {
                if let Some(command) = v {
                    app_args.command =
                        AppArgCommands::from_str(&command).expect("Failed to parse subcommand. It's a bug.");
                };
            }
            Err(_) => {
                help::emit_cli_err_msg();
                exit(1);
            }
        };

        // help has a higher priority and should be handled separately
        if pargs.contains(["-h", "--help"]) {
            app_args.command = AppArgCommands::Help;
        }

        // --noupdate param with different misspellings
        app_args.dryrun = pargs.contains("--dryrun") || pargs.contains("--dry-run") || pargs.contains("--dry_run");

        // --primary_email
        if let Some(primary_email) =
            find_arg_value(&mut pargs, vec!["--primary_email", "--primary-email", "--primaryemail"])
        {
            app_args.primary_email = Some(primary_email);
        };

        // emails are a comma-separated list and should be cleaned up from various forms like
        // a@example.com,,d@example.com,
        // "a@example.com d@example.com"
        // can be empty if the user wants the project report only and no contributor reports
        if let Some(emails) = find_arg_value(&mut pargs, vec!["--emails"]) {
            let emails = emails
                .trim()
                .to_lowercase()
                .replace(" ", ",")
                .split(",")
                .filter_map(|v| if v.is_empty() { None } else { Some(v.to_owned()) })
                .collect::<Vec<String>>();

            app_args.emails = Some(emails);
        };

        // --gist
        if let Some(gist_url) = find_arg_value(&mut pargs, vec!["--gist"]) {
            // extract the gist id from the input, which can be the full URL, just the ID or the raw URL which is even longer
            // e.g. fb8fc0f87ee78231f064131022c8154a
            // or https://gist.github.com/rimutaka/fb8fc0f87ee78231f064131022c8154a
            // or https://gist.githubusercontent.com/rimutaka/fb8fc0f87ee78231f064131022c8154a
            // or https://gist.githubusercontent.com/rimutaka/fb8fc0f87ee78231f064131022c8154a/raw/1e99cbb2ae82c4ebfb3df7195a150f81142b894a/stm.txt

            if gist_url.is_empty() {
                // the user requested removal of GH login
                app_args.gh_validation_id = Some(String::new());
            } else if let Some(matches) = Regex::new(GIST_ID_REGEX)
                .expect("Invalid gist_id_regex. It's a bug.")
                .find(&gist_url)
            {
                // some value with a likely-looking gist id was provided
                app_args.gh_validation_id = Some(matches.as_str().to_string());
            } else {
                // some other value was provided
                eprintln!("STACKMUNCHER CONFIG ERROR: param `--gist` has an invalid value.",);
                eprintln!();
                eprintln!("    It accepts either the Gist URL or the Gist ID found in the Gist URL:",);
                eprintln!("    * https://gist.github.com/rimutaka/fb8fc0f87ee78231f064131022c8154a");
                eprintln!("    * fb8fc0f87ee78231f064131022c8154a");
                eprintln!();
                eprintln!("    To unlink from GitHub and remove private projects from your public profile use `stackmuncher{} --gist \"\"`", EXE_SUFFIX);
                help::emit_usage_msg();
                exit(1);
            }
        };

        // project folder
        if let Some(project) = find_arg_value(&mut pargs, vec!["--project", "-p"]) {
            // en empty value doesn't make sense in this context
            if project.trim().is_empty() {
                eprintln!(
                    "STACKMUNCHER CONFIG ERROR: param `--project` has no value. Omit it to use the current folder or provide a valid path to where the project is located (absolute or relative).",
                );
                help::emit_usage_msg();
                exit(1);
            }

            match PathBuf::from_str(&project) {
                Ok(v) => app_args.project = Some(v),
                Err(_) => {
                    eprintln!(
                        "STACKMUNCHER CONFIG ERROR: `{}` is not a valid path for `--project`. Omit that param to use the current folder or provide a valid path to where the project is located (absolute or relative).",
                        project
                    );
                    help::emit_usage_msg();
                    exit(1);
                }
            }
        };

        // report folder
        if let Some(reports) = find_arg_value(&mut pargs, vec!["--reports"]) {
            // en empty value doesn't make sense in this context
            if reports.trim().is_empty() {
                eprintln!(
                    "STACKMUNCHER CONFIG ERROR: param `--reports` has no value. Omit it to use the default location or provide a valid path to where report files should be placed (absolute or relative).",
                );
                help::emit_report_dir_msg();
                exit(1);
            }

            match PathBuf::from_str(&reports) {
                Ok(v) => app_args.reports = Some(v),
                Err(_) => {
                    eprintln!(
                        "STACKMUNCHER CONFIG ERROR: `{}` is not a valid path for `--reports`. Omit it to use the default location or provide a valid path to where report files should be placed (absolute or relative).",
                        reports
                    );
                    help::emit_report_dir_msg();
                    exit(1);
                }
            }
        };

        // config folder
        if let Some(config_folder) = find_arg_value(&mut pargs, vec!["--config"]) {
            // en empty value doesn't make sense in this context
            if config_folder.trim().is_empty() {
                eprintln!(
                    "STACKMUNCHER CONFIG ERROR: param `--config` has no value. Omit it to use the default location or provide a valid path to where your encryption keys and config details should be stored (absolute or relative).",
                );
                help::emit_usage_msg();
                exit(1);
            }

            match PathBuf::from_str(&config_folder) {
                Ok(v) => app_args.config = Some(v),
                Err(_) => {
                    eprintln!(
                        "STACKMUNCHER CONFIG ERROR: `{}` is not a valid path for `--config`. Omit it to use the default location or provide a valid path to where your encryption keys and config details should be stored (absolute or relative)",
                        config_folder
                    );
                    help::emit_usage_msg();
                    exit(1);
                }
            }
        };

        // logging level
        if let Some(log) = find_arg_value(&mut pargs, vec!["--log", "-l"]) {
            app_args.log = Some(string_to_log_level(log));
        };

        // check for any leftovers or unrecognized params
        let leftovers = pargs.finish();
        if !leftovers.is_empty() {
            eprintln!("STACKMUNCHER CONFIG ERROR: {:?} params are not recognized.", leftovers);
            help::emit_usage_msg();
            exit(1);
        }

        app_args
    }
}

/// Returns the value for the first matching param name. Prints an error and exists if the parser fails.
fn find_arg_value(pargs: &mut pico_args::Arguments, arg_names: Vec<&'static str>) -> Option<String> {
    //

    for arg_name in arg_names {
        // try to read the setting and inform the user if there is an error
        let value: Option<String> = match pargs.opt_value_from_str(arg_name) {
            Ok(v) => v,
            Err(_) => {
                eprintln!(
                    "STACKMUNCHER CONFIG ERROR: invalid or missing value for `{}`. Add \"\" to reset this setting.",
                    arg_name
                );
                help::emit_usage_msg();
                exit(1);
            }
        };

        // return the first value encountered
        if let Some(v) = value {
            return Some(v.trim().to_owned());
        }
    }

    // no value was found
    None
}

/// Converts case insensitive level as String into Enum, defaults to INFO
fn string_to_log_level(s: String) -> tracing::Level {
    match s.trim().to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        _ => {
            // the user specified something, but is it not a valid value
            // it may still be better off to complete the job with some extended logging, so defaulting to INFO
            eprintln!("STACKMUNCHER CONFIG ERROR. `{}` is an invalid logging output level for --log option. Omit that param to get error messages only or use any of `info | warn | debug | trace` for more verbose output.", s);
            help::emit_usage_msg();
            exit(1);
        }
    }
}
