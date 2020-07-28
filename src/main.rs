use regex::Regex;
use serde_json;
use std::error::Error;
use std::fs;
//use std::io::{self, prelude::*, BufReader};
use std::path::Path;

mod structures;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Stack munching started ...");

    let dir = "/home/ubuntu/rust/cm_repos/eShopOnWeb/".to_string();

    // load config
    let conf = Path::new("/home/ubuntu/rust/stackmuncher/assets/config.json");
    let conf = fs::File::open(conf).expect("Cannot read config file");
    let mut conf: structures::Config = serde_json::from_reader(conf).expect("Cannot parse config file");

    // get list of files
    let mut files = get_file_names_recursively(Path::new(dir.as_str()));

    // remove .git/ files from the list
    let re = Regex::new(r"\.git/").unwrap();
    files.retain(|f| !re.is_match(f.as_str()));

    // pre-compile all file name regex

    for mut f in conf.files.iter_mut() {
        f.name_regex = Some(Regex::new(f.name.as_str()).expect("Regex compilation failed"));
    }

    let mut processed_files: Vec<String> = Vec::new();

    // loop through all the files and process them one by one
    for file_path in &files {
        // loop through the rules and process the file if it's a match
        for file_rules in &conf.files {
            if file_rules.name_regex.as_ref().unwrap().is_match(file_path.as_str()) {
                if let Ok(_) = process_file(&file_path, &file_rules) {
                    processed_files.push(file_path.clone());
                }
            }
        }
    }

    // discard processed files
    files.retain(|f| !processed_files.contains(&f));

    // output the list
    println!("\nSkipped files:\n");
    for s in &files {
         println!("{}", s);
    }
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

// fn get_asset_lines(asset_path: String) -> Vec<String> {
//     let mut lines: Vec<String> = Vec::new();

//     let file = fs::File::open(asset_path.as_str()).unwrap();
//     let reader = BufReader::new(file);

//     for line in reader.lines() {
//         lines.push(line.unwrap());
//     }

//     lines
// }

fn process_file(file_path: &String, rules: &structures::FileRules ) -> Result<(), String> {

    println!("{}: {}", rules.name, file_path);

    Ok(())
}
