use regex::Regex;
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Deserialize, Clone, Debug)]
pub struct FileTypeMatch {
    /// Some basic munching is performed anyway even if no special
    /// munching rules are provided
    pub muncher: Option<String>,
    /// A regex string for matching the full file name or part of the path
    pub in_path: Option<Vec<String>>,
    /// A compiled regex for `in_path`
    #[serde(skip_deserializing)]
    pub in_path_regex: Option<Vec<Regex>>, // it has other unimplemented properties
}

/// Contains a list of code processors for a given file type as defined by the file extension.
/// E.g. `.json` can have different processors for `project.json`, `lock.json`, `yarn.json`.
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
    /// Returns `None` if the any part of the definition is invalid.
    /// * `file_name` - name of the definition file, must end with `.json`
    /// * `file_contents` - the actual json inside the file
    pub fn new(file_name: &String, contents: &str) -> Option<Self> {
        debug!("Loading {}", file_name);

        // convert into a struct
        let mut file_def = match serde_json::from_str::<Self>(contents) {
            Err(e) => {
                error!("Cannot parse file_type definition file {} due to {}", file_name, e);
                return None;
            }
            Ok(v) => v,
        };

        // set the file ext from the file name
        // e.g. `cs.json` -> `cs`
        file_def.file_ext = file_name[..file_name.len() - 5].to_lowercase();

        // compile regex on matches (FileTypeMatch)
        if let Some(file_type_matches) = file_def.matches.as_mut() {
            for file_type_match in file_type_matches {
                // check if the muncher name is missing
                let muncher_name = match file_type_match.muncher.as_ref() {
                    Some(v) => v,
                    None => {
                        error!("Missing muncher name for {}", file_def.file_ext);
                        return None;
                    }
                };
                // compile regex for the file path/name
                if let Some(in_paths) = file_type_match.in_path.as_ref() {
                    let mut in_paths_regex: Vec<Regex> = Vec::new();
                    for in_path in in_paths {
                        let compiled_regex = match Regex::new(in_path) {
                            Ok(r) => r,
                            Err(e) => {
                                // stop processing this muncher
                                error!("Failed to compile regex {} with {}", in_path, e);
                                return None;
                            }
                        };
                        in_paths_regex.push(compiled_regex);
                    }
                    file_type_match.in_path_regex = Some(in_paths_regex);
                    debug!("Compiled in_path regex for {}", muncher_name);
                };
            }
        };

        return Some(file_def);
    }

    /// Matches the file to the right muncher based on the rules inside this struct.
    /// It picks the last match that meets the conditions.
    /// Only conditions included in `FileTypeMatch` struct are checked. The schema may have more, but they are not implemented.
    pub fn get_muncher_name(&self, file_name_with_path: &String) -> Option<String> {
        let mut best_match: Option<String> = None;
        if let Some(muncher_matches) = self.matches.as_ref() {
            // check all the matches and pick the last match that meets the conditions
            for muncher_match in muncher_matches {
                let muncher_name = muncher_match
                    .muncher
                    .as_ref()
                    .expect("Missing muncher name. It's a bug.");
                // if in_path is specified it must match
                if let Some(in_paths) = &muncher_match.in_path_regex {
                    for in_path in in_paths {
                        if in_path.is_match(file_name_with_path) {
                            best_match = Some(muncher_name.clone());
                            break;
                        }
                    }
                } else {
                    // if no in_path is in the match return it as the default
                    best_match = Some(muncher_name.clone());
                }
            }
        }

        // otherwise return None
        // if best_match.is_none() {
        //     warn!("No matching muncher found for {}.", file_name_with_path);
        // }
        best_match
    }
}
