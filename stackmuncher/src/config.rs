use path_absolutize::{self, Absolutize};
use regex::Regex;
use stackmuncher_lib::{config::Config, git::check_git_version, utils::hash_str_sha1};
use std::path::Path;
use std::process::exit;

pub(crate) const CMD_ARGS: &'static str = "Optional CLI params: [--rules path_to_folder_with_alternative_code_rules], \
[--project path_to_project_to_be_analyzed] defaults to the current directory, \
[--report path_to_reports_folder] defaults to a platform specific location, \
[--log error|warn|info|debug|trace] defaults to `error`.";

/// Inits values from ENV vars and the command line arguments
pub(crate) async fn new_config() -> Config {
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
                .expect(&format!(
                    "Cannot determine the location of the exe file from: {}",
                    v.to_string_lossy()
                ))
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

    // assume that the project_dir is the current working folder
    let project_dir = std::env::current_dir().expect("Cannot access the current directory.");

    // init the minimal config structure with the default values
    let mut config = Config {
        log_level,
        code_rules_dir,
        report_dir: Some(report_dir),
        project_dir,
        user_name: String::new(),
        repo_name: String::new(),
        git_remote_url_regex: Regex::new(Config::GIT_REMOTE_URL_REGEX).unwrap(),
    };

    // check if there were any arguments passed to override the ENV vars
    let mut args = std::env::args().peekable();
    loop {
        if let Some(arg) = args.next() {
            match arg.to_lowercase().as_str() {
                "--rules" => {
                    let code_rules_dir = Path::new(
                        args.peek()
                            .expect("`--rules` requires a path to the folder with code rules"),
                    )
                    .to_path_buf();

                    // this checks if the rules dir is present, but not its contents
                    // incomplete, may fall over later
                    let code_rules_dir = code_rules_dir
                        .absolutize()
                        .expect("Cannot convert rules dir path to absolute. Check if it looks valid and try to simplify it.")
                        .to_path_buf();
                    if !code_rules_dir.is_dir() {
                        eprintln!(
                            "STACKMUNCHER CONFIG ERROR: Invalid code rules folder `{}`.",
                            code_rules_dir.to_string_lossy()
                        );
                        emit_code_rules_msg();
                        exit(1);
                    }
                    // it's a valid path
                    config.code_rules_dir = code_rules_dir;
                }

                "--project" => {
                    let project_dir = Path::new(
                        args.peek()
                            .expect("--project requires a path to the root of the project to be analyzed"),
                    )
                    .absolutize()
                    .expect(
                        "Cannot convert project dir path to absolute. Check if it looks valid and try to simplify it.",
                    )
                    .to_path_buf();

                    // this only tests the presence of the project dir, not .git inside it
                    // incomplete, may fall over later
                    if !project_dir.exists() {
                        eprintln!(
                            "STACKMUNCHER CONFIG ERROR: Cannot access the project folder at {}",
                            project_dir.to_string_lossy()
                        );
                        emit_usage_msg();
                        exit(1);
                    }
                    if !project_dir.is_dir() {
                        eprintln!(
                            "STACKMUNCHER CONFIG ERROR: The path to project folder is not a folder: {}",
                            project_dir.to_string_lossy()
                        );
                        emit_usage_msg();
                        exit(1);
                    }
                    // it's a valid path
                    config.project_dir = project_dir;
                }
                "--report" => {
                    config.report_dir = Some(
                        Path::new(
                            args.peek()
                                .expect("--report requires a path to a writable folder where to store the reports"),
                        )
                        .to_path_buf(),
                    )
                }
                "--log" => {
                    config.log_level =
                        string_to_log_level(args.peek().expect("--log requires a valid logging level").into())
                }
                _ => { //do nothing
                }
            };
        } else {
            break;
        }
    }

    // -------------- RULES DIRECTORY CHECKS -------------------

    // this checks if the rules dir is present, but not its contents
    // incomplete, may fall over later
    if !config.code_rules_dir.exists() {
        eprintln!("STACKMUNCHER CONFIG ERROR: Cannot find StackMuncher code parsing rules.");
        emit_code_rules_msg();
        exit(1);
    }

    // check if the sub-folders of stm_rules are present
    let file_type_dir = config.code_rules_dir.join(Config::RULES_SUBFOLDER_FILE_TYPES);
    if !file_type_dir.exists() {
        let file_type_dir = file_type_dir
            .absolutize()
            .expect("Cannot convert rules / file_types dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find file type rules folder {}",
            file_type_dir.to_string_lossy()
        );
        emit_code_rules_msg();
        std::process::exit(1);
    }

    // check if the munchers sub-folder is present
    let muncher_dir = config.code_rules_dir.join(Config::RULES_SUBFOLDER_MUNCHERS);
    if !muncher_dir.exists() {
        let muncher_dir = muncher_dir
            .absolutize()
            .expect("Cannot convert rules / munchers dir path to absolute. It's a bug.")
            .to_path_buf();

        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot find rules directory for munchers in {}",
            muncher_dir.to_string_lossy()
        );
        emit_code_rules_msg();
        std::process::exit(1);
    }

    // -------------- PROJECT DIRECTORY CHECKS -------------------

    // the project dir at this point is either a tested param from the CLI or the current dir
    // a full-trust app is guaranteed access to the current dir
    // a restricted app would need to test if the dir is actually accessible, but it may fail over even earlier when it tried to get the current dir name

    // check if there is .git subfolder in the project dir
    let git_path = config.project_dir.join(".git");
    if !git_path.is_dir() {
        eprintln!(
            "STACKMUNCHER CONFIG ERROR: No Git repository found in the project folder {}",
            config.project_dir.to_string_lossy()
        );
        emit_usage_msg();
        exit(1);
    }

    // check if GIT is installed
    // this check will change to using the git supplied as part of STM package
    if let Err(_e) = check_git_version(&config.project_dir).await {
        eprintln!(
            "STACKMUNCHER CONFIG ERROR: Cannot launch Git from {} folder. Is it installed on this machine?",
            config.project_dir.to_string_lossy()
        );
        emit_usage_msg();
        exit(1);
    };

    // individual project reports are grouped in their own folders - build that path here
    // this can be relative or absolute, which should be converted into absolute in a canonical form as a single folder name
    // e.g. /var/tmp/stackmuncher/reports/home_ubuntu_projects_some_project_name_1_6bdf08b3 were the last part is a canonical project name built
    // out of the absolute project path and its own hash
    // the hash is included in the path for ease of search and matching with the report contents because the report itself does not contain any project or user
    // identifiable info
    let project_dir = &config.project_dir;
    let absolute_project_path = if project_dir.is_absolute() {
        project_dir.to_string_lossy().to_string()
    } else {
        // join the current working folder with the relative path to the project
        std::env::current_dir()
            .expect("Cannot get the current dir. It's a bug.")
            .join(project_dir)
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
    let report_dir = config
        .report_dir
        .as_ref()
        .expect("Cannot unwrap the report dir. It's a bug.")
        .join(canonical_project_name);

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
    config.report_dir = Some(report_dir);

    config
}

