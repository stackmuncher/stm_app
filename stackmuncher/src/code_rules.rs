use super::file_type::FileType;
use super::muncher::Muncher;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use tracing::{info, trace};

#[derive(Debug, Clone)]
pub struct CodeRules {
    /// All file types are added at init time
    pub files_types: BTreeMap<String, FileType>,

    /// Munchers are loaded on-demand
    pub munchers: BTreeMap<String, Option<Muncher>>,

    /// Directory path where muncher files are stored
    muncher_files_dir: String,

    /// A compiled regex for fetching a file extension from the full
    /// file path, including directories
    pub file_ext_regex: Regex,

    /// A compiled regex for extracting the file name without path or extension.
    /// E.g. `/dir/dir/cs.json` -> `cs`
    pub file_name_as_ext_regex: Regex,

    /// Contains names of newly added munchers to assist merging multiple instances
    /// of CodeRules for parallel processing.
    pub new_munchers: Option<HashSet<String>>,
}

impl CodeRules {
    /// Create a new instance from a a list of file-type files at `file_type_dir`
    /// File-type rules are loaded upfront, munchers are loaded dynamically
    pub fn new(config_dir: &String) -> Self {
        // assume that the rule-files live in subfolders of the config dir
        let file_type_dir = [config_dir.trim_end_matches("/").to_owned(), "/file-types".to_owned()].concat();
        let muncher_dir = [config_dir.trim_end_matches("/").to_owned(), "/munchers".to_owned()].concat();

        info!(
            "Loading config file for file_types from {} and munchers from {}",
            file_type_dir, muncher_dir
        );

        // get the list of files from the target folder
        let dir = match fs::read_dir(&file_type_dir) {
            Err(e) => {
                panic!("Cannot load file rules from {} with {}. Aborting.", file_type_dir, e);
            }
            Ok(v) => v,
        };

        // collect relevant file names, ignore the rest
        let mut file_names: Vec<String> = Vec::new();
        for f in dir {
            if let Ok(entry) = f {
                let path = entry.path();
                let f_n = path.to_str().expect("Invalid file-type file name");
                if f_n.ends_with(".json") {
                    file_names.push(f_n.to_owned());
                }
            }
        }

        info!("FileTypes files found: {}", file_names.len());

        // prepare the output collector
        let mut code_rules = CodeRules {
            files_types: BTreeMap::new(),
            munchers: BTreeMap::new(),
            muncher_files_dir: muncher_dir.clone(),
            file_ext_regex: Regex::new("\\.[a-zA-Z0-1_]+$").unwrap(),
            file_name_as_ext_regex: Regex::new("[a-zA-Z0-1_]+\\.json$").unwrap(),
            new_munchers: None,
        };

        // load the files one by one
        for file in file_names {
            if let Some(ft) = FileType::new(&file, &code_rules.file_name_as_ext_regex) {
                code_rules.files_types.insert(ft.file_ext.clone(), ft);
            }
        }

        code_rules
    }

    /// Return the right muncher for the file extension extracted from the full path.
    pub fn get_muncher(&mut self, file_path: &String) -> Option<&Muncher> {
        // try to get file extension
        if let Some(ext) = self.file_ext_regex.find(&file_path) {
            // try to find a file_type match for the ext
            if let Some(file_type) = self.files_types.get(ext.as_str()) {
                // try to find a matching muncher
                if let Some(muncher_name) = file_type.get_muncher_name(file_path) {
                    // load the muncher from its file on the first use
                    if !self.munchers.contains_key(&muncher_name) {
                        let muncher_file_name = [
                            self.muncher_files_dir.clone(),
                            "/".to_owned(),
                            muncher_name.clone(),
                            ".json".to_owned(),
                        ]
                        .concat();

                        trace!("Loading muncher {} for the 1st time", muncher_file_name);

                        // Insert None if the muncher could not be loaded so that it doesn't try to load it again
                        self.munchers
                            .insert(muncher_name.clone(), Muncher::new(&muncher_file_name, &muncher_name));
                        // indicate to the caller that there were new munchers added so they can be shared with other threads
                        if self.new_munchers.is_none() {
                            self.new_munchers = Some(HashSet::new());
                        }
                        self.new_munchers.as_mut().unwrap().insert(muncher_name.clone());
                    }

                    return self.munchers.get(&muncher_name).unwrap().as_ref();
                }
            }
        }

        None
    }
}
