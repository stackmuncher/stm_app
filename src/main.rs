use std::error::Error;
use std::path::Path;
use tracing::info;

mod lib;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // get input params
    let params = lib::Params::new();

    tracing_subscriber::fmt()
        .with_max_level(params.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = lib::code_rules::CodeRules::new(&params.config_file_path);

    let report = lib::process_project(
        &mut code_rules,
        &params.project_dir_path,
        &params.user_name,
        &params.repo_name,
    )
    .await?;

    report.save_as_local_file(&params.report_file_name);

    info!("Done in {}ms", instant.elapsed().as_millis());

    Ok(())
}

impl lib::Params {
    /// Inits values from ENV vars and the command line arguments
    pub fn new() -> Self {
        const ENV_LOG_LEVEL: &'static str = "STACK_MUNCHER_LOG_LEVEL";
        const ENV_PROJECT_PATH: &'static str = "STACK_MUNCHER_PROJECT_PATH";
        const ENV_REPORT_NAME: &'static str = "STACK_MUNCHER_REPORT_NAME";
        const ERR_INVALID_PARAMS: &'static str =
            "Available params: -c config_path -p project_path -r report_path -l log_level(trace,error)";

        // init the structure from env vars
        let mut params = lib::Params {
            config_file_path: std::env::var(lib::ENV_CONF_PATH).unwrap_or_default(),
            log_level: lib::Params::string_to_log_level(std::env::var(ENV_LOG_LEVEL).unwrap_or_default()),
            project_dir_path: std::env::var(ENV_PROJECT_PATH).unwrap_or_default(),
            report_file_name: std::env::var(ENV_REPORT_NAME).unwrap_or_default(),
            user_name: String::new(),
            repo_name: String::new(),
        };

        // check if there were any arguments passed to override the ENV vars
        let mut args = std::env::args().peekable();
        loop {
            if let Some(arg) = args.next() {
                match arg.to_lowercase().as_str() {
                    "-c" => params.config_file_path = args.peek().expect(ERR_INVALID_PARAMS).into(),
                    "-p" => params.project_dir_path = args.peek().expect(ERR_INVALID_PARAMS).into(),
                    "-r" => params.report_file_name = args.peek().expect(ERR_INVALID_PARAMS).into(),
                    "-l" => {
                        params.log_level =
                            lib::Params::string_to_log_level(args.peek().expect(ERR_INVALID_PARAMS).into())
                    }
                    _ => { //do nothing
                    }
                };
            } else {
                break;
            }
        }

        // check if the params are correct
        if !Path::new(&params.config_file_path).is_file() {
            println!("Invalid config file location: {}", params.config_file_path);
            panic!();
        }

        if !Path::new(&params.project_dir_path).is_dir() {
            println!("Invalid project dir location: {}", params.project_dir_path);
            panic!();
        }

        // generate a random report file name based on the current timestamp if none was provided
        if params.report_file_name.is_empty() {
            params.report_file_name = [chrono::Utc::now().timestamp().to_string().as_str(), ".json"].concat();
        }
        // check if the report file can be created
        if let Err(e) = std::fs::File::create(&params.report_file_name) {
            println! {"Invalid report file name: {} due to {}.", params.report_file_name, e};
            panic!();
        }

        params
    }

    /// Converts case insensitive level as String into Enum, defaults to INFO
    pub fn string_to_log_level(s: String) -> tracing::Level {
        match s.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "error" => tracing::Level::DEBUG,
            "warn" => tracing::Level::WARN,
            _ => tracing::Level::INFO,
        }
    }
}
