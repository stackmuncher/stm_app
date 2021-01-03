use stackmuncher::{
    code_rules::CodeRules,
    config::{Config, FileListType},
    git::{get_all_tree_files, get_last_commit_files},
    process_project_files,
    report::Report,
};
use std::path::Path;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // get input params
    let config = new_config();

    tracing_subscriber::fmt()
        .with_max_level(config.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = CodeRules::new(&config.code_rules_dir);

    // load the existing report
    let existing_report = Report::from_disk(&config.report_file_name);

    // we have to get the list of all tree files every time because the latest commit does not contain deleted files
    let all_tree_files = get_all_tree_files(&config.project_dir_path).await?;

    // get the list of files to process (all files in the tree)
    let files_to_process = if config.file_list_type == FileListType::FullTree || existing_report.is_none() {
        // this clone is unnecessary and can probably be avoided, but I couldn't see a quick way
        all_tree_files.clone()
    } else {
        get_last_commit_files(&config.project_dir_path, &all_tree_files).await?
    };

    // generate the report
    let report = process_project_files(
        &mut code_rules,
        &config.project_dir_path,
        &config.user_name,
        &config.repo_name,
        existing_report,
        &files_to_process,
        &all_tree_files,
    )
    .await?;

    // update the report with additional info
    let report = report
        .extract_commit_info(&config.project_dir_path, &config.git_remote_url_regex)
        .await;
    let report = report.update_list_of_tree_files(all_tree_files);

    report.save_as_local_file(&config.report_file_name);

    info!("Done in {}ms", instant.elapsed().as_millis());

    Ok(())
}

/// Inits values from ENV vars and the command line arguments
fn new_config() -> Config {
    pub const ENV_RULES_PATH: &'static str = "STACK_MUNCHER_CODERULES_DIR";
    const CMD_ARGS: &'static str =
        "Available CLI params: [--rules code_rules_dir] or use STACK_MUNCHER_CODERULES_DIR env var, \
    [--project project_path] defaults to the current dir, [--report report_path] defaults to report.json, \
    [--files all|recent] defaults for all, [--log log_level] defaults to info.";

    // Output it every time for now. Review and remove later when it's better documented.
    println!("{}", CMD_ARGS);

    // init the structure with the default values

    let mut config = Config::new(
        std::env::var(ENV_RULES_PATH).unwrap_or_default(),
        String::new(),
        String::new(),
    );

    // project_dir_path code is dodgy and may fail cross-platform with non-ASCII chars
    config.project_dir_path = std::env::current_dir()
        .expect("Cannot access the current directory.")
        .to_str()
        .unwrap_or_default()
        .to_owned();
    config.report_file_name = "stm-report.json".to_owned();

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
                    config.report_file_name = args
                        .peek()
                        .expect("--report requires a report file name with or without a path")
                        .into()
                }
                "--files" => {
                    config.file_list_type = string_to_file_list_type(
                        args.peek()
                            .expect("--files requires `all` for full tree or `recent` for the last commit")
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

    // check if the params are correct
    if !Path::new(&config.code_rules_dir).is_dir() {
        panic!("Invalid config files folder: {}", config.code_rules_dir);
    }

    if !Path::new(&config.project_dir_path).is_dir() {
        panic!("Invalid project dir location: {}", config.project_dir_path);
    }

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

/// Converts case insensitive name of the file list to an enum
fn string_to_file_list_type(s: String) -> FileListType {
    match s.to_lowercase().as_str() {
        "all" => FileListType::FullTree,
        "recent" => FileListType::LastCommit,
        _ => {
            panic!("Invalid FileListType value. Use `full` or `recent`. Default: `full`.");
        }
    }
}
