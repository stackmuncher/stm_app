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
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        //.without_time()
        .init();

    info!("Stack munching started ...");

    let dir = "/home/ubuntu/rust/cm_repos/eShopOnWeb/".to_string();

    // load config
    let conf = Path::new("/home/ubuntu/rust/stackmuncher/assets/config.json");
    let conf = fs::File::open(conf).expect("Cannot read config file");
    let mut conf: config::Config = serde_json::from_reader(conf).expect("Cannot parse config file");

    // pre-compile regex rules for file names
    for file_rules in conf.files.iter_mut() {
        file_rules.compile_file_name_regex();
    }

    // get list of files
    let mut files = get_file_names_recursively(Path::new(dir.as_str()));

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
            if file_rules.file_names_regex.as_ref().unwrap().iter().any(|r| r.is_match(file_path.as_str())) {
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
        report.add_unprocessed_file(f);
    }

    // output the report as json
    //report.unknown_file_types.clear();
    report.unprocessed_file_names.clear();
    info!("\n\n{}", report);

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
