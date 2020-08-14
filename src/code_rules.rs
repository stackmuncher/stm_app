use regex::Regex;
use serde::Deserialize;
use std::fs;
use tracing::{error, trace};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename = "file")]
pub struct FileRules {
    pub language: Option<String>,
    pub technology: Option<String>,
    pub file_names: Vec<String>,
    pub keywords: Option<Vec<String>>,
    pub bracket_only: Option<Vec<String>>,
    pub line_comments: Option<Vec<String>>,
    pub inline_comments: Option<Vec<String>>,
    pub doc_comments: Option<Vec<String>>,
    pub block_comments_start: Option<Vec<String>>,
    pub block_comments_end: Option<Vec<String>>,
    pub refs: Option<Vec<String>>,

    // Regex section is compiled once from the above strings
    /// `file_names` field is always compiled to identify files
    #[serde(skip)]
    pub file_names_regex: Option<Vec<Regex>>,
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
    pub blank_line_regex: Option<Vec<Regex>>,
    #[serde(skip)]
    pub keywords_regex: Option<Vec<Regex>>,

    /// Set to true if all the regex for this object was compiled
    #[serde(skip)]
    pub compiled: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CodeRules {
    pub files: Vec<FileRules>,
    /// Set to true if there was a compilation for any file-specific regex
    /// to assist merging multiple instances
    #[serde(skip)]
    pub recompiled: bool,
}

impl CodeRules {

    /// Create a new instance from a file at `code_rules_path` and pre-compile regex for file names.
    pub fn new(code_rules_path: &String) -> Self {
        // load code analysis rules config
        let conf = fs::File::open(code_rules_path).expect("Cannot read config file");
        let mut conf: Self = serde_json::from_reader(conf).expect("Cannot parse config file");

        // pre-compile regex rules for file names
        for file_rules in conf.files.iter_mut() {
            file_rules.compile_file_name_regex();
        };

        conf
    }
}

impl FileRules {
    /// Compiles `file_names` field only. Idempotent. Will recompile anew on every call.
    pub(crate) fn compile_file_name_regex(&mut self) {
        // are there any names?
        if self.file_names.is_empty() {
            error!("An empty list of names in the config!");
            panic!();
        }

        trace!("compile_file_name_regex for {}", self.file_names.join(", "));

        // reset the list in case this function was called more than once
        self.file_names_regex = None;

        // compile regex for file names
        for s in &self.file_names {
            add_regex_to_list(&mut self.file_names_regex, s);
        }
    }

    /// Compiles regex strings other than `file_names`. It is safe to call it multiple times.
    /// It will only try to compile once per lifetime of the object using `compiled` field as
    /// a flag.
    pub(crate) fn compile_other_regex(&mut self) -> bool  {
        trace!("compile_other_regex for {}", self.file_names.join(", "));

        // check if it was compiled before
        if self.compiled {
            trace!("Already compiled.");
            return false;
        }

        // resets to `false` if any of the regex statements failed to compile
        let mut compilation_success = true;

        if let Some(v) = self.bracket_only.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.bracket_only_regex, s);
            }
        }

        if let Some(v) = self.line_comments.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.line_comments_regex, s);
            }
        }

        if let Some(v) = self.inline_comments.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.inline_comments_regex, s);
            }
        }

        if let Some(v) = self.doc_comments.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.doc_comments_regex, s);
            }
        }

        if let Some(v) = self.block_comments_start.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.block_comments_start_regex, s);
            }
        }

        if let Some(v) = self.block_comments_end.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.block_comments_end_regex, s);
            }
        }

        if let Some(v) = self.refs.as_ref() {
            for s in v {
                compilation_success &= add_regex_to_list(&mut self.refs_regex, s);
            }
        }

        if let Some(v) = self.keywords.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.keywords_regex, s);
            }
        }

        // empty strings should have the same regex, but this may change - odd one out
        compilation_success &= add_regex_to_list(&mut self.blank_line_regex, &r"^\s*$".to_string());

        // panic if there were compilation errors
        if !compilation_success {
            panic!();
        }

        // indicate this file struct has been compiled
        self.compiled = true;

        // return true
        true
    }
}

/// Adds the `regex` to the supplied `list`. Creates an instance of Vec<Regex> on the first insert.
/// Always returns Some(). Returns FALSE on regex compilation error.
fn add_regex_to_list(list: &mut Option<Vec<Regex>>, regex: &String) -> bool {
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
