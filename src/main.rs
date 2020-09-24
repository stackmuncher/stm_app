use std::error::Error;
use std::path::Path;
use tracing::info;

mod lib;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // get input params
    let params = lib::Config::new();

    tracing_subscriber::fmt()
        .with_max_level(params.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    let instant = std::time::Instant::now();

    // load code rules
    let mut code_rules = lib::code_rules::CodeRules::new(&params.code_rules_dir);

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

impl lib::Config {
    /// Inits values from ENV vars and the command line arguments
    pub fn new() -> Self {
        pub const ENV_RULES_PATH: &'static str = "STACK_MUNCHER_CODERULES_DIR";
        const CMD_ARGS: &'static str = "Available CLI params: [-c code_rules_dir] or use STACK_MUNCHER_CODERULES_DIR env var, [-p project_path] defaults to the current dir, [-r report_path] defaults to report.json, [-l log_level] defaults to info.";

        // Output it every time for now. Review and remove later when it's better documented.
        println!("{}", CMD_ARGS);

        // init the structure with the default values
        let mut config = lib::Config {
            code_rules_dir: std::env::var(ENV_RULES_PATH).unwrap_or_default(),
            log_level: tracing::Level::INFO,
            // project_dir_path code is dodgy and may fail cross-platform with non-ASCII chars
            project_dir_path: std::env::current_dir()
                .expect("Cannot access the current directory.")
                .to_str()
                .unwrap_or_default()
                .to_owned(),
            report_file_name: "stm-report.json".to_owned(),
            user_name: String::new(),
            repo_name: String::new(),
        };

        // check if there were any arguments passed to override the ENV vars
        let mut args = std::env::args().peekable();
        loop {
            if let Some(arg) = args.next() {
                match arg.to_lowercase().as_str() {
                    "-c" => {
                        config.code_rules_dir = args
                            .peek()
                            .expect("-c requires a path to the folder with code rules")
                            .into()
                    }

                    "-p" => {
                        config.project_dir_path = args
                            .peek()
                            .expect("-p requires a path to the root of the project to be analyzed")
                            .into()
                    }

                    "-r" => {
                        config.report_file_name = args
                            .peek()
                            .expect("-r requires a report file name with or without a path")
                            .into()
                    }
                    "-l" => {
                        config.log_level = lib::Config::string_to_log_level(
                            args.peek().expect("-l requires a valid logging level").into(),
                        )
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

        // check if the report file can be created
        if let Err(e) = std::fs::File::create(&config.report_file_name) {
            panic! {"Invalid report file name: {} due to {}.", config.report_file_name, e};
        }

        config
    }

    /// Converts case insensitive level as String into Enum, defaults to INFO
    pub fn string_to_log_level(s: String) -> tracing::Level {
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
}
