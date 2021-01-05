use stackmuncher::{
    code_rules::CodeRules,
    config::{Config, FileListType},
    git::{get_all_tree_files, get_last_commit_files},
    report::Report,
    utils::hash_str_sha1,
};
use std::path::Path;
use tracing::{error, info};

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

    // Reports are saved into .git/stm_reports folder. It should be a safe per-project location.
    // GIT ignores extra folders and they don't get checked into the repo. If the project is cloned to a different
    // location the report would have to be regenerated.
    let report_dir = Path::new(&config.project_dir_path)
        .join(Config::GIT_FOLDER_NAME)
        .join(Config::REPORT_FOLDER_NAME);

    // create the reports folder if it doesn't exist
    if !report_dir.exists() {
        if let Err(e) = std::fs::create_dir(report_dir.clone()) {
            error!(
                "Cannot create reports folder at {} due to {}",
                report_dir.to_string_lossy(),
                e
            );
            panic!();
        };
        info!("Created reports folder at {}", report_dir.to_string_lossy());
    }

    // load a previously generated report if it exists
    let project_report_filename = report_dir
        .join([Config::PROJECT_REPORT_FILE_NAME, Config::REPORT_FILE_EXTENSION].concat())
        .as_os_str()
        .to_string_lossy()
        .to_string();
    let existing_report = Report::from_disk(&project_report_filename);

    // we have to get the list of all tree files every time because the latest commit does not contain deleted files
    let all_tree_files = get_all_tree_files(&config.project_dir_path, None).await?;

    // get the list of files to process (all files in the tree)
    let files_to_process = if config.file_list_type == FileListType::FullTree || existing_report.is_none() {
        // this clone is unnecessary and can probably be avoided, but I couldn't see a quick way
        all_tree_files.clone()
    } else {
        get_last_commit_files(&config.project_dir_path, &all_tree_files).await?
    };

    // generate the report
    let report = Report::process_project_files(
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
        .extract_commit_history(&config.project_dir_path, &config.git_remote_url_regex)
        .await;
    let report = report.update_list_of_tree_files(all_tree_files);

    report.save_as_local_file(&project_report_filename);

    // check if there are multiple contributors and generate individual reports
    if let Some(contributors) = &report.contributors {
        info!("Project report done in {}ms", instant.elapsed().as_millis());

        // skip this step if there is only one contributor
        if contributors.len() < 2 {
            return Ok(());
        }

        for contributor in contributors {
            let contributor_instant = std::time::Instant::now();
            // load the previous contributor report, if any
            let contributor_hash = hash_str_sha1(contributor.git_identity.as_str());
            let contributor_report_filename = report_dir
                .join(
                    [
                        Config::CONTRIBUTOR_REPORT_FILE_NAME,
                        contributor_hash.as_str(),
                        Config::REPORT_FILE_EXTENSION,
                    ]
                    .concat(),
                )
                .as_os_str()
                .to_string_lossy()
                .to_string();
            let old_report = Report::from_disk(&contributor_report_filename);

            let contributor_report = report
                .process_contributor(
                    &mut code_rules,
                    &config.project_dir_path,
                    &contributor.git_identity,
                    old_report,
                    contributor,
                )
                .await?;

            contributor_report.save_as_local_file(&contributor_report_filename);

            info!(
                "Contributor report for {} done in {}ms",
                contributor.git_identity,
                contributor_instant.elapsed().as_millis()
            );
        }
    }
    info!("Project processed in {}ms", instant.elapsed().as_millis());
    Ok(())
}

/// Inits values from ENV vars and the command line arguments
fn new_config() -> Config {
    pub const ENV_RULES_PATH: &'static str = "STACK_MUNCHER_CODERULES_DIR";
    const CMD_ARGS: &'static str =
        "Available CLI params: [--rules code_rules_dir] or use STACK_MUNCHER_CODERULES_DIR env var, \
    [--project project_path] defaults to the current dir, \
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
