use super::kwc::{KeywordCounter, KeywordCounterSet};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::trace;

#[derive(Serialize, Deserialize, Debug, Eq, Clone)]
#[serde(rename = "tech")]
pub struct Tech {
    pub language: String,
    pub muncher_name: String,
    /// A short hash of the muncher rules to detect a change for reprocessing
    #[serde(default)]
    pub muncher_hash: u64,
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
    pub keywords: HashSet<KeywordCounter>, // has to be Option<>
    /// References to other libs, packages and namespaces
    /// E.g. `use` keyword
    pub refs: HashSet<KeywordCounter>, // has to be Option<>
    /// Unique words from refs. Only populated during the final merge of
    /// all user reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refs_kw: Option<HashSet<KeywordCounter>>,
    /// References to other libs and packages in pkg managers
    /// E.g. refs from NuGet or Cargo.toml
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
        state.finish();
    }
}

impl PartialEq for Tech {
    fn eq(&self, other: &Self) -> bool {
        self.muncher_name == other.muncher_name && self.language == other.language
    }
}

impl Tech {
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
                    let (cap, group_len) = if groups.len() > 1 {
                        // for g in groups.iter().skip(1) {
                        //     g.unwrap_or_default().as_str()
                        // }

                        let gr_ar: Vec<&str> = groups.iter().skip(1).map(|g| g.unwrap().as_str()).collect();
                        (gr_ar.join(" "), groups.len() - 1)
                    //(groups[1].to_string(), groups.len() - 1)
                    } else {
                        (groups[0].to_string(), 1)
                    };

                    trace!("{} x {} for {}", cap, groups.len(), r);

                    // do not allow empty keywords or references
                    if cap.is_empty() {
                        continue;
                    }

                    // add the counts depending with different factory functions for different Tech fields
                    kw_counter.increment_counters(kw_counter_factory(cap, group_len));
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
}
