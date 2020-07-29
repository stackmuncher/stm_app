use regex::Regex;
use serde_json;
use std::error::Error;
use std::fs;
//use std::io::{self, prelude::*, BufReader};
use std::path::Path;

mod config;
mod processors;
mod report;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Stack munching started ...");

    let dir = "/home/ubuntu/rust/cm_repos/eShopOnWeb/".to_string();

    // load config
    let conf = Path::new("/home/ubuntu/rust/stackmuncher/assets/config.json");
    let conf = fs::File::open(conf).expect("Cannot read config file");
    let mut conf: config::Config = serde_json::from_reader(conf).expect("Cannot parse config file");

    // get list of files
    let mut files = get_file_names_recursively(Path::new(dir.as_str()));

    // remove .git/ files from the list
    let re = Regex::new(r"\.git/").unwrap();
    files.retain(|f| !re.is_match(f.as_str()));

    // pre-compile all file name regex

    for mut f in conf.files.iter_mut() {
        f.name_regex = Some(Regex::new(f.name.as_str()).expect("Regex compilation failed"));
    }

    // result collectors
    let mut processed_files: Vec<String> = Vec::new();
    let mut report = report::Report::new();

    // loop through all the files and process them one by one
    for file_path in &files {
        // loop through the rules and process the file if it's a match
        for file_rules in &conf.files {
            if file_rules.name_regex.as_ref().unwrap().is_match(file_path.as_str()) {
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
    println!("\n\n{}",report);

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
        println!("{} is not a dir", dir.to_str().unwrap().to_owned());
    }

    files
}
