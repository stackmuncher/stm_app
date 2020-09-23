use regex::Regex;
use serde::Deserialize;
use std::fs;
use tracing::{debug, error};

#[derive(Deserialize, Clone, Debug)]
pub struct FileTypeMatch {
    /// Some basic munching is performed anyway even if no special
    /// munching rules are provided
    pub muncher: Option<String>,
    // it has other unimplemented properties
}

#[derive(Deserialize, Clone, Debug)]
pub struct FileType {
    #[serde(default)]
    /// The value comes from the assets file name, e.g. `cs.json` will become `.cs`,
    /// including the `.`
    pub file_ext: String,
    /// A list of possible matches for the file type.
    pub matches: Option<Vec<FileTypeMatch>>,
}

impl FileType {
    /// Create a new instance from a file at `json_definition_file_path`.
    pub fn new(json_definition_file_path: &String, file_name_as_ext_regex: &Regex) -> Option<Self> {
        debug!("Loading {}", json_definition_file_path);

        // load the file definition from a json file
        let conf = match fs::File::open(json_definition_file_path) {
            Err(e) => {
                error!("Cannot read config file {} with {}", json_definition_file_path, e);
                std::process::exit(1);
            }
            Ok(v) => v,
        };

        // convert into a struct
        let mut conf: Self = match serde_json::from_reader(conf) {
            Err(e) => {
                error!("Cannot parse config file {} with {}", json_definition_file_path, e);
                return None;
            }
            Ok(v) => v,
        };

        // set the file ext from the file name
        // e.g. `/dir/dir/cs.json` -> `.cs`
        if let Some(file_ext) = file_name_as_ext_regex.find(&json_definition_file_path) {
            let file_ext = file_ext.as_str();
            let file_ext = file_ext[..file_ext.len()-5].to_owned();
            conf.file_ext = [".".to_owned(), file_ext].concat();

           return Some(conf);
        }

        error!("Invalid config file name {}", json_definition_file_path);
        None
    }

    /// Matches the file to the right muncher based on the rules inside this struct
    /// The current impl only returns the very first muncher.
    /// It will need to be expanded to add other matching params.
    pub fn get_muncher_name(&self) -> Option<String> {
        // return the first matcher in the list
        if let Some(muncher_matches) = self.matches.as_ref() {
            if let Some(muncher_name) = muncher_matches[0].muncher.as_ref() {
                return Some(muncher_name.clone());
            }
        }

        // otherwise return None
        error!("Missing a muncher for {}.", self.file_ext);
        None
    }
}