/// Converts case insensitive level as String into Enum, defaults to INFO
fn string_to_log_level(s: String) -> tracing::Level {
    match s.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        _ => {
            // the user specified something, but is it not a valid value
            // it may still be better off to complete the job with some extended logging, so defaulting to INFO
            println!("STACKMUNCHER CONFIG ERROR. Invalid tracing level. Use TRACE, DEBUG, WARN or ERROR. Choosing INFO level.");
            return tracing::Level::INFO;
        }
    }
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

/// Prints out a standard multi-line message on how to use the app and where to find more info
pub(crate) fn emit_usage_msg() {
    println!("Launch StackMuncher app from the root folder of your project with a Git repository in .git subfolder.");
    println!("The app will analyze the Git repo and produce a report.");
    println!("");
    println!("{}", CMD_ARGS);
    println!("");
    emit_support_msg();
}

/// Prints out a standard multi-line message on where to find more info
pub(crate) fn emit_support_msg() {
    println!("Source code and usage instructions: https://github.com/stackmuncher/stm");
    println!("Bug reports and questions: https://github.com/stackmuncher/stm/issues or mailto:info@stackmuncher.com");
}

/// Prints out info on where the rules are expected
pub(crate) fn emit_code_rules_msg() {
    println!("");
    if cfg!(debug_assertions) {
        println!("The default location for StackMuncher code rules in DEBUGGING MODE is `{}` sub-folder of the current working directory.", Config::RULES_FOLDER_NAME_DEBUG);
    } else if cfg!(target_os = "linux") {
        println!(
            "The default location for StackMuncher code rules on Linux is `{}` folder.",
            Config::RULES_FOLDER_NAME_LINUX
        );
    } else if cfg!(target_os = "windows") {
        println!(
            "The default location for StackMuncher code rules on Windows is `{}` folder placed next stackmuncher.exe.",
            Config::RULES_FOLDER_NAME_WIN
        );
    }
    println!("To specify a different location use `--rules` param followed by a relative or absolute path to the rules folder.");
    println!("The latest copy of the rules can be downloaded from https://github.com/stackmuncher/stm repo or https://distro.stackmuncher.com/stm_rules.zip");
    println!("");
    emit_support_msg();
}
