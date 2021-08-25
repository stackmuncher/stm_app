use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::warn;

/// A very concise overview of a single Tech record
/// to show the share of the technology in the project
#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub struct TechOverview {
    /// The same as Tech.language
    pub language: String,
    /// Lines Of Code including blank lines
    pub loc: usize,
    /// Total number of unique library names
    pub libs: usize,
    /// Percentage of the LoC for this tech from the total LoC for the project
    pub loc_percentage: usize,
}

impl std::hash::Hash for TechOverview {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.language.as_bytes());
        state.finish();
    }
}

impl PartialEq for TechOverview {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language
    }
}

/// An overview of an individual project report included in the combined report
/// to avoid loading the full project report every time the combined report is looked at.
#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub struct ProjectReportOverview {
    /// A human-readable project name. It should not be used as an ID.
    #[serde(default = "String::new")]
    pub project_name: String,
    /// `owner_id` + `project_id` are used to identify which project the overview belongs to.
    /// There should be just one project included in a contributor or a combined contributor report.
    /// Each combined report is submitted to STM server and is further combined with reports for other projects from the same dev there.
    /// Values are set on the server. Values set on the client are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    /// `owner_id` + `project_id` are used to identify which project the overview belongs to.
    /// The value is an internal STM server project ID derived from commit hashes.
    /// E.g. `KxnFH4mTcfEQ73umbt6e1Y`.
    /// Values are set on the server. Values set on the client are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// GitHub user name, if known.
    /// Values are set on the server. Values set on the client are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_user_name: Option<String>,
    /// A GitHub name of the project, if known. GitHub project names do not include the user name.
    /// E.g. `https://github.com/awslabs/aws-lambda-rust-runtime.git` would have project name as `aws-lambda-rust-runtime`.
    /// Values are set on the server. Values set on the client are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_repo_name: Option<String>,
    /// The date of the first project or contributor commit, e.g. 2020-08-26T14:15:46+01:00
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_init: Option<String>,
    /// The date of the current HEAD for project or the latest contributor commit,
    /// e.g. 2021-06-30T22:06:42+01:00
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_head: Option<String>,
    /// Lines Of Code (excludes blank lines) to show the size of the project
    pub loc: usize,
    /// Total number of unique library names to show the breadth of the project
    pub libs: usize,
    /// Total number of contributors to show the size of the team
    pub ppl: usize,
    pub tech: HashSet<TechOverview>,
    /// The last N commits for matching reports to projects.
    /// Full project reports have the list of commits from all contributors. Contributor reports only have commits for that contributor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<String>>,
}

impl std::hash::Hash for ProjectReportOverview {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        // what will happen if all of them are NONE?
        if let Some(v) = &self.owner_id {
            state.write(v.as_bytes());
        }
        if let Some(v) = &self.project_id {
            state.write(v.as_bytes());
        }
        if let Some(v) = &self.github_user_name {
            state.write(v.as_bytes());
        }
        if let Some(v) = &self.github_repo_name {
            state.write(v.as_bytes());
        }
        state.finish();
    }
}

impl PartialEq for ProjectReportOverview {
    fn eq(&self, other: &Self) -> bool {
        // the priority matching is for owner/project, if they are set, otherwise match on github ids
        // it will equate if they are all None
        (self.owner_id.is_some()
            && self.project_id.is_some()
            && self.owner_id == other.owner_id
            && self.project_id == other.project_id)
            || (self.github_user_name == other.github_user_name && self.github_repo_name == other.github_repo_name)
    }
}

impl super::tech::Tech {
    /// Returns an abridged version of Self in the form of TechBrief.
    /// Calculation of `libs` is not very accurate. See comments inside the body.
    pub(crate) fn get_overview(&self) -> TechOverview {
        TechOverview {
            language: self.language.clone(),
            loc: self.total_lines,
            // the percentage is not known at this stage
            loc_percentage: 0,
            // this is not a good way of doing it
            // there will be some overlap between pkgs and refs,
            // but getting a unique list is not that straight forward and is language specific
            libs: self.pkgs.len() + self.refs.len(),
        }
    }
}

impl super::report::Report {
    /// Returns an abridged version of Self in the form of ProjectReportOverview.
    /// Calculation of `libs` is not very accurate. See comments inside the body.
    pub(crate) fn get_overview(&self) -> ProjectReportOverview {
        // collect all tech data in the overview form
        // there may be multiple records for the same tech, e.g. Rust/.rs and Rust/.toml, so they need to be added up
        let mut tech_overviews: HashMap<String, TechOverview> = HashMap::new();
        for tech in &self.tech {
            let tech_to_update_from = tech.get_overview();
            // update the existing record or add a new one
            if let Some(tech_to_update) = tech_overviews.get_mut(&tech.language) {
                tech_to_update.libs += tech_to_update_from.libs;
                tech_to_update.loc += tech_to_update_from.loc;
            } else {
                tech_overviews.insert(tech.language.clone(), tech_to_update_from);
            }
        }

        // convert to an easier to use HashSet
        let tech_overviews = tech_overviews
            .into_iter()
            .map(|(_, t)| t)
            .collect::<HashSet<TechOverview>>();

        // collect summary
        let loc = tech_overviews.iter().map(|t| t.loc).sum::<usize>();
        let libs = tech_overviews.iter().map(|t| t.libs).sum::<usize>();
        let ppl = match self.contributor_git_ids.as_ref() {
            None => 0,
            Some(v) => v.len(),
        };

        // update percentages
        let tech = tech_overviews
            .into_iter()
            .map(|mut t| {
                // avoid division by zero
                t.loc_percentage = t.loc * 100 / loc.max(1);
                t
            })
            .collect::<HashSet<TechOverview>>();

        // use GitHub's project name if it exists, otherwise make one up
        let project_name = match &self.github_repo_name {
            Some(v) => v.clone(),
            None => project_name_from_date(&self.date_init),
        };

        ProjectReportOverview {
            project_name,
            project_id: self.project_id.clone(),
            owner_id: self.owner_id.clone(),
            github_repo_name: self.github_repo_name.clone(),
            github_user_name: self.github_user_name.clone(),
            tech,
            date_init: self.date_init.clone(),
            date_head: self.date_head.clone(),
            loc,
            libs,
            ppl,
            commits: self.recent_project_commits.clone(),
        }
    }
}

/// Converts an ISO3339 date into a project name using numbers based on the repo init date.
/// Returns a random timestamp-based name if the input is invalid.
fn project_name_from_date(date: &Option<String>) -> String {
    // try to convert the data from the report into a valid UTC struct
    // fall back to Utc::now if it fails at any of the steps
    let date = match date {
        None => Utc::now(),
        Some(d) => {
            let date = match DateTime::parse_from_rfc3339(d) {
                Ok(d) => d.with_timezone(&Utc),
                Err(e) => {
                    warn!("Invalid project date: {} with {}", d, e);
                    Utc::now()
                }
            };

            date
        }
    };

    // the name is structured to make it look more or less random to an outsider and vaguely recognizable to the owner
    // E.g. `Private project #0821bb`, `Private project #3420s`
    [
        "Private project #",
        // pad 1-digi weeks with a leading 0
        if date.iso_week().week() < 10 { "0" } else { "" },
        date.iso_week().week().to_string().as_str(),
        &date.year().to_string()[2..],
        DAYS_AS_LETTERS[date.day() as usize - 1],
    ]
    .concat()
}

const DAYS_AS_LETTERS: [&str; 31] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w",
    "x", "y", "z", "aa", "bb", "cc", "dd", "xx",
];
