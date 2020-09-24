use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{error, warn};

#[derive(Debug, Serialize, Deserialize, Eq, Clone)]
pub struct KeywordCounter {
    /// keyword
    pub k: String,
    /// array of free text after the keyword
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<HashSet<String>>,
    /// count
    pub c: usize,
}

pub(crate) trait KeywordCounterSet {
    fn increment_counters(&mut self, new_kw_counter: KeywordCounter);
}

impl std::hash::Hash for KeywordCounter {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.k.as_bytes());
        state.finish();
    }
}

impl PartialEq for KeywordCounter {
    fn eq(&self, other: &Self) -> bool {
        self.k == other.k
    }
}

impl KeywordCounterSet for HashSet<KeywordCounter> {
    /// Insert a new record or increment the counter for the existing one
    fn increment_counters(&mut self, new_kw_counter: KeywordCounter) {
        // this should not happen, but handling it just in case
        if new_kw_counter.c == 0 {
            warn!("Empty keyword counter.");
            return;
        }

        // increment if the record exists
        if let Some(mut existing_kw_counter) = self.take(&new_kw_counter) {
            existing_kw_counter.c += new_kw_counter.c;

            // additional parts of the keyword need to be added to the set
            if let Some(new_t) = new_kw_counter.t {
                if existing_kw_counter.t.is_none() {
                    existing_kw_counter.t = Some(new_t);
                } else {
                    if let Some(s) = new_t.iter().next().to_owned() {
                        existing_kw_counter.t.as_mut().unwrap().insert(s.to_owned());
                    }
                }
            };

            self.insert(existing_kw_counter);
        } else {
            // insert if it's a new one
            self.insert(new_kw_counter);
        }
    }
}

impl KeywordCounter {
    /// Returns Self with `t` as `None`. Panics if `keyword` is empty.
    pub(crate) fn new_keyword(keyword: String, count: usize) -> Self {
        if keyword.is_empty() {
            error!("Empty keyword for KeywordCounter in new_keyword");
        }

        Self {
            k: keyword,
            t: None,
            c: count,
        }
    }

    /// Splits `keyword` into `k` and `t`. Panics if `keyword` is empty.
    pub(crate) fn new_ref(keyword: String, count: usize) -> Self {
        if keyword.is_empty() {
            error!("Empty keyword for KeywordCounter in new_ref");
        }

        // output collector
        let mut kwc = Self {
            k: keyword,
            t: None,
            c: count,
        };

        // loop through the characters to find the first boundary
        for (i, c) in kwc.k.as_bytes().iter().enumerate() {
            // keep iterating until the first separator (not ._"')
            if c.is_ascii_alphanumeric() || *c == 46u8 || *c == 95u8 {
                continue;
            }

            // the very first character is a boundary - return as-is
            if i == 0 {
                warn!("Invalid ref: {}", kwc.k);
                return kwc;
            }

            // split the keyword at the boundary
            let (k, t) = kwc.k.split_at(i);
            let mut ths: HashSet<String> = HashSet::new();
            ths.insert(t.to_string());
            kwc.t = Some(ths);
            kwc.k = k.to_string();

            return kwc;
        }

        // return as-is if the keyword is taking the entire length
        // or starts with a boundary
        kwc
    }
}
