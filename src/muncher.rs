use regex::Regex;
use serde::Deserialize;
use std::fs;
use tracing::{trace, error};

#[derive(Deserialize, Clone, Debug)]
pub struct Muncher {
    #[serde(default)]
    pub muncher_name: String,
    pub language: String,
    pub keywords: Option<Vec<String>>,
    pub bracket_only: Option<Vec<String>>,
    pub line_comments: Option<Vec<String>>,
    pub inline_comments: Option<Vec<String>>,
    pub doc_comments: Option<Vec<String>>,
    pub block_comments_start: Option<Vec<String>>,
    pub block_comments_end: Option<Vec<String>>,
    pub refs: Option<Vec<String>>,
    pub packages: Option<Vec<String>>,

    // Regex section is compiled once from the above properties
    #[serde(skip)]
    pub bracket_only_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub line_comments_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub inline_comments_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub doc_comments_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub block_comments_start_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub block_comments_end_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub refs_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub packages_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub blank_line_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub keywords_regex: Option<Vec<Regex>>,
}

impl Muncher {
    /// Create a new instance from a file at `json_definition_file_path`.
    /// Returns None if there was a problem loading it
    pub fn new(json_definition_file_path: &String, muncher_name: &String) -> Option<Self> {
        trace!("Loading {}", json_definition_file_path);

        // load the file definition from a json file
        let conf = match fs::File::open(json_definition_file_path) {
            Err(e) => {
                error!("Cannot read config file {} with {}", json_definition_file_path, e);
                return None;
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

        
        conf.muncher_name = muncher_name.clone();

        // compile all regex strings
        if conf.compile_all_regex().is_err() {
            return None;
        }

        Some(conf)
    }

    /// Compiles regex strings.
    fn compile_all_regex(&mut self) -> Result<(),()> {
        trace!("Compiling regex for {}", self.muncher_name);

        // resets to `false` if any of the regex statements failed to compile
        // this is done to loop through all regex strings in the file and give
        // a combined view of any failed ones
        let mut compilation_success = true;

        if let Some(v) = self.bracket_only.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.bracket_only_regex, s);
            }
        }

        if let Some(v) = self.line_comments.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.line_comments_regex, s);
            }
        }

        if let Some(v) = self.inline_comments.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.inline_comments_regex, s);
            }
        }

        if let Some(v) = self.doc_comments.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.doc_comments_regex, s);
            }
        }

        if let Some(v) = self.block_comments_start.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.block_comments_start_regex, s);
            }
        }

        if let Some(v) = self.block_comments_end.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.block_comments_end_regex, s);
            }
        }

        if let Some(v) = self.refs.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.refs_regex, s);
            }
        }

        if let Some(v) = self.packages.as_ref() {
            for s in v {
                compilation_success &= Muncher::add_regex_to_list(&mut self.packages_regex, s);
            }
        }

        if let Some(v) = self.keywords.as_ref() {
            for s in v {
                Muncher::add_regex_to_list(&mut self.keywords_regex, s);
            }
        }

        // empty strings should have the same regex, but this may change - odd one out
        compilation_success &= Muncher::add_regex_to_list(&mut self.blank_line_regex, &r"^\s*$".to_string());

        // panic if there were compilation errors
        if compilation_success {
            return Ok(());
        }
        else {
            error!("Compilation for {} failed.", self.muncher_name);
            return Err(());
        }


    }

    /// Adds the `regex` to the supplied `list`. Creates an instance of Vec<Regex> on the first insert.
    /// Always returns Some(). Returns FALSE on regex compilation error.
   pub fn add_regex_to_list(list: &mut Option<Vec<Regex>>, regex: &String) -> bool {
        // try to compile the regex
        let compiled_regex = match Regex::new(regex) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to compile regex {} with {}", regex, e);
                return false;
            }
        };

        // get the existing vector or create a new one
        if list.is_none() {
            list.replace(Vec::new());
        }

        // add the new regex to the list
        list.as_mut().unwrap().push(compiled_regex);
        true
    }
}
