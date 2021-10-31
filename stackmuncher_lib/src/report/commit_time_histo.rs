use super::Report;
use chrono::{self, Duration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Number of days for including a commit in the recent counts.
pub const RECENT_PERIOD_LENGTH_IN_DAYS: i64 = 365;

/// Number of commits or percentage of commits per UTC hour.
/// The structure is skipped in JSON if all values are zero and is initialized to all zeros to have fewer Option<T> unwraps.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitTimeHistoHours {
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h00: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h01: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h02: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h03: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h04: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h05: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h06: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h07: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h08: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h09: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h10: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h11: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h12: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h13: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h14: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h15: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h16: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h17: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h18: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h19: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h20: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h21: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h22: usize,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub h23: usize,
}

/// Contains members and methods related to commit time histogram
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CommitTimeHisto {
    /// Initially, the contains the number of commits per UTC hour for the last N days as defined in `RECENT_PERIOD_LENGTH_IN_DAYS` const.
    /// Later the values are recalculated to percentages for storing in the DB.
    #[serde(
        skip_serializing_if = "CommitTimeHistoHours::is_empty",
        default = "CommitTimeHistoHours::default"
    )]
    pub histogram_recent: CommitTimeHistoHours,
    /// Initially, the contains the number of commits per UTC hour for the entire commits history.
    /// Later the values are recalculated to percentages for storing in the DB.
    #[serde(
        skip_serializing_if = "CommitTimeHistoHours::is_empty",
        default = "CommitTimeHistoHours::default"
    )]
    pub histogram_all: CommitTimeHistoHours,
    /// The sum of all commits included in `histogram_recent`. This value is used as the 100% of all recent commits.
    /// The value is populated once after all commits have been added.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub histogram_recent_sum: usize,
    /// The sum of all commits included in `histogram_all`. This value is used as the 100% of all commits.
    /// The value is populated once after all commits have been added.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "usize::default")]
    pub histogram_all_sum: usize,
}

impl CommitTimeHistoHours {
    /// A helper function for serde. Returns true if the value is zero.
    fn is_zero(val: &usize) -> bool {
        val == &0
    }

    /// A helper function for serde. Returns true if all members have value of zero.
    fn is_empty(&self) -> bool {
        if self.h00 > 0
            || self.h01 > 0
            || self.h02 > 0
            || self.h03 > 0
            || self.h04 > 0
            || self.h05 > 0
            || self.h06 > 0
            || self.h07 > 0
            || self.h08 > 0
            || self.h09 > 0
            || self.h10 > 0
            || self.h11 > 0
            || self.h12 > 0
            || self.h13 > 0
            || self.h14 > 0
            || self.h15 > 0
            || self.h16 > 0
            || self.h17 > 0
            || self.h18 > 0
            || self.h19 > 0
            || self.h20 > 0
            || self.h21 > 0
            || self.h22 > 0
            || self.h23 > 0
        {
            false
        } else {
            true
        }
    }

    /// Updates the counts for the specified hour. Panics if `hour > 23`.
    fn add_commit(&mut self, hour: u32) {
        match hour {
            0 => self.h00 += 1,
            1 => self.h01 += 1,
            2 => self.h02 += 1,
            3 => self.h03 += 1,
            4 => self.h04 += 1,
            5 => self.h05 += 1,
            6 => self.h06 += 1,
            7 => self.h07 += 1,
            8 => self.h08 += 1,
            9 => self.h09 += 1,
            10 => self.h10 += 1,
            11 => self.h11 += 1,
            12 => self.h12 += 1,
            13 => self.h13 += 1,
            14 => self.h14 += 1,
            15 => self.h15 += 1,
            16 => self.h16 += 1,
            17 => self.h17 += 1,
            18 => self.h18 += 1,
            19 => self.h19 += 1,
            20 => self.h20 += 1,
            21 => self.h21 += 1,
            22 => self.h22 += 1,
            23 => self.h23 += 1,
            _ => panic!("Invalid value for HOUR: {}. ts.time().hour() should never return > 23.", hour),
        }
    }
}

