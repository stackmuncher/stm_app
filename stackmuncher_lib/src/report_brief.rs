use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    /// E.g. `save-golds`
    #[serde(skip_serializing_if = "String::is_empty", default = "String::new")]
    pub project_name: String,
    /// E.g. `644/save-golds.report`
    #[serde(skip_serializing_if = "String::is_empty", default = "String::new")]
    pub report_s3_key: String,
    /// The date of the first commit, e.g. 2020-08-26T14:15:46+01:00
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_init: Option<String>,
    /// The date of the current HEAD, e.g. 2021-06-30T22:06:42+01:00
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_head: Option<String>,
    /// Lines Of Code (excludes blank lines) to show the size of the project
    pub loc: usize,
    /// Total number of unique library names to show the breadth of the project
    pub libs: usize,
    /// Total number of contributors to show the size of the team
    pub ppl: usize,
    pub tech: HashSet<TechOverview>,
    /// A list of hashed remote URLs from the repo. They are used in place of the private project name
    /// and can be used to match a local project to publicly available projects. If that happens the project name
    /// is populated automatically by STM on the server side
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url_hashes: Option<HashSet<String>>,
    /// The last N commits for matching projects that changed name, remote URL or any other identifying property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_project_commits: Option<Vec<String>>,
}

impl std::hash::Hash for ProjectReportOverview {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        state.write(self.report_s3_key.as_bytes());
        state.finish();
    }
}

impl PartialEq for ProjectReportOverview {
    fn eq(&self, other: &Self) -> bool {
        self.report_s3_key == other.report_s3_key
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
    /// Returns an abridged version of Self in the form of TechBrief.
    /// Calculation of `libs` is not very accurate. See comments inside the body.
    pub(crate) fn get_overview(&self) -> ProjectReportOverview {
        // collect all tech data in the overview form
        let tech = self
            .tech
            .iter()
            .map(|t| t.get_overview())
            .collect::<HashSet<TechOverview>>();

        // collect summary
        let loc = tech.iter().map(|t| t.loc).sum::<usize>();
        let libs = tech.iter().map(|t| t.libs).sum::<usize>();
        let ppl = match self.contributor_git_ids.as_ref() {
            None => 0,
            Some(v) => v.len(),
        };

        // update percentages
        let tech = tech
            .into_iter()
            .map(|mut t| {
                // avoid division by zero
                t.loc_percentage = t.loc * 100 / loc.max(1);
                t
            })
            .collect::<HashSet<TechOverview>>();

        ProjectReportOverview {
            project_name: self.github_repo_name.clone(),
            report_s3_key: self.report_s3_name.clone(),
            tech,
            date_init: self.date_init.clone(),
            date_head: self.date_head.clone(),
            loc,
            libs,
            ppl,
            remote_url_hashes: self.remote_url_hashes.clone(),
            recent_project_commits: self.recent_project_commits.clone(),
        }
    }
}
