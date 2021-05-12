use regex::Regex;
use stackmuncher_lib::{config::Config, utils::hash_str_sha1};
use std::path::Path;

/// Inits values from ENV vars and the command line arguments
pub(crate) fn new_config() -> Config {
    const CMD_ARGS: &'static str =
        "Optional CLI params: [--rules code_rules_dir] defaults to a platform-specific location, \
    [--project project_path] defaults to the current dir, \
    [--report report_dir_path] defaults to a platform specific location, \
    [--log log_level] defaults to warn.";

    // Output it every time for now. Review and remove later when it's better documented.
    println!("{}", CMD_ARGS);

    // look for the rules in the current working dir if in debug mode
    // otherwise default to a platform-specific location
    // this can be overridden by `--rules` CLI param
    let (code_rules_dir, report_dir, log_level) = if cfg!(debug_assertions) {
        println!("debug");
        (
            Path::new(Config::RULES_FOLDER_NAME_DEBUG).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_DEBUG).to_path_buf(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        println!("linux");
        (
            Path::new(Config::RULES_FOLDER_NAME_LINUX).to_path_buf(),
            Path::new(Config::REPORT_FOLDER_NAME_LINUX).to_path_buf(),
            tracing::Level::WARN,
        )
    } else if cfg!(target_os = "windows") {
        println!("windows");
        // the easiest way to store the rules on Win is next to the executable
        let exec_dir = match std::env::current_exe() {
            Err(e) => {
                panic!("No current dir: {}", e);
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
            tracing::Level::INFO,
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
                    config.code_rules_dir = Path::new(
                        args.peek()
                            .expect("--rules requires a path to the folder with code rules"),
                    )
                    .to_path_buf()
                }

                "--project" => {
                    config.project_dir = Path::new(
                        args.peek()
                            .expect("--project requires a path to the root of the project to be analyzed"),
                    )
                    .to_path_buf()
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

    println!("Config rules dir: {}", config.code_rules_dir.to_string_lossy());
    println!("Config proj dir: {}", config.project_dir.to_string_lossy());

    // this checks if the rules dir is present, but not its contents
    // incomplete, may fall over later
    if !config.code_rules_dir.is_dir() {
        panic!(
            "Invalid path to folder with code parsing rules: {}",
            config.code_rules_dir.to_string_lossy()
        );
    }

    // this tests the presence of the project dir, but it actually needs .git inside it
    // incomplete, may fall over later
    if !config.project_dir.exists() {
        panic!("Project dir location doesn't exist: {}", config.project_dir.to_string_lossy());
    }
    if !config.project_dir.is_dir() {
        panic!("Invalid project dir location: {}", config.project_dir.to_string_lossy());
    }

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
            panic!("Invalid report directory: {}", report_dir.to_string_lossy());
        }
        // create it
        if let Err(e) = std::fs::create_dir_all(report_dir.clone()) {
            panic!(
                "Cannot create reports directory at {} due to {}",
                report_dir.to_string_lossy(),
                e
            );
        };
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
        _ => {
            panic!("Invalid tracing level. Use trace, debug, warn, error. Default level: info.");
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
