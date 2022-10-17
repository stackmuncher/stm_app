use super::Report;
use crate::graphql::RustScalarValue;
use chrono::{self, Duration, TimeZone, Timelike, Utc};
use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Number of days for including a commit in the recent counts.
pub const RECENT_PERIOD_LENGTH_IN_DAYS: i64 = 365;

/// Number of commits or percentage of commits per UTC hour.
/// The structure is skipped in JSON if all values are zero and is initialized to all zeros to have fewer Option<T> unwraps.
#[derive(Serialize, Deserialize, Clone, Debug, GraphQLObject)]
#[graphql(scalar = RustScalarValue)]
pub struct CommitTimeHistoHours {
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h00: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h01: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h02: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h03: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h04: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h05: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h06: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h07: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h08: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h09: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h10: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h11: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h12: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h13: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h14: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h15: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h16: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h17: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h18: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h19: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h20: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h21: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h22: u64,
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub h23: u64,
}

/// Contains members and methods related to commit time histogram
#[derive(Serialize, Deserialize, Clone, Debug, GraphQLObject)]
#[graphql(scalar = RustScalarValue)]
pub struct CommitTimeHisto {
    /// The sum of all commits included in `histogram_recent`. This value is used as the 100% of all recent commits.
    /// The value is populated once after all commits have been added.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub histogram_recent_sum: u64,
    /// The sum of all commits included in `histogram_all`. This value is used as the 100% of all commits.
    /// The value is populated once after all commits have been added.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero", default = "u64::default")]
    pub histogram_all_sum: u64,

    /// The standard deviation of `histogram_recent` values.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero_f64", default = "f64::default")]
    pub histogram_recent_std: f64,
    /// The standard deviation of `histogram_all` values.
    #[serde(skip_serializing_if = "CommitTimeHistoHours::is_zero_f64", default = "f64::default")]
    pub histogram_all_std: f64,

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

    /// Number of working hours in overlap between the dev's active time and the 8am - 6pm working day in the specified time zone.
    /// Timezones with negative offset are represented as 24-offset. E.g. `-6` hours will be in `h18`.
    /// Only activity within standard deviation is included.
    #[serde(
        skip_serializing_if = "CommitTimeHistoHours::is_empty",
        default = "CommitTimeHistoHours::default"
    )]
    pub timezone_overlap_recent: CommitTimeHistoHours,
    /// Number of working hours in overlap between the dev's active time and the 8am - 6pm working day in the specified time zone.
    /// Timezones with negative offset are represented as 24-offset + the negative value. E.g. `-6` hours will be in `h18` (24-6-18).
    /// Only activity within standard deviation is included.
    #[serde(
        skip_serializing_if = "CommitTimeHistoHours::is_empty",
        default = "CommitTimeHistoHours::default"
    )]
    pub timezone_overlap_all: CommitTimeHistoHours,
}

impl CommitTimeHistoHours {
    /// A helper function for serde. Returns true if the value is zero.
    fn is_zero(val: &u64) -> bool {
        val == &0
    }

