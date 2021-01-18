use stackmuncher::{
    code_rules::CodeRules, config::Config, git::get_local_git_identities, report::Report, utils::hash_str_sha1,
};
use std::path::Path;
use tracing::{debug, error, info, warn};

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
    let cached_project_report = Report::from_disk(&project_report_filename);

    let project_report = match Report::process_project(
        &mut code_rules,
        &config.project_dir_path,
        &config.user_name,
        &config.repo_name,
        cached_project_report,
        &config.git_remote_url_regex,
        None,
    )
    .await?
    {
        None => {
            // there were no changes since the previous report - it can be reused as-is
            info!("Done in {}ms", instant.elapsed().as_millis());
            return Ok(());
        }
        Some(v) => v,
    };

    project_report.save_as_local_file(&project_report_filename);
    info!("Project report done in {}ms", instant.elapsed().as_millis());

    // get the list of user identities for processing their contributions individually
    let git_identities = get_local_git_identities(&config.project_dir_path).await?;
    if git_identities.is_empty() {
        warn!("No git identity found. Individual contributions will not be processed. Use `git config set --global user.email=<your email>` before the next run.");
        eprintln!(
            "Git user details are not set. Use `git config set --global user.email=<your email>` before the next run."
        );
        return Err(());
    }

    // check if there are multiple contributors and generate individual reports
    if let Some(contributors) = &project_report.contributors {
        let last_commit_author = project_report.last_commit_author.as_ref().unwrap().clone();

        for contributor in contributors {
            // only process a single contributor of the latest commit if it's a single commit report update
            if project_report.is_single_commit && contributor.git_identity != last_commit_author {
                debug!("Contributor {} skipped / single commit", contributor.git_identity);
                continue;
            } else if !git_identities.contains(&contributor.git_identity.trim().to_lowercase()) {
                // only process known local identities if it's not a single commit
                debug!(
                    "Contributor {} skipped / not a local identity",
                    contributor.git_identity
                );
                continue;
            }

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

            let contributor_report = project_report
                .process_contributor(
                    &mut code_rules,
                    &config.project_dir_path,
                    &config.repo_name,
                    Report::from_disk(&contributor_report_filename),
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
    info!("repo processed in {}ms", instant.elapsed().as_millis());
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
