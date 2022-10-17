use crate::graphql::RustScalarValue;
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{error, warn};

#[derive(Debug, Serialize, Deserialize, Eq, Clone, GraphQLObject)]
#[graphql(scalar = RustScalarValue)]
pub struct KeywordCounter {
    /// keyword
    pub k: String,
    /// array of free text after the keyword
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t: Option<HashSet<String>>,
    /// count
    pub c: u64,
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
    pub(crate) fn new_keyword(keyword: String, count: u64) -> Self {
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
    pub(crate) fn new_ref(keyword: String, count: u64) -> Self {
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
            // keep iterating until the first separator (not -._"'@)
            if c.is_ascii_alphanumeric() || *c == 45u8 || *c == 46u8 || *c == 95u8 || *c == 64u8 {
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
            ths.insert(t.trim().to_string());
            kwc.t = Some(ths);
            kwc.k = k.to_string();

            return kwc;
        }

        // return as-is if the keyword is taking the entire length
        // or starts with a boundary
        kwc
    }

    /// Splits the value in `k` into separate keywords at any non-aplhanumeric character and converts them into lowercase.
    /// May exit early and return a blank on error. Only ASCII characters are included in the output.
    pub(crate) fn split(&self) -> Vec<String> {
        // a container for keywords
        // there should be no more than 10 keywords per any kind of reference - allocate straight away
        let mut kws: Vec<String> = Vec::new();
        kws.reserve(10);

        // a container for keyword characters
        let mut kw: Vec<u8> = Vec::new();
        kw.reserve(self.k.len());

        // refs and packages are expected to be ASCII only
        for k in self.k.to_lowercase().chars() {
            if k.is_ascii_alphanumeric() {
                let mut buf = [0_u8; 4];
                k.encode_utf8(&mut buf);
                // we only need the first byte because it is ASCII
                kw.push(buf[0]);
            } else {
                if let Ok(kw_utf8) = String::from_utf8(kw) {
                    if !kw_utf8.is_empty() {
                        // e.g. ___futures___ produces a few empty strings
                        kws.push(kw_utf8);
                    }
                    kw = Vec::new();
                } else {
                    error!("Failed to extract keywords from: {}. It's a bug.", self.k);
                    return Vec::new();
                }
            }
        }

        // push remaining characters into the output container
        if !kw.is_empty() {
            if let Ok(kw_utf8) = String::from_utf8(kw) {
                kws.push(kw_utf8);
            } else {
                error!("Failed to extract keywords from: {}. It's a bug.", self.k);
            }
        }

        kws
    }
}
