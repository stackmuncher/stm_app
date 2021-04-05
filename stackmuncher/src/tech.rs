use super::kwc::{KeywordCounter, KeywordCounterSet};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, trace, warn};

#[derive(Serialize, Deserialize, Debug, Eq, Clone)]
#[serde(rename = "tech")]
pub struct Tech {
    /// The name of the file for individual file reports. Not present in combined tech reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// The computer language as identified by the muncher
    pub language: String,
    /// The name of the muncher used to process the file
    pub muncher_name: String,
    /// A short hash of the muncher rules to detect a muncher change for reprocessing
    #[serde(default)]
    pub muncher_hash: u64,
    /// SHA1 of the commit this file was taken from. E.g. 105eaf871c7248c93ae2f13337e9881caf89d489
    /// It is used for hashing. Not present in combined tech reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha1: Option<String>,
    /// The date of the commit this file was taken from in EPOCH format. E.g. 1544532686
    /// Not present in combined tech reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_date_epoch: Option<i64>,
    /// The date of the commit this file was taken from in ISO format. E.g. 2018-12-09T22:29:40+01:00
    /// Not present in combined tech reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_date_iso: Option<String>,
    pub files: usize,
    pub total_lines: usize,
    pub blank_lines: usize,
    pub bracket_only_lines: usize,
    pub code_lines: usize,
    pub inline_comments: usize,
    pub line_comments: usize,
    pub block_comments: usize,
    pub docs_comments: usize,
    /// Language-specific keywords, e.g. static, class, try-catch
    #[serde(skip_serializing_if = "HashSet::is_empty", default = "HashSet::new")]
    pub keywords: HashSet<KeywordCounter>, // has to be Option<>
    /// References to other libs, packages and namespaces
    /// E.g. `use` keyword
    #[serde(skip_serializing_if = "HashSet::is_empty", default = "HashSet::new")]
    pub refs: HashSet<KeywordCounter>, // has to be Option<>
    /// Unique words from refs. Only populated during the final merge of
    /// all user reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs_kw: Option<HashSet<KeywordCounter>>,
    /// References to other libs and packages in pkg managers
    /// E.g. refs from NuGet or Cargo.toml
    #[serde(skip_serializing_if = "HashSet::is_empty", default = "HashSet::new")]
    pub pkgs: HashSet<KeywordCounter>, // has to be Option<>
    /// Unique words from pkgs. Only populated during the final merge of
    /// all user reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pkgs_kw: Option<HashSet<KeywordCounter>>,
}

impl std::hash::Hash for Tech {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.muncher_name.as_bytes());
        state.write(self.language.as_bytes());
        if let Some(file_name) = &self.file_name {
            state.write(file_name.as_bytes());
        };
        if let Some(commit_sha1) = &self.commit_sha1 {
            state.write(commit_sha1.as_bytes());
        };
        state.finish();
    }
}

impl PartialEq for Tech {
    fn eq(&self, other: &Self) -> bool {
        self.muncher_name == other.muncher_name && self.language == other.language && self.file_name == other.file_name
    }
}

impl Tech {
    /// Sets `file_name` and commit info to None to match tech records on `muncher_name` and `language` only.
    /// `per_file_tech` records are matched with all that info present because it is specific to the file.
    /// `tech` records in the report are aggregates across multiple files and should have that info removed.
    pub(crate) fn reset_file_and_commit_info(self) -> Self {
        let mut tech = self;

        tech.file_name = None;
        tech.commit_sha1 = None;
        tech.commit_date_epoch = None;
        tech.commit_date_iso = None;

        tech
    }

    /// Extract and count matches for `self.refs`
    #[inline]
    pub(crate) fn count_refs(&mut self, regex: &Option<Vec<Regex>>, line: &String) {
        Self::count_matches(regex, line, &mut self.refs, &KeywordCounter::new_ref);
    }

    /// Extract and count keywords for `self.keywords`
    #[inline]
    pub(crate) fn count_keywords(&mut self, regex: &Option<Vec<Regex>>, line: &String) {
        Self::count_matches(regex, line, &mut self.keywords, &KeywordCounter::new_keyword);
    }

    /// Extract and count matches for `self.pkgs`
    #[inline]
    pub(crate) fn count_pkgs(&mut self, regex: &Option<Vec<Regex>>, line: &String) {
        Self::count_matches(regex, line, &mut self.pkgs, &KeywordCounter::new_ref);
    }

