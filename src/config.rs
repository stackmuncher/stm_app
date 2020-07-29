use regex::Regex;
use serde::Deserialize;
use tracing::trace;

#[derive(Deserialize)]
#[serde(rename = "file")]
pub(crate) struct FileRules {
    pub name: String,
    pub required: bool,
    pub keywords: Vec<String>,
    pub bracket_only: String,
    pub line_comments: String,
    pub inline_comments: String,
    pub doc_comments: String,
    pub block_comments: [String; 2],
    pub use_dependency: String,

    // Regex section is compiled once from the above strings
    #[serde(skip)]
    pub name_regex: Option<Regex>,
    #[serde(skip)]
    pub bracket_only_regex: Option<Regex>,
    #[serde(skip)]
    pub line_comments_regex: Option<Regex>,
    #[serde(skip)]
    pub inline_comments_regex: Option<Regex>,
    #[serde(skip)]
    pub doc_comments_regex: Option<Regex>,
    #[serde(skip)]
    pub block_comments_regex: Option<Regex>,
    #[serde(skip)]
    pub use_dependency_regex: Option<Regex>,
    #[serde(skip)]
    pub blank_line_regex: Option<Regex>,
    #[serde(skip)]
    pub keywords_regex: Option<Vec<(String, Regex)>>,
}

#[derive(Deserialize)]
pub(crate) struct Config {
    pub files: Vec<FileRules>,
}

impl FileRules {
    /// Compiles all regex strings. It is safe to call it multiple times.
    /// It will only try to compile once per lifetime of the object.
    pub(crate) fn compile_regex(&mut self) {
        trace!("compile_regex for {}", self.name);

        // check if it was compiled before
        if self.name_regex.is_some() {
            trace!("Already compiled.");
            return;
        }

        // compile single regex items
        if !self.bracket_only.is_empty() {
            self.bracket_only_regex = Some(Regex::new(self.bracket_only.as_str()).expect("Bad regex for bracket_only"));
        }
        if !self.line_comments.is_empty() {
            self.line_comments_regex =
                Some(Regex::new(self.line_comments.as_str()).expect("Bad regex for line_comments"));
        }
        if !self.inline_comments.is_empty() {
            self.inline_comments_regex =
                Some(Regex::new(self.inline_comments.as_str()).expect("Bad regex for inline_comments"));
        }
        if !self.doc_comments.is_empty() {
            self.doc_comments_regex = Some(Regex::new(self.doc_comments.as_str()).expect("Bad regex for doc_comments"));
        }
        if !self.use_dependency.is_empty() {
            //let block_comments = Regex::new(rules.block_comments.as_str());
            self.use_dependency_regex =
                Some(Regex::new(self.use_dependency.as_str()).expect("Bad regex for use_dependency"));
        }

        self.blank_line_regex = Some(Regex::new(r"^\s*$").expect("Bad regex for blank_line - hardcoded"));

        // prepare regex for keywords
        if self.keywords.len() > 0 {
            let mut kw_regex: Vec<(String, Regex)> = Vec::new();
            for kw in &self.keywords {
                let re = &["\\b", kw.as_str(), "\\b"].concat();
                let re = Regex::new(re).expect("Invalid regex from a keyword");
                kw_regex.push((kw.clone(), re));
            }

            self.keywords_regex = Some(kw_regex);

        }
    }
}