impl Default for CommitTimeHistoHours {
    fn default() -> Self {
        Self {
            h00: 0,
            h01: 0,
            h02: 0,
            h03: 0,
            h04: 0,
            h05: 0,
            h06: 0,
            h07: 0,
            h08: 0,
            h09: 0,
            h10: 0,
            h11: 0,
            h12: 0,
            h13: 0,
            h14: 0,
            h15: 0,
            h16: 0,
            h17: 0,
            h18: 0,
            h19: 0,
            h20: 0,
            h21: 0,
            h22: 0,
            h23: 0,
        }
    }
}

impl CommitTimeHisto {
    /// Adds the time from the list of commits to the histogram structure.
    /// Logs any errors and warnings and returns regardless of success of failure.
    pub(crate) fn add_commits(report: &mut Report, commits: &Option<Vec<String>>) {
        // is there anything to add?
        if let Some(commits) = commits {
            // init the histo structure if there is none
            if report.commit_time_histo.is_none() {
                report.commit_time_histo = Some(CommitTimeHisto {
                    histogram_recent: CommitTimeHistoHours::default(),
                    histogram_all: CommitTimeHistoHours::default(),
                    histogram_recent_sum: 0,
                    histogram_all_sum: 0,
                });
            }

            let histo = report
                .commit_time_histo
                .as_mut()
                .expect("report.commit_time_histo should exist by now. It's a bug.");

            // update the commit time histogram
            let now = Utc::now();
            let recent_period_start = now - Duration::days(RECENT_PERIOD_LENGTH_IN_DAYS);
            for commit in commits {
                if let Some((_, ts)) = commit.split_once("_") {
                    if let Ok(ts) = i64::from_str_radix(ts, 10) {
                        let ts = Utc.timestamp(ts, 0);
                        // update recent commits histo if the TS is within the recent period
                        if ts > recent_period_start && ts < now {
                            histo.histogram_recent.add_commit(ts.time().hour());
                        }
                        // update all commits histo
                        histo.histogram_all.add_commit(ts.time().hour());
                    } else {
                        warn!("Invalid time part in commit {}.", ts);
                    }
                } else {
                    warn!("No time part in commit {}.", commit);
                }
            }
        } else {
            warn!("No commit info in proj overview.");
        }
    }

