use serde::Deserialize;
use regex::Regex;

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
    #[serde(skip)]
    pub name_regex: Option<Regex>
}

#[derive(Deserialize)]
pub(crate) struct Config {
    pub files: Vec<FileRules>,
}

impl Default for FileRules {
    fn default() -> Self {
        FileRules {
            name: ".\\.cs".to_owned(),
            required: true,
            keywords: vec!["class".to_owned(), "using".to_owned()],
            line_comments: "//".to_owned(),
            doc_comments: "///".to_owned(),
            block_comments: ["/*".to_owned(), "*/".to_owned()],
            inline_comments: "".to_string(),
            bracket_only: "".to_string(),
            use_dependency: "using".to_owned(),
            name_regex: None
        }
    }
}

impl Default for Config {
  fn default() -> Self {
    Config {
        files: vec![FileRules::default()]
      }
  }

}
