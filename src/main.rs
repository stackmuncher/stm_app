use regex::Regex;
use serde_json;
use std::error::Error;
use std::fs;
use std::path::Path;
use tracing::{error, info};

mod config;
mod processors;
mod report;

fn main() -> Result<(), Box<dyn Error>> {
    // get input params
    let params = Params::new();

    tracing_subscriber::fmt()
        .with_max_level(params.log_level)
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    // load config
    let conf = fs::File::open(params.config_file_path).expect("Cannot read config file");
    let mut conf: config::Config = serde_json::from_reader(conf).expect("Cannot parse config file");

    // pre-compile regex rules for file names
    for file_rules in conf.files.iter_mut() {
        file_rules.compile_file_name_regex();
    }

    // get list of files
    let mut files = get_file_names_recursively(Path::new(params.project_dir_path.as_str()));

    // remove .git/ files from the list
    let re = Regex::new(r"\.git/").unwrap();
    files.retain(|f| !re.is_match(f.as_str()));

    // result collectors
    let mut processed_files: Vec<String> = Vec::new();
    let mut report = report::Report::new();

    // loop through all the files and process them one by one
    for file_path in &files {
        // loop through the rules and process the file if it's a match
        for file_rules in &mut conf.files {
            // &mut conf.files is required to do this JIT compilation
            file_rules.compile_other_regex();

            // there can be multiple patterns per rule - loop through the list with the closure
            if file_rules
                .file_names_regex
                .as_ref()
                .unwrap()
                .iter()
                .any(|r| r.is_match(file_path.as_str()))
            {
                if let Ok(tech) = processors::process_file(&file_path, &file_rules) {
                    processed_files.push(file_path.clone());
                    report.add_tech_record(tech);
                }
            }
        }
    }

    // discard processed files
    files.retain(|f| !processed_files.contains(&f));

    // log unprocessed files in the report
    for f in &files {
        report.add_unprocessed_file(f, &params.project_dir_path);
    }

    // output the report as json
    //report.unknown_file_types.clear();
    //report.unprocessed_file_names.clear();
    //info!("\n\n{}", report);

    report.save_as_local_file(&params.report_file_name);

    Ok(())
}

fn get_file_names_recursively(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();

    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                let mut f = get_file_names_recursively(&path);
                files.append(&mut f);
            } else if path.is_file() {
                files.push(entry.path().to_str().unwrap().to_owned());
            }
        }
    } else {
        error!(
            "get_file_names_recursively: {} is not a dir",
            dir.to_str().unwrap().to_owned()
        );
    }

    files
}

struct Params {
    /// Full path to the config file. Absolute or relative to the working dir.
    config_file_path: String,
    log_level: tracing::Level,
    /// Absolute or relative path to the project directory with the files to analyze.
    project_dir_path: String,
    /// File name only. Reports are always saved in the current dir
    report_file_name: String,
}

impl Params {
    /// Inits values from ENV vars and the command line arguments
    fn new() -> Self {
        const ENV_CONF_PATH: &'static str = "STACK_MUNCHER_CONFIG_PATH";
        const ENV_LOG_LEVEL: &'static str = "STACK_MUNCHER_LOG_LEVEL";
        const ENV_PROJECT_PATH: &'static str = "STACK_MUNCHER_PROJECT_PATH";
        const ENV_REPORT_NAME: &'static str = "STACK_MUNCHER_REPORT_NAME";
        const ERR_INVALID_PARAMS: &'static str =
            "Available params: -c config_path -p project_path -r report_path -l log_level(trace,error)";

        // init the structure from env vars
        let mut params = Params {
            config_file_path: std::env::var(ENV_CONF_PATH).unwrap_or_default(),
            log_level: Params::string_to_log_level(std::env::var(ENV_LOG_LEVEL).unwrap_or_default()),
            project_dir_path: std::env::var(ENV_PROJECT_PATH).unwrap_or_default(),
            report_file_name: std::env::var(ENV_REPORT_NAME).unwrap_or_default(),
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
                        params.log_level = Params::string_to_log_level(args.peek().expect(ERR_INVALID_PARAMS).into())
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

        // generate a random file name based on the current timestamp
        if params.report_file_name.is_empty() {
            params.report_file_name = [chrono::Utc::now().timestamp().to_string().as_str(), ".json"].concat();
        }

        params
    }

    /// Converts case insensitive level as String into Enum, defaults to INFO
    fn string_to_log_level(s: String) -> tracing::Level {
        match s.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "error" => tracing::Level::DEBUG,
            "warn" => tracing::Level::WARN,
            _ => tracing::Level::INFO,
        }
    }
}
