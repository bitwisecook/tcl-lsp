// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The regexp owner's typed precision result
//! (`docs/design/compiler/value-evaluation.md` § *The typed precision result*
//! and § *The witnesses*): a bounded search answers a completed match, a
//! completed no-match, or why it established neither — and exhaustion is
//! never spelled no-match.
//!
//! The three witnesses are the page's table, checked against every `tclsh`
//! release on `PATH` (8.4 to 9.1); a release that is not installed is
//! skipped, and the engine's own answer is asserted either way.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use tcl_regex::defs::REG_ADVANCED;
use tcl_regex::{ExecLimits, ExecOutcome, ExecStop, Regex, Span};

/// The release series the witnesses are pinned against.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// `script`'s standard output under `tclsh`, or `None` when it is not there.
fn run_tcl(tclsh: &str, script: &str) -> Option<String> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
}

/// Every release on `PATH`, as its `tclsh` name.
fn releases_on_path() -> Vec<String> {
    RELEASES
        .iter()
        .map(|series| format!("tclsh{series}"))
        .filter(|tclsh| run_tcl(tclsh, "puts [info patchlevel]").is_some())
        .collect()
}

fn cps(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

/// The engine's answer for `pattern` over `subject`, from character 0.
fn exec(pattern: &str, subject: &str) -> ExecOutcome {
    Regex::compile_str(pattern, REG_ADVANCED)
        .expect("compiles")
        .exec(&cps(subject), 0, 0)
}

/// A span in Tcl's inclusive `-indices` spelling.
fn indices(span: Option<Span>) -> String {
    match span {
        Some(Span { start, end }) => format!("{start} {}", end.cast_signed() - 1),
        None => "-1 -1".to_owned(),
    }
}

/// The page's three witnesses, each against every release on `PATH`, and
/// the engine's own answer for each: exact where the oracle matches, a
/// completed no-match where it does not, never a stop and never an
/// approximated span.
#[test]
fn the_three_precision_witnesses() {
    let releases = releases_on_path();
    let run_a = "a".repeat(300);
    let run_a_bb = format!("{run_a}bb");
    let run_x = "x".repeat(300);

    // `regexp -indices {^a*(b)\1$} <300×a>bb whole g1`: 1, `0 301`,
    // `300 300` — the backtracking path, whose `a*` once recursed past its
    // depth cap and answered no match.
    let ExecOutcome::Matched(groups) = exec(r"^a*(b)\1$", &run_a_bb) else {
        panic!(
            "^a*(b)\\1$ over 300 a and bb: {:?}",
            exec(r"^a*(b)\1$", &run_a_bb)
        );
    };
    let ours = format!("1 {{{}}} {{{}}}", indices(groups[0]), indices(groups[1]));
    assert_eq!(ours, "1 {0 301} {300 300}");
    for tclsh in &releases {
        let oracle = run_tcl(
            tclsh,
            "set s [string repeat a 300]bb\n\
             puts [list [regexp -indices {^a*(b)\\1$} $s whole g1] $whole $g1]\n",
        );
        assert_eq!(oracle.as_deref(), Some(ours.as_str()), "{tclsh}");
    }

    // `regexp -indices {(x)*} <300×x> whole g1`: 1, `0 299`, `299 299` —
    // exactly, never the span a dissection cap approximated.
    let ExecOutcome::Matched(groups) = exec("(x)*", &run_x) else {
        panic!("(x)* over 300 x: {:?}", exec("(x)*", &run_x));
    };
    let ours = format!("1 {{{}}} {{{}}}", indices(groups[0]), indices(groups[1]));
    assert_eq!(ours, "1 {0 299} {299 299}");
    for tclsh in &releases {
        let oracle = run_tcl(
            tclsh,
            "set s [string repeat x 300]\n\
             puts [list [regexp -indices {(x)*} $s whole g1] $whole $g1]\n",
        );
        assert_eq!(oracle.as_deref(), Some(ours.as_str()), "{tclsh}");
    }

    // `regexp {^(a+)+\1$}` and `regexp {^(a+)+b$}` over 300 and 301 `a`:
    // 1 and 0. The second is the load-bearing one — a search that ran out
    // of budget and called that a no-match would answer the first wrongly.
    for length in [300, 301] {
        let subject = "a".repeat(length);
        assert!(
            matches!(exec(r"^(a+)+\1$", &subject), ExecOutcome::Matched(_)),
            "^(a+)+\\1$ over {length} a: {:?}",
            exec(r"^(a+)+\1$", &subject)
        );
        assert_eq!(
            exec("^(a+)+b$", &subject),
            ExecOutcome::NoMatch,
            "^(a+)+b$ over {length} a completes"
        );
        for tclsh in &releases {
            let oracle = run_tcl(
                tclsh,
                &format!(
                    "set s [string repeat a {length}]\n\
                     puts [list [regexp {{^(a+)+\\1$}} $s] [regexp {{^(a+)+b$}} $s]]\n"
                ),
            );
            assert_eq!(oracle.as_deref(), Some("1 0"), "{tclsh}, {length} a");
        }
    }
    eprintln!("precision witnesses checked against {releases:?}");
}

/// A budget small enough to exhaust yields `Fuel`, never `NoMatch`, on
/// both matching paths — the set simulation (`^(a+)+b$`) and the
/// backtracker (`^(a+)+\1$`, which has a match to find) — and a set
/// cancellation token yields `Cancelled`, a different stop.
#[test]
fn an_exhausted_search_is_never_a_no_match() {
    let subject = cps(&"a".repeat(300));
    let starved = ExecLimits {
        fuel: 50,
        cancel: None,
    };
    for pattern in ["^(a+)+b$", r"^(a+)+\1$"] {
        let re = Regex::compile_str(pattern, REG_ADVANCED).expect("compiles");
        assert_eq!(
            re.exec_with(&subject, 0, 0, &starved),
            ExecOutcome::Stopped(ExecStop::Fuel { spent: 50 }),
            "{pattern}"
        );
        let token = AtomicBool::new(true);
        let cancelled = ExecLimits {
            fuel: tcl_regex::MATCH_FUEL,
            cancel: Some(&token),
        };
        assert_eq!(
            re.exec_with(&subject, 0, 0, &cancelled),
            ExecOutcome::Stopped(ExecStop::Cancelled),
            "{pattern}"
        );
    }
}

/// A search reports the work it spent (`value-evaluation.md` § *Units and
/// charges*: one unit per `MATCH_FUEL` unit spent), on both matching paths —
/// the set simulation and the backtracker: some of the budget for a
/// completed match, all of it for an exhausted one.
#[test]
fn a_search_reports_the_work_it_spent() {
    for (pattern, subject) in [("^(a+)b$", "aaab"), (r"^(a+)\1$", "aaaa")] {
        let re = Regex::compile_str(pattern, REG_ADVANCED).expect("compiles");
        let (outcome, spent) = re.exec_metered(&cps(subject), 0, 0, &ExecLimits::default());
        assert!(matches!(outcome, ExecOutcome::Matched(_)), "{pattern}");
        assert!(
            spent > 0 && spent < tcl_regex::MATCH_FUEL,
            "{pattern}: {spent}"
        );
        let starved = ExecLimits {
            fuel: 3,
            cancel: None,
        };
        assert_eq!(
            re.exec_metered(&cps(&"a".repeat(300)), 0, 0, &starved),
            (ExecOutcome::Stopped(ExecStop::Fuel { spent: 3 }), 3),
            "{pattern}"
        );
    }
}

/// Through the plumbing's engine, searches charged to one counter share one
/// budget: the counter holds what each spent, and the second of two
/// identical searches runs under what the first left — so a `-all` loop
/// cannot spend the budget once per match.
#[cfg(feature = "cmd-core")]
#[test]
fn searches_charged_to_one_counter_share_one_budget() {
    use std::cell::Cell;
    use tcl_cmd_core::regex::{
        MatchLimits, PrecisionDecline, RegexEngine, RegexFlags, RegexpPrecision,
    };
    use tcl_regex::cmd_core::AreEngine;

    for (pattern, subject) in [("^(a+)b$", "aaab"), (r"^(a+)\1$", "aaaa")] {
        let (_, spent) = Regex::compile_str(pattern, REG_ADVANCED)
            .expect("compiles")
            .exec_metered(&cps(subject), 0, 0, &ExecLimits::default());
        let mut compiled =
            AreEngine::compile(pattern.as_bytes(), RegexFlags::default()).expect("compiles");
        let subject: Vec<i32> = subject.chars().map(|c| c as i32).collect();
        let counter = Cell::new(0);
        let shared = MatchLimits {
            fuel: Some(spent + spent / 2),
            spent: Some(&counter),
            ..MatchLimits::default()
        };
        assert!(
            matches!(
                AreEngine::exec_within(&mut compiled, &subject, 0, false, shared),
                RegexpPrecision::Exact { .. }
            ),
            "{pattern}"
        );
        assert_eq!(counter.get(), spent, "{pattern}");
        assert!(
            matches!(
                AreEngine::exec_within(&mut compiled, &subject, 0, false, shared),
                RegexpPrecision::Declined(PrecisionDecline::FuelExhausted { .. })
            ),
            "{pattern}: the second search runs under what the first left"
        );
        assert_eq!(counter.get(), spent + spent / 2, "{pattern}");
    }
}
