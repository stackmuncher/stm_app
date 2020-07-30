use regex::Regex;
use serde::Deserialize;
use tracing::{error, trace};

#[derive(Deserialize)]
#[serde(rename = "file")]
pub(crate) struct FileRules {
    pub file_names: Vec<String>,
    pub keywords: Option<Vec<String>>,
    pub bracket_only: Option<Vec<String>>,
    pub line_comments: Option<Vec<String>>,
    pub inline_comments: Option<Vec<String>>,
    pub doc_comments: Option<Vec<String>>,
    pub block_comments_start: Option<Vec<String>>,
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
    pub block_comments_regex: Option<Vec<Regex>>,
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

#[derive(Deserialize)]
pub(crate) struct Config {
    pub files: Vec<FileRules>,
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
    pub(crate) fn compile_other_regex(&mut self) {
        trace!("compile_other_regex for {}", self.file_names.join(", "));

        // check if it was compiled before
        if self.compiled {
            trace!("Already compiled.");
            return;
        }

        if let Some(v) = self.bracket_only.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.bracket_only_regex, s);
            }
        }

        if let Some(v) = self.line_comments.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.line_comments_regex, s);
            }
        }

        if let Some(v) = self.inline_comments.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.inline_comments_regex, s);
            }
        }

        if let Some(v) = self.doc_comments.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.doc_comments_regex, s);
            }
        }

        if let Some(v) = self.refs.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.refs_regex, s);
            }
        }

        if let Some(v) = self.keywords.as_ref() {
            for s in v {
                add_regex_to_list(&mut self.keywords_regex, s);
            }
        }

        // empty strings should have the same regex, but this may change - odd one out
        add_regex_to_list(&mut self.blank_line_regex, &r"^\s*$".to_string());

        self.compiled = true;

    }
}

/// Adds the `regex` to the supplied `list`. Creates an instance of Vec<Regex> on the first insert.
/// Always returns Some(). Panics on regex compilation error.
fn add_regex_to_list(list: &mut Option<Vec<Regex>>, regex: &String) {
    // try to compile the regex
    let compiled_regex = match Regex::new(regex) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to compile regex {} with {}", regex, e);
            panic!();
        }
    };

    // get the existing vector or create a new one
    if list.is_none() {
        list.replace(Vec::new());
    }

    // add the new regex to the list
    list.as_mut().unwrap().push(compiled_regex); //
}