    /// Calculates the percentage of each bucket from the total sum of commits in the histogram for `_recent` and `_all`.
    pub(crate) fn recalculate_counts_to_percentage(&mut self) {
        self.histogram_recent_sum = self.histogram_recent.h00
            + self.histogram_recent.h01
            + self.histogram_recent.h02
            + self.histogram_recent.h03
            + self.histogram_recent.h04
            + self.histogram_recent.h05
            + self.histogram_recent.h06
            + self.histogram_recent.h07
            + self.histogram_recent.h08
            + self.histogram_recent.h09
            + self.histogram_recent.h10
            + self.histogram_recent.h11
            + self.histogram_recent.h12
            + self.histogram_recent.h13
            + self.histogram_recent.h14
            + self.histogram_recent.h15
            + self.histogram_recent.h16
            + self.histogram_recent.h17
            + self.histogram_recent.h18
            + self.histogram_recent.h19
            + self.histogram_recent.h20
            + self.histogram_recent.h21
            + self.histogram_recent.h22
            + self.histogram_recent.h23;

        self.histogram_all_sum = self.histogram_all.h00
            + self.histogram_all.h01
            + self.histogram_all.h02
            + self.histogram_all.h03
            + self.histogram_all.h04
            + self.histogram_all.h05
            + self.histogram_all.h06
            + self.histogram_all.h07
            + self.histogram_all.h08
            + self.histogram_all.h09
            + self.histogram_all.h10
            + self.histogram_all.h11
            + self.histogram_all.h12
            + self.histogram_all.h13
            + self.histogram_all.h14
            + self.histogram_all.h15
            + self.histogram_all.h16
            + self.histogram_all.h17
            + self.histogram_all.h18
            + self.histogram_all.h19
            + self.histogram_all.h20
            + self.histogram_all.h21
            + self.histogram_all.h22
            + self.histogram_all.h23;

        if self.histogram_recent_sum > 0 {
            self.histogram_recent.h00 = self.histogram_recent.h00 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h01 = self.histogram_recent.h01 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h02 = self.histogram_recent.h02 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h03 = self.histogram_recent.h03 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h04 = self.histogram_recent.h04 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h05 = self.histogram_recent.h05 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h06 = self.histogram_recent.h06 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h07 = self.histogram_recent.h07 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h08 = self.histogram_recent.h08 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h09 = self.histogram_recent.h09 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h10 = self.histogram_recent.h10 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h11 = self.histogram_recent.h11 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h12 = self.histogram_recent.h12 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h13 = self.histogram_recent.h13 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h14 = self.histogram_recent.h14 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h15 = self.histogram_recent.h15 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h16 = self.histogram_recent.h16 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h17 = self.histogram_recent.h17 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h18 = self.histogram_recent.h18 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h19 = self.histogram_recent.h19 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h20 = self.histogram_recent.h20 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h21 = self.histogram_recent.h21 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h22 = self.histogram_recent.h22 * 100 / self.histogram_recent_sum;
            self.histogram_recent.h23 = self.histogram_recent.h23 * 100 / self.histogram_recent_sum;
        }

        if self.histogram_all_sum > 0 {
            self.histogram_all.h00 = self.histogram_all.h00 * 100 / self.histogram_all_sum;
            self.histogram_all.h01 = self.histogram_all.h01 * 100 / self.histogram_all_sum;
            self.histogram_all.h02 = self.histogram_all.h02 * 100 / self.histogram_all_sum;
            self.histogram_all.h03 = self.histogram_all.h03 * 100 / self.histogram_all_sum;
            self.histogram_all.h04 = self.histogram_all.h04 * 100 / self.histogram_all_sum;
            self.histogram_all.h05 = self.histogram_all.h05 * 100 / self.histogram_all_sum;
            self.histogram_all.h06 = self.histogram_all.h06 * 100 / self.histogram_all_sum;
            self.histogram_all.h07 = self.histogram_all.h07 * 100 / self.histogram_all_sum;
            self.histogram_all.h08 = self.histogram_all.h08 * 100 / self.histogram_all_sum;
            self.histogram_all.h09 = self.histogram_all.h09 * 100 / self.histogram_all_sum;
            self.histogram_all.h10 = self.histogram_all.h10 * 100 / self.histogram_all_sum;
            self.histogram_all.h11 = self.histogram_all.h11 * 100 / self.histogram_all_sum;
            self.histogram_all.h12 = self.histogram_all.h12 * 100 / self.histogram_all_sum;
            self.histogram_all.h13 = self.histogram_all.h13 * 100 / self.histogram_all_sum;
            self.histogram_all.h14 = self.histogram_all.h14 * 100 / self.histogram_all_sum;
            self.histogram_all.h15 = self.histogram_all.h15 * 100 / self.histogram_all_sum;
            self.histogram_all.h16 = self.histogram_all.h16 * 100 / self.histogram_all_sum;
            self.histogram_all.h17 = self.histogram_all.h17 * 100 / self.histogram_all_sum;
            self.histogram_all.h18 = self.histogram_all.h18 * 100 / self.histogram_all_sum;
            self.histogram_all.h19 = self.histogram_all.h19 * 100 / self.histogram_all_sum;
            self.histogram_all.h20 = self.histogram_all.h20 * 100 / self.histogram_all_sum;
            self.histogram_all.h21 = self.histogram_all.h21 * 100 / self.histogram_all_sum;
            self.histogram_all.h22 = self.histogram_all.h22 * 100 / self.histogram_all_sum;
            self.histogram_all.h23 = self.histogram_all.h23 * 100 / self.histogram_all_sum;
        }
    }
}
