use super::file_type::FileType;
use super::muncher::Muncher;
use regex::Regex;
use rust_embed::RustEmbed;
use std::collections::{BTreeMap, HashSet};
use tracing::{debug, info, trace};

/// A container for embedded file_type rules
#[derive(RustEmbed)]
#[folder = "stm_rules/file_types"]
struct EmbeddedCodeRulesFileTypes;

/// A container for embedded muncher rules
#[derive(RustEmbed)]
#[folder = "stm_rules/munchers"]
struct EmbeddedCodeRulesMunchers;

#[derive(Debug, Clone)]
pub struct CodeRules {
    /// All file types are added at init time
    pub files_types: BTreeMap<String, FileType>,

    /// Munchers are loaded on-demand
    pub munchers: BTreeMap<String, Option<Muncher>>,

    /// A compiled regex for fetching a file extension from the full
    /// file path, including directories
    pub file_ext_regex: Regex,

    /// Contains names of newly added munchers to assist merging multiple instances
    /// of CodeRules for parallel processing.
    pub new_munchers: Option<HashSet<String>>,

    /// Compiled regex for file names and paths that should be ignored regardless of any other rules
    pub ignore_paths: Vec<Regex>,
}

impl CodeRules {
    /// Create a new instance from a a list of file-type files at `file_type_dir`
    /// File-type rules are loaded upfront, munchers are loaded dynamically
    pub fn new() -> Self {
        // collect relevant file names, ignore the rest
        let file_names: Vec<String> = EmbeddedCodeRulesFileTypes::iter()
            .filter_map(|file_name| {
                if file_name.ends_with(".json") {
                    Some(file_name.to_string())
                } else {
                    None
                }
            })
            .collect();
        info!("FileTypes files found: {}", file_names.len());

        // prepare the output collector
        let mut code_rules = CodeRules {
            files_types: BTreeMap::new(),
            munchers: BTreeMap::new(),
            // c:/dir/foo.bar -> bar
            // c:/dir/.bar -> bar
            // c:/dir/foo -> foo
            // dir\foo -> foo
            file_ext_regex: Regex::new(r#"[\.\\/][a-zA-Z0-1_]+$|^[a-zA-Z0-1_]+$"#).unwrap(),
            new_munchers: None,
            ignore_paths: crate::ignore_paths::compile_ignore_paths(),
        };

        // load the contents of file_type definitions one by one
        for file in file_names {
            let contents = EmbeddedCodeRulesFileTypes::get(&file)
                .expect(format!("Missing embedded file_type contents: {}", file).as_str());

            let contents = std::str::from_utf8(contents.data.as_ref())
                .expect(format!("Invalid file_type contents: {}", file).as_str());

            if let Some(ft) = FileType::new(&file, contents) {
                debug!("File type def found: {}", ft.file_ext);
                code_rules.files_types.insert(ft.file_ext.clone(), ft);
            }
        }

        code_rules
    }

    /// Return the right muncher for the file extension extracted from the full path.
    pub fn get_muncher(&mut self, file_path: &String) -> Option<&Muncher> {
        debug!("Getting a muncher for: {}", file_path);
        // try to get file extension or the file name if it has no extension like Dockerfile
        if let Some(ext) = self.file_ext_regex.find(&file_path) {
            // the file ext regex returns the ext with the separator, which is a ., but if the file has no extension it returns
            // the file name with the leading separator, which can be / or \
            // if the file has chars outside what the regex expects in a valid ext or file name it returns nothing
            let ext = ext
                .as_str()
                .trim_start_matches(".")
                .trim_start_matches("\\")
                .trim_start_matches("/")
                .to_lowercase();
            debug!("Extracted file extension: {}", ext);
            // try to find a file_type match for the ext
            if let Some(file_type) = self.files_types.get(&ext) {
                debug!("Matching file-type: {}", file_type.file_ext);
                // try to find a matching muncher
                if let Some(muncher_name) = file_type.get_muncher_name(file_path) {
                    // load the muncher from its file on the first use
                    if !self.munchers.contains_key(&muncher_name) {
                        // all muncher definition files have .json ext
                        let muncher_file_name = [&muncher_name, ".json"].concat();
                        trace!("Loading muncher {} for the 1st time", muncher_file_name);

                        let contents = EmbeddedCodeRulesMunchers::get(&muncher_file_name)
                            .expect(format!("Missing embedded muncher contents: {}", muncher_file_name).as_str());
                        let contents = std::str::from_utf8(contents.data.as_ref())
                            .expect(format!("Invalid muncher contents: {}", muncher_file_name).as_str());

                        // Insert None if the muncher could not be loaded so that it doesn't try to load it again
                        self.munchers
                            .insert(muncher_name.clone(), Muncher::new(contents, &muncher_name));

                        // indicate to the caller that there were new munchers added so they can be shared with other threads
                        if self.new_munchers.is_none() {
                            self.new_munchers = Some(HashSet::new());
                        }
                        self.new_munchers.as_mut().unwrap().insert(muncher_name.clone());
                    }

                    return self.munchers.get(&muncher_name).unwrap().as_ref();
                }
            } else {
                debug!("File-type is unknown");
            }
        }

        debug!("No muncher found for {}", file_path);

        None
    }
}
