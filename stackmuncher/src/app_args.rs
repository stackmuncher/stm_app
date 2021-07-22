use crate::help;
use pico_args;
use std::str::FromStr;
use std::{path::PathBuf, process::exit};

/// List of valid app commands
pub(crate) enum AppArgCommands {
    /// The default value
    Munch,
    Help,
    ViewReports,
    MakeAnon,
    DeleteProfile,
}

/// A container for user-provided CLI args or their defaults if none was supplied.
pub(crate) struct AppArgs {
    pub command: AppArgCommands,
    pub no_update: bool,
    pub primary_email: Option<String>,
    pub emails: Option<Vec<String>>,
    pub public_name: Option<String>,
    pub public_contact: Option<String>,
    pub project: Option<PathBuf>,
    pub rules: Option<PathBuf>,
    pub reports: Option<PathBuf>,
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
            "viewreports" | "view-reports" | "view_reports" | "viewreport" | "view-report" | "view_report" => {
                Self::ViewReports
            }
            "makeanon" | "make-anon" | "make_anon" => Self::MakeAnon,
            "deleteprofile" | "delete-profile" | "delete_profile" | "delete" => Self::DeleteProfile,
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
    /// Read the CLI params and replace the default values in `self`.
    pub(crate) fn read_params() -> Self {
        let mut app_args = AppArgs {
            command: AppArgCommands::Munch,
            no_update: false,
            primary_email: None,
            emails: None,
            public_name: None,
            public_contact: None,
            project: None,
            rules: None,
            reports: None,
            log: None,
        };

        let mut pargs = pico_args::Arguments::from_env();

        // help has a higher priority and should be handled separately
        if pargs.contains(["-h", "--help"]) {
            help::emit_welcome_msg();
            std::process::exit(0);
        }

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

        app_args.no_update = pargs.contains("--no_update") || pargs.contains("--no-update") || pargs.contains("--noupdate");

        if let Some(primary_email) = find_arg_value(&mut pargs, vec!["--primary_email", "--primary-email", "--primaryemail"])
        {
            app_args.primary_email = Some(primary_email);
        };

        // emails are a comma-separated list and should be cleaned up from various forms like
        // a@gmail.com,,d@gmail.com,
        // "a@gmail.com d@gmail.com"
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

        if let Some(public_name) = find_arg_value(&mut pargs, vec!["--public_name", "--public-name", "--publicname"]) {
            app_args.public_name = Some(public_name);
        };

        if let Some(public_contact) =
            find_arg_value(&mut pargs, vec!["--public_contact", "--public-contact", "--public_contact"])
        {
            app_args.public_contact = Some(public_contact);
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

        // rules folder
        if let Some(rules) = find_arg_value(&mut pargs, vec!["--rules"]) {
            // en empty value doesn't make sense in this context
            if rules.trim().is_empty() {
                eprintln!(
                    "STACKMUNCHER CONFIG ERROR: param `--rules` has no value. Omit it to use the default set or provide a valid path to where the rules files are located (absolute or relative).",
                );
                help::emit_usage_msg();
                exit(1);
            }

            match PathBuf::from_str(&rules) {
                Ok(v) => app_args.rules = Some(v),
                Err(_) => {
                    eprintln!(
                        "STACKMUNCHER CONFIG ERROR: `{}` is not a valid path for `--rules`. Omit it to use the default set or provide a valid path to where the rules files are located (absolute or relative).",
                        rules
                    );
                    help::emit_report_dir_msg();
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
                help::emit_usage_msg();
                exit(1);
            }

            match PathBuf::from_str(&reports) {
                Ok(v) => app_args.reports = Some(v),
                Err(_) => {
                    eprintln!(
                        "STACKMUNCHER CONFIG ERROR: `{}` is not a valid path for `--reports`. Omit it to use the default location or provide a valid path to where report files should be placed (absolute or relative).",
                        reports
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
                eprintln!("STACKMUNCHER CONFIG ERROR: invalid value for `{}`", arg_name);
                help::emit_usage_msg();
                exit(1);
            }
        };

        // return the first value encountered
        if value.is_some() {
            return value;
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
