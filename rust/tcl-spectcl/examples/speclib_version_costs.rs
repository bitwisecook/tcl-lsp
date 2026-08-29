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

//! What the `SpecTcl` 2.0 vocabulary costs the bundled corpus.
//!
//! Every shipped pack is written in the 1.x vocabulary. The 2.0 rewrite says
//! the same facts in the algebra (`available {tcl 8.6-}` for `dialects
//! tcl8.6+`), so the two files must load to the same registry — and the
//! question this answers is what the second spelling costs to read.
//!
//! Three numbers per pack, each over the *same* source read twice:
//!
//!   - **1.x** — the file as it ships.
//!   - **2.0** — `tcl spec upgrade`'s rewrite of it, evaluated the same way.
//!   - **2.0 (VM)** — the same 2.0 source with the static fast path off, so
//!     the pack really is executed as a Tcl program by `tcl-vm` rather than
//!     captured from its CST. This is the honest upper bound on what design
//!     E's "a pack is a Tcl program" costs when nothing short-circuits it.
//!
//! Wall time is the median of `--runs` loads; memory is the resident bytes
//! one load **retains** — the loader interns what it registers, so that is
//! what a pack costs the process for as long as it runs.
//!
//! Run: `cargo run --release -p tcl-spectcl --example speclib_version_costs`

use std::time::{Duration, Instant};

use tcl_spectcl::{
    EvalOptions, UpgradeOptions, UpgradeStatus, evaluate_pack_with, upgrade_source,
};

/// Resident bytes, from `/proc/self/statm`.
///
/// The loader interns every spec it builds into `&'static` storage, so a load
/// **retains** what it registers: resident growth across N loads, divided by
/// N, is the memory one pack costs the process for as long as it runs. That
/// is the number an editor pays; a transient allocation peak is not.
fn resident_bytes() -> usize {
    let statm = std::fs::read_to_string("/proc/self/statm").unwrap_or_default();
    let pages: usize = statm
        .split_whitespace()
        .nth(1)
        .and_then(|field| field.parse().ok())
        .unwrap_or(0);
    pages * 4096
}

/// One measured way of loading one pack.
struct Measured {
    median: Duration,
    retained: usize,
    commands: usize,
}

fn measure(source: &str, fast_path: bool, runs: usize) -> Measured {
    let options = EvalOptions {
        static_fast_path: fast_path,
        ..EvalOptions::default()
    };
    // One warm load first: it pays the one-off costs (lazy statics, the
    // interner's first pages) that would otherwise land on run 1.
    let commands = evaluate_pack_with(source, &options).commands.len();
    let before = resident_bytes();
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            let pack = evaluate_pack_with(source, &options);
            let elapsed = start.elapsed();
            drop(pack);
            elapsed
        })
        .collect();
    let retained = resident_bytes().saturating_sub(before) / runs;
    times.sort_unstable();
    Measured {
        median: times[times.len() / 2],
        retained,
        commands,
    }
}

fn kib(bytes: usize) -> String {
    format!("{}", bytes / 1024)
}

fn ms(duration: Duration) -> String {
    format!("{:.0}", duration.as_secs_f64() * 1000.0)
}

/// `+N%` / `-N%` of `now` against `was`.
fn delta(was: Duration, now: Duration) -> String {
    let was = was.as_secs_f64();
    if was == 0.0 {
        return "—".to_owned();
    }
    let percent = (now.as_secs_f64() - was) / was * 100.0;
    format!("{percent:+.0}%")
}

fn main() {
    let runs: usize = std::env::args()
        .skip_while(|arg| arg != "--runs")
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(5);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("specs");
    let mut packs: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "tclspec"))
        .collect();
    packs.sort();

    println!("| pack | lines | commands | 1.x ms | 2.0 ms | Δ | 2.0 VM ms | 1.x KiB | 2.0 KiB | 2.0 VM KiB |");
    println!("|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|");
    let mut totals = [Duration::ZERO; 3];
    let mut retained = [0usize; 3];
    for path in &packs {
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        let legacy = std::fs::read_to_string(path).expect("a readable pack");
        let outcome = upgrade_source(&legacy, &UpgradeOptions::default());
        assert!(
            matches!(outcome.status, UpgradeStatus::Upgraded | UpgradeStatus::AlreadyCurrent),
            "{name}: {:?} {:?}",
            outcome.status,
            outcome.refusals
        );
        let modern = outcome.source;

        let old = measure(&legacy, true, runs);
        let new = measure(&modern, true, runs);
        let vm = measure(&modern, false, runs);
        assert_eq!(
            old.commands, new.commands,
            "{name}: the rewrite must register the same commands"
        );
        assert_eq!(old.commands, vm.commands, "{name}: so must the VM route");

        println!(
            "| `{name}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            legacy.lines().count(),
            old.commands,
            ms(old.median),
            ms(new.median),
            delta(old.median, new.median),
            ms(vm.median),
            kib(old.retained),
            kib(new.retained),
            kib(vm.retained),
        );
        for (total, measured) in totals.iter_mut().zip([&old, &new, &vm]) {
            *total += measured.median;
        }
        for (total, measured) in retained.iter_mut().zip([&old, &new, &vm]) {
            *total += measured.retained;
        }
    }
    println!(
        "| **corpus** | | | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** | **{}** |",
        ms(totals[0]),
        ms(totals[1]),
        delta(totals[0], totals[1]),
        ms(totals[2]),
        kib(retained[0]),
        kib(retained[1]),
        kib(retained[2]),
    );
}
