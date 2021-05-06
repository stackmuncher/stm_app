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
        (
            Config::RULES_FOLDER_NAME_DEBUG.to_owned(),
            Config::REPORT_FOLDER_NAME_DEBUG.to_owned(),
            tracing::Level::INFO,
        )
    } else if cfg!(target_os = "linux") {
        (
            Config::RULES_FOLDER_NAME_LINUX.to_owned(),
            Config::REPORT_FOLDER_NAME_LINUX.to_owned(),
            tracing::Level::WARN,
        )
    } else {
        unimplemented!("Only linux target is supported at the moment");
    };

    // project_dir_path code is dodgy and may fail cross-platform with non-ASCII chars
    let project_dir_path = std::env::current_dir()
        .expect("Cannot access the current directory.")
        .to_string_lossy()
        .to_string();

    // init the minimal config structure with the default values
    let mut config = Config {
        log_level,
        code_rules_dir,
        report_dir: Some(report_dir),
        project_dir_path,
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
                    config.code_rules_dir = args
                        .peek()
                        .expect("--rules requires a path to the folder with code rules")
                        .into()
                }

                "--project" => {
                    config.project_dir_path = args
                        .peek()
                        .expect("--project requires a path to the root of the project to be analyzed")
                        .into()
                }
                "--report" => {
                    config.report_dir = Some(
                        args.peek()
                            .expect("--report requires a path to a writable folder where to store the reports")
                            .into(),
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

    // this checks if the rules dir is present, but not its contents
    // incomplete, may fall over later
    if config.code_rules_dir.is_empty() {
        panic!("Path to files with code parsing rules was not specified.");
    }
    if !Path::new(&config.code_rules_dir).is_dir() {
        panic!(
            "Invalid path to folder with code parsing rules: {}",
            config.code_rules_dir
        );
    }

    // this tests the presence of the project dir, but it actually needs .git inside it
    // incomplete, may fall over later
    if !Path::new(&config.project_dir_path).is_dir() {
        panic!("Invalid project dir location: {}", config.project_dir_path);
    }

    // check if the reports dir is ready to receive reports
    // this is for the root report folder that can hold reports for multiple projects
    // e.g. /var/tmp/stackmuncher or /home/ubuntu/stackmuncher
    let report_dir = config
        .report_dir
        .as_ref()
        .expect("Cannot unwrap the report dir. It's a bug.");
    let report_dir_path = Path::new(report_dir);
    if !report_dir_path.is_dir() {
        // is there something with this name that is not a directory?
        if report_dir_path.exists() {
            panic!("Invalid report directory: {}", report_dir);
        }
        // create it
        if let Err(e) = std::fs::create_dir_all(report_dir_path) {
            panic!("Cannot create reports directory at {} due to {}", report_dir, e);
        };
    }

    // individual project reports are grouped in their own folders - build that path here
    // this can be relative or absolute, which should be converted into absolute in a canonical form as a single folder name
    // e.g. /var/tmp/stackmuncher/reports/home_ubuntu_projects_some_project_name_1_6bdf08b3 were the last part is a canonical project name built
    // out of the absolute project path and its own hash
    // the hash is included in the path for ease of search and matching with the report contents because the report itself does not contain any project or user
    // identifiable info
    let project_dir_path = Path::new(&config.project_dir_path);
    let absolute_project_path = if project_dir_path.is_absolute() {
        project_dir_path.to_string_lossy().to_string()
    } else {
        // join the current working folder with the relative path to the project
        std::env::current_dir()
            .expect("Cannot get the current dir. It's a bug.")
            .join(project_dir_path)
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

    // append the project report subfolder name to the reports root folder
    let report_dir_path = report_dir_path.join(canonical_project_name);
    let report_dir = report_dir_path.to_string_lossy().to_string();

    // check if the project report folder exists or create it if possible
    if !report_dir_path.is_dir() {
        if report_dir_path.exists() {
            // the path exists as something else
            panic!("Invalid report directory: {}", report_dir);
        }
        // create it
        if let Err(e) = std::fs::create_dir_all(report_dir_path) {
            panic!("Cannot create reports directory at {} due to {}", report_dir, e);
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