    /// Count `regex` matches in the given `line` using `kw_counter_factory` Fn
    /// and add the counts to `kw_counter`.
    #[inline]
    fn count_matches<B>(
        regex: &Option<Vec<Regex>>,
        line: &String,
        kw_counter: &mut HashSet<KeywordCounter>,
        kw_counter_factory: &B,
    ) where
        B: Fn(String, usize) -> KeywordCounter,
    {
        // process if there is a regex in the list of rules
        if let Some(v) = regex {
            for r in v {
                if let Some(groups) = r.captures(line) {
                    // The regex may or may not have capture groups. The counts depend on that.
                    // We'll assume that if there is only capture[0], which is the whole string,
                    // then it's one match. If there is > 1, then it's .len()-1, because capture[0]
                    // is always present as the full string match.

                    // grab the exact match, if any, otherwise grab the whole string match
                    let cap = if groups.len() > 1 {
                        let gr_ar: Vec<&str> = groups
                            .iter()
                            .skip(1)
                            .filter_map(|g| if g.is_some() { Some(g.unwrap().as_str()) } else { None })
                            .collect();
                        gr_ar.join(" ").trim().to_string()
                    } else {
                        groups[0].to_string()
                    };

                    trace!("{} x {} for {}", cap, groups.len(), r);

                    // do not allow empty keywords or references
                    if cap.is_empty() {
                        continue;
                    }

                    // Python imports may start with . which needs to be removed
                    let cap = cap.trim_matches('.').to_owned();

                    // add the counts depending with different factory functions for different Tech fields
                    kw_counter.increment_counters(kw_counter_factory(cap, 1));
                }
            }
        }
    }

    /// Generate a summary of keywords for Tech.refs_kw or Tech.pkgs_kw
    pub(crate) fn new_kw_summary(refs: &HashSet<KeywordCounter>) -> Option<HashSet<KeywordCounter>> {
        // exit early if there are no refs
        if refs.is_empty() {
            return None;
        };

        // a collector of split keywords with their counts, e.g. System, Text, Regex
        // from System.Text.Regex
        let mut kw_sum: HashSet<KeywordCounter> = HashSet::new();

        // loop through all Tech.refs
        for kwc in refs {
            // split at . and add them app
            for kw in kwc.k.split('.') {
                if kw.len() > 2 {
                    let split_kwc = KeywordCounter {
                        k: kw.to_owned(),
                        t: None,
                        c: kwc.c,
                    };
                    kw_sum.increment_counters(split_kwc);
                }
            }
        }

        Some(kw_sum)
    }

    /// Removes report refs (imports) that match local file names because they are likely to be local imports
    /// and should not be included in the report. The tree must correspond to the commit. If the tree was taken from
    /// HEAD then some files may be missing making local import names appear as if they are 3rd party.
    /// #### Optimized for Python.
    pub(crate) fn remove_local_imports(self, all_tree_files: Option<&HashSet<String>>) -> Self {
        // exit if no file names or references are missing
        if all_tree_files.is_none() {
            warn!("No tree files supplied for local import removal.");
            return self;
        }
        if self.refs.is_empty() {
            return self;
        }

        // we need a mut self to remove local imports later
        // it is hard to make it mut later in the code because of a borrow we have to make
        let mut tech = self;

        // normalize the file names for easy string comparison, e.g. zerver/worker/queue_processors.py -> zerver.worker.queue_processors
        let files_normalized = all_tree_files
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v,
                    v[0..v.rfind(".").unwrap_or_else(|| v.len())]
                        .to_string()
                        .replace("/", ".")
                        .replace("\\", ".")
                        .to_lowercase(),
                )
            })
            .collect::<Vec<(&String, String)>>();

        // normalize the keywords the same way as the file names, e.g. zerver::worker::queue_processors -> zerver.worker.queue_processors
        let all_imports_normalized = tech
            .refs
            .iter()
            .map(|kwc| (kwc, kwc.k.replace("::", ".").replace(":", ".").to_lowercase()))
            .collect::<Vec<(&KeywordCounter, String)>>();

        // a collector of pointers at kwc.k for local imports
        let mut local_imports: Vec<KeywordCounter> = Vec::new();

        // check every import name against the file names for possible matches
        for (kwc, normalized_import) in all_imports_normalized {
            let is_compound_name = normalized_import.contains(".");
            for (full_file_name, file_normalized) in &files_normalized {
                if is_compound_name {
                    // imports with a . in it can be safely matched mid-string
                    // e.g. zerver.worker matches zerver.worker.queue_processors
                    if let Some(start_idx) = file_normalized.find(&normalized_import) {
                        if (start_idx > 0 && file_normalized.as_bytes()[start_idx - 1] != 46u8)
                            || (start_idx + normalized_import.len() < file_normalized.len()
                                && file_normalized.as_bytes()[start_idx + normalized_import.len()] != 46u8)
                        {
                            // either start or end of the match falls mid-word of the file name
                            // e.g. `os.file` and `dos.file`
                            continue;
                        }
                    } else {
                        // there was no match on the substring at all
                        continue;
                    };
                } else if file_normalized.ends_with(&normalized_import) {
                    // check if it's the full keyword match at the end of the file name
                    // e.g. `os` should not be matched with `dos`, but `d.os` should be OK
                    if file_normalized.len() > normalized_import.len() {
                        if !&file_normalized[0..file_normalized.len() - normalized_import.len()].ends_with(".") {
                            continue;
                        }
                    }
                } else {
                    // it's not a local import
                    continue;
                }
                // it's a local import - remove
                // .clone() is necessary to remove the local kwc from the list later
                // there should only be a small number of imports per project
                local_imports.push(kwc.clone());
                debug!("Removing local import: {} / {}", kwc.k, full_file_name);
                break;
            }
        }

        // remove the local imports from the list
        for local_import in local_imports {
            tech.refs.remove(&local_import);
        }

        tech
    }
}