    /// A helper function for serde. Returns true if the value is zero.
    fn is_zero_f64(val: &f64) -> bool {
        val == &0.0
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

    /// Converts `hxx` values from the number of commits to percentage.
    fn convert_counts_to_percentage(&mut self, sum: u64) {
        if sum == 0 {
            return;
        }

        // convert the sum into f64 to allow for rounding of the fraction after doing division
        let sum = sum as f64;
        // re-calculate every member
        self.h00 = (self.h00 as f64 * 100.0 / sum).round() as u64;
        self.h01 = (self.h01 as f64 * 100.0 / sum).round() as u64;
        self.h02 = (self.h02 as f64 * 100.0 / sum).round() as u64;
        self.h03 = (self.h03 as f64 * 100.0 / sum).round() as u64;
        self.h04 = (self.h04 as f64 * 100.0 / sum).round() as u64;
        self.h05 = (self.h05 as f64 * 100.0 / sum).round() as u64;
        self.h06 = (self.h06 as f64 * 100.0 / sum).round() as u64;
        self.h07 = (self.h07 as f64 * 100.0 / sum).round() as u64;
        self.h08 = (self.h08 as f64 * 100.0 / sum).round() as u64;
        self.h09 = (self.h09 as f64 * 100.0 / sum).round() as u64;
        self.h10 = (self.h10 as f64 * 100.0 / sum).round() as u64;
        self.h11 = (self.h11 as f64 * 100.0 / sum).round() as u64;
        self.h12 = (self.h12 as f64 * 100.0 / sum).round() as u64;
        self.h13 = (self.h13 as f64 * 100.0 / sum).round() as u64;
        self.h14 = (self.h14 as f64 * 100.0 / sum).round() as u64;
        self.h15 = (self.h15 as f64 * 100.0 / sum).round() as u64;
        self.h16 = (self.h16 as f64 * 100.0 / sum).round() as u64;
        self.h17 = (self.h17 as f64 * 100.0 / sum).round() as u64;
        self.h18 = (self.h18 as f64 * 100.0 / sum).round() as u64;
        self.h19 = (self.h19 as f64 * 100.0 / sum).round() as u64;
        self.h20 = (self.h20 as f64 * 100.0 / sum).round() as u64;
        self.h21 = (self.h21 as f64 * 100.0 / sum).round() as u64;
        self.h22 = (self.h22 as f64 * 100.0 / sum).round() as u64;
        self.h23 = (self.h23 as f64 * 100.0 / sum).round() as u64;
    }

    /// Converts `hxx` values from the number of commits to percentage.
    fn standard_deviation(&mut self, mean: f64) -> f64 {
        if mean == 0.0 {
            return 0.0;
        }

        // calculate variance
        let variance = (self.h00 as f64 - mean).powi(2)
            + (self.h01 as f64 - mean).powi(2)
            + (self.h02 as f64 - mean).powi(2)
            + (self.h03 as f64 - mean).powi(2)
            + (self.h04 as f64 - mean).powi(2)
            + (self.h05 as f64 - mean).powi(2)
            + (self.h06 as f64 - mean).powi(2)
            + (self.h07 as f64 - mean).powi(2)
            + (self.h08 as f64 - mean).powi(2)
            + (self.h09 as f64 - mean).powi(2)
            + (self.h10 as f64 - mean).powi(2)
            + (self.h11 as f64 - mean).powi(2)
            + (self.h12 as f64 - mean).powi(2)
            + (self.h13 as f64 - mean).powi(2)
            + (self.h14 as f64 - mean).powi(2)
            + (self.h15 as f64 - mean).powi(2)
            + (self.h16 as f64 - mean).powi(2)
            + (self.h17 as f64 - mean).powi(2)
            + (self.h18 as f64 - mean).powi(2)
            + (self.h19 as f64 - mean).powi(2)
            + (self.h20 as f64 - mean).powi(2)
            + (self.h21 as f64 - mean).powi(2)
            + (self.h22 as f64 - mean).powi(2)
            + (self.h23 as f64 - mean).powi(2);

        (variance / 24.0).sqrt()
    }

    /// Returns the sum of all `hxx` members.
    fn sum(&self) -> u64 {
        self.h00
            + self.h01
            + self.h02
            + self.h03
            + self.h04
            + self.h05
            + self.h06
            + self.h07
            + self.h08
            + self.h09
            + self.h10
            + self.h11
            + self.h12
            + self.h13
            + self.h14
            + self.h15
            + self.h16
            + self.h17
            + self.h18
            + self.h19
            + self.h20
            + self.h21
            + self.h22
            + self.h23
    }

    /// Calculates how many working (8am - 6pm) hours overlap between commit time and the target timezone.
    /// Only commit hours above the standard deviation (std) are included.
    fn overlap(&self, std: f64) -> Self {
        // copy the commit counts into an array for easy referencing in a loop
        let commit_counts: [u64; 24] = [
            self.h00, self.h01, self.h02, self.h03, self.h04, self.h05, self.h06, self.h07, self.h08, self.h09,
            self.h10, self.h11, self.h12, self.h13, self.h14, self.h15, self.h16, self.h17, self.h18, self.h19,
            self.h20, self.h21, self.h22, self.h23,
        ];

        let mut tz_overlap: [u64; 24] = [0; 24];

        // populate an array for all possible timezones with the number of overlapping hours
        for tz in 0..23 {
            tz_overlap[tz] = commit_counts
                .iter()
                .enumerate()
                .map(|(hr, v)| {
                    // normalize the UTC time of the commit counts to the working hours of the target timezone
                    // e.g. 18hr for UTC+12 = 30
                    let hr = hr + tz;
                    // e.g. 30 - 24 = 6am UTC
                    let hr = if hr > 23 { hr - 24 } else { hr };
                    // hours between 8am and 6pm of the target timezone where the number of commits is above the standard deviation
                    if hr >= 8 && hr < 18 && *v as f64 > std {
                        1
                    } else {
                        0
                    }
                })
                .sum::<u64>();
        }

        Self {
            h00: tz_overlap[0],
            h01: tz_overlap[1],
            h02: tz_overlap[2],
            h03: tz_overlap[3],
            h04: tz_overlap[4],
            h05: tz_overlap[5],
            h06: tz_overlap[6],
            h07: tz_overlap[7],
            h08: tz_overlap[8],
            h09: tz_overlap[9],
            h10: tz_overlap[10],
            h11: tz_overlap[11],
            h12: tz_overlap[12],
            h13: tz_overlap[13],
            h14: tz_overlap[14],
            h15: tz_overlap[15],
            h16: tz_overlap[16],
            h17: tz_overlap[17],
            h18: tz_overlap[18],
            h19: tz_overlap[19],
            h20: tz_overlap[20],
            h21: tz_overlap[21],
            h22: tz_overlap[22],
            h23: tz_overlap[23],
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
                    timezone_overlap_recent: CommitTimeHistoHours::default(),
                    timezone_overlap_all: CommitTimeHistoHours::default(),
                    histogram_recent_std: 0.0,
                    histogram_all_std: 0.0,
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
        self.histogram_recent_sum = self.histogram_recent.sum();
        let mean_recent = self.histogram_recent_sum as f64 / 24.0;
        self.histogram_recent_std = self.histogram_recent.standard_deviation(mean_recent);
        self.timezone_overlap_recent = self.histogram_recent.overlap(self.histogram_recent_std);
        self.histogram_recent
            .convert_counts_to_percentage(self.histogram_recent_sum);

        self.histogram_all_sum = self.histogram_all.sum();
        let mean_all = self.histogram_all_sum as f64 / 24.0;
        self.histogram_all_std = self.histogram_all.standard_deviation(mean_all);
        self.timezone_overlap_all = self.histogram_all.overlap(self.histogram_all_std);
        self.histogram_all.convert_counts_to_percentage(self.histogram_all_sum);
    }
}
