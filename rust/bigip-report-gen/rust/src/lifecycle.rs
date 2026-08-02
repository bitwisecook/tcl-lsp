// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! BIG-IP software-branch support milestones from F5 K5903.
//!
//! Reports are deliberately self-contained and make no network requests, so
//! the current policy table is compiled into the generator. Keep the revision
//! date visible in the output: operators can immediately tell when they should
//! re-check K5903 for a newer schedule.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Map, Value as J};

pub const K5903_URL: &str = "https://my.f5.com/manage/s/article/K5903";
const POLICY_UPDATED: &str = "2026-07-01";
const WARN_DAYS: i64 = 365;

#[derive(Clone, Copy)]
struct BranchLifecycle {
    branch: &'static str,
    released: &'static str,
    eosd: &'static str,
    eots: &'static str,
}

// First-customer-ship dates and support milestones. The actively supported
// rows are from K5903's 2026-07-01 table; recently retired rows are retained so
// reports for old estates say "EoL" instead of silently omitting lifecycle
// context. EoL is reached at EoTS under the policy.
const BRANCHES: &[BranchLifecycle] = &[
    BranchLifecycle {
        branch: "13.0.x",
        released: "2017-02-03",
        eosd: "2018-05-22",
        eots: "2019-05-22",
    },
    BranchLifecycle {
        branch: "13.1.x",
        released: "2017-12-19",
        eosd: "2022-12-31",
        eots: "2023-12-31",
    },
    BranchLifecycle {
        branch: "14.0.x",
        released: "2018-08-10",
        eosd: "2019-11-09",
        eots: "2019-11-09",
    },
    BranchLifecycle {
        branch: "14.1.x",
        released: "2018-12-11",
        eosd: "2023-12-31",
        eots: "2023-12-31",
    },
    BranchLifecycle {
        branch: "15.0.x",
        released: "2019-05-23",
        eosd: "2020-08-23",
        eots: "2020-08-23",
    },
    BranchLifecycle {
        branch: "15.1.x",
        released: "2019-12-11",
        eosd: "2024-12-31",
        eots: "2024-12-31",
    },
    BranchLifecycle {
        branch: "16.0.x",
        released: "2020-07-07",
        eosd: "2021-10-07",
        eots: "2021-10-07",
    },
    BranchLifecycle {
        branch: "16.1.x",
        released: "2021-07-07",
        eosd: "2025-07-31",
        eots: "2025-07-31",
    },
    BranchLifecycle {
        branch: "17.0.x",
        released: "2022-04-26",
        eosd: "2023-07-31",
        eots: "2023-07-31",
    },
    BranchLifecycle {
        branch: "17.1.x",
        released: "2023-03-14",
        eosd: "2027-03-31",
        eots: "2027-03-31",
    },
    BranchLifecycle {
        branch: "17.5.x",
        released: "2025-02-27",
        eosd: "2029-01-01",
        eots: "2029-01-01",
    },
    BranchLifecycle {
        branch: "21.0.x",
        released: "2025-11-06",
        eosd: "2026-08-06",
        eots: "2026-08-06",
    },
    BranchLifecycle {
        branch: "21.1.x",
        released: "2026-05-05",
        eosd: "2029-05-05",
        eots: "2029-05-05",
    },
];

fn parse_date(value: &str) -> NaiveDate {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("static K5903 date is valid")
}

fn today_utc() -> NaiveDate {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    DateTime::<Utc>::from_timestamp(i64::try_from(seconds).unwrap_or(i64::MAX), 0)
        .expect("current time is representable")
        .date_naive()
}

fn branch_for(version: &str) -> Option<&'static BranchLifecycle> {
    let mut parts = version.trim().split('.');
    let major = parts.next()?.parse::<u16>().ok()?;
    let minor = parts.next()?.parse::<u16>().ok()?;
    BRANCHES.iter().find(|entry| {
        let prefix = entry.branch.trim_end_matches('x').trim_end_matches('.');
        prefix == format!("{major}.{minor}")
    })
}

fn lifecycle_at(version: &str, today: NaiveDate) -> Option<J> {
    let entry = branch_for(version)?;
    let eosd = parse_date(entry.eosd);
    let eots = parse_date(entry.eots);
    let days_until_eosd = (eosd - today).num_days();
    let days_until_eots = (eots - today).num_days();

    let (status, level, text) = if days_until_eots <= 0 {
        let days = -days_until_eots;
        (
            "eol",
            "danger",
            format!(
                "This BIG-IP branch reached EoTS and completed its EoL lifecycle {days} day{} ago.",
                if days == 1 { "" } else { "s" }
            ),
        )
    } else if days_until_eosd <= 0 {
        (
            "eosd",
            "danger",
            format!(
                "This BIG-IP branch has reached EoSD; technical support ends in {days_until_eots} days."
            ),
        )
    } else if days_until_eosd <= WARN_DAYS {
        (
            "nearing-eosd",
            "warn",
            format!(
                "This BIG-IP branch reaches EoSD/EoTS/EoL in {days_until_eosd} days. Plan an upgrade."
            ),
        )
    } else {
        (
            "supported",
            "ok",
            format!(
                "This BIG-IP branch is in standard support; EoSD is in {days_until_eosd} days."
            ),
        )
    };

    let mut out = Map::new();
    out.insert("branch".into(), J::String(entry.branch.into()));
    out.insert("releaseDate".into(), J::String(entry.released.into()));
    out.insert("eosdDate".into(), J::String(entry.eosd.into()));
    out.insert("eotsDate".into(), J::String(entry.eots.into()));
    out.insert("eolDate".into(), J::String(entry.eots.into()));
    out.insert("status".into(), J::String(status.into()));
    out.insert("level".into(), J::String(level.into()));
    out.insert("text".into(), J::String(text));
    out.insert("daysUntilEosd".into(), J::from(days_until_eosd));
    out.insert("daysUntilEots".into(), J::from(days_until_eots));
    out.insert("policyUpdated".into(), J::String(POLICY_UPDATED.into()));
    out.insert("sourceUrl".into(), J::String(K5903_URL.into()));
    out.insert(
        "scopeNote".into(),
        J::String(
            "Software lifecycle only; hardware and FIPS exceptions have independent dates.".into(),
        ),
    );
    Some(J::Object(out))
}

/// Return report-ready K5903 lifecycle metadata for a BIG-IP version.
#[must_use]
pub fn bigip_release_lifecycle(version: &str) -> J {
    lifecycle_at(version, today_utc()).unwrap_or(J::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        parse_date(value)
    }

    #[test]
    fn maps_maintenance_versions_to_their_policy_branch() {
        let got = lifecycle_at("21.1.0.1", date("2026-08-02")).unwrap();
        assert_eq!(got["branch"], "21.1.x");
        assert_eq!(got["releaseDate"], "2026-05-05");
        assert_eq!(got["eolDate"], "2029-05-05");
        assert_eq!(got["status"], "supported");
    }

    #[test]
    fn warns_within_a_year_and_marks_eol() {
        let near = lifecycle_at("17.1.3", date("2026-08-02")).unwrap();
        assert_eq!(near["status"], "nearing-eosd");
        assert_eq!(near["level"], "warn");

        let ended = lifecycle_at("16.1.6", date("2026-08-02")).unwrap();
        assert_eq!(ended["status"], "eol");
        assert_eq!(ended["level"], "danger");
    }

    #[test]
    fn unknown_or_malformed_versions_have_no_policy_guess() {
        assert!(lifecycle_at("18.0.0", date("2026-08-02")).is_none());
        assert!(lifecycle_at("not-a-version", date("2026-08-02")).is_none());
    }
}
