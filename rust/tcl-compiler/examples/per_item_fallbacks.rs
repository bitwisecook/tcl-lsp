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

//! How often does incremental analysis give up, and what does that cost?
//!
//! `per_item_divergence` asks whether the fast path is *correct*. This asks
//! whether it is *taken* — a different question with a different answer, and
//! the one that decides per-keystroke latency: every fallback re-walks the
//! whole document on every edit, making the per-body memoisation above it dead
//! weight for that file.
//!
//! Reports the [`PerItemFallback`] histogram over a corpus, weighted three ways
//! — by file, by source line, and by measured milliseconds — because they rank
//! the gates differently: a gate that fires on a few very large files can be
//! rare by count and dominant by time, and time is what the user feels.
//!
//! Usage: `cargo run --release --example per_item_fallbacks` (corpus: `tmp/`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::per_item::PerItemFallback;

fn gather_tcl(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if out.len() >= cap {
            return;
        }
        if path.is_dir() {
            gather_tcl(&path, out, cap);
        } else if path.extension().is_some_and(|e| e == "tcl") {
            out.push(path);
        }
    }
}

/// One corpus file's verdict.
struct Row {
    reason: Option<PerItemFallback>,
    lines: usize,
    ms: f64,
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let dialect = "tcl8.6";
    let cap: usize = std::env::var("CAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    let mut files = Vec::new();
    gather_tcl(&root, &mut files, cap);

    let mut rows = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.trim().is_empty() {
            continue;
        }
        let mut a = Analyser::new();
        let start = Instant::now();
        let _ = a.analyse_per_item(&src, dialect);
        rows.push(Row {
            reason: a.per_item_fallback,
            lines: src.lines().count(),
            ms: start.elapsed().as_secs_f64() * 1000.0,
        });
    }

    let files_n = rows.len();
    let lines_n: usize = rows.iter().map(|r| r.lines).sum();
    let ms_n: f64 = rows.iter().map(|r| r.ms).sum();
    let fast: Vec<&Row> = rows.iter().filter(|r| r.reason.is_none()).collect();
    println!("corpus: {files_n} files, {lines_n} lines, {ms_n:.0} ms total (dialect {dialect})\n");

    println!(
        "  {:<24} {:>7} {:>7}   {:>9} {:>7}   {:>9} {:>7}",
        "outcome", "files", "%", "lines", "%", "ms", "%"
    );
    // Counts are file/line tallies, far below `f64`'s exact-integer range, so the
    // narrowing through `u32` is lossless here and keeps the cast lints quiet.
    let ratio = |part: usize, whole: usize| -> f64 {
        if whole == 0 {
            return 0.0;
        }
        f64::from(u32::try_from(part).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(whole).unwrap_or(u32::MAX))
            * 100.0
    };
    let pct = |part: f64, whole: f64| {
        if whole > 0.0 {
            part / whole * 100.0
        } else {
            0.0
        }
    };
    let report = |label: &str, sel: &dyn Fn(&Row) -> bool| {
        let f = rows.iter().filter(|r| sel(r)).count();
        if f == 0 {
            return;
        }
        let l: usize = rows.iter().filter(|r| sel(r)).map(|r| r.lines).sum();
        let m: f64 = rows.iter().filter(|r| sel(r)).map(|r| r.ms).sum();
        println!(
            "  {label:<24} {f:>7} {:>6.1}% {l:>9} {:>6.1}% {m:>9.0} {:>6.1}%",
            ratio(f, files_n),
            ratio(l, lines_n),
            pct(m, ms_n),
        );
    };
    report("FAST PATH", &|r: &Row| r.reason.is_none());
    report("fell back (all)", &|r: &Row| r.reason.is_some());
    println!();

    // Per-gate breakdown, ordered by the cost that actually matters: time.
    let mut by_reason: BTreeMap<&'static str, (usize, usize, f64)> = BTreeMap::new();
    for r in rows.iter().filter_map(|r| r.reason.map(|x| (x, r))) {
        let e = by_reason.entry(r.0.as_str()).or_default();
        e.0 += 1;
        e.1 += r.1.lines;
        e.2 += r.1.ms;
    }
    let mut ordered: Vec<_> = by_reason.into_iter().collect();
    ordered.sort_by(|a, b| {
        b.1.2
            .partial_cmp(&a.1.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (name, (f, l, m)) in ordered {
        println!(
            "  {name:<24} {f:>7} {:>6.1}% {l:>9} {:>6.1}% {m:>9.0} {:>6.1}%",
            ratio(f, files_n),
            ratio(l, lines_n),
            pct(m, ms_n),
        );
    }

    // Mean cost per file on each side — the per-keystroke number.
    let mean = |v: &[&Row]| -> f64 {
        if v.is_empty() {
            0.0
        } else {
            v.iter().map(|r| r.ms).sum::<f64>() / f64::from(u32::try_from(v.len()).unwrap_or(1))
        }
    };
    let slow: Vec<&Row> = rows.iter().filter(|r| r.reason.is_some()).collect();
    println!(
        "\n  mean per file: fast {:.1} ms  |  fell back {:.1} ms",
        mean(&fast),
        mean(&slow),
    );
}
