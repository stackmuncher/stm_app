use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Debug)]
#[serde(rename = "tech")]
pub(crate) struct Tech {
    pub name: String,
    pub files: usize,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub bracket_only_lines: usize,
    pub code_lines: usize,
    pub inline_comments: usize,
    pub line_comments: usize,
    pub block_comments: usize,
    pub docs_comments: usize,
    pub keywords: HashMap<String, usize>,
    pub use_dependencies: HashSet<String>,
}

#[derive(Serialize)]
#[serde(rename = "tech")]
pub(crate) struct Report {
    pub tech: HashMap<String, Tech>,
    pub timestamp: String,
}
