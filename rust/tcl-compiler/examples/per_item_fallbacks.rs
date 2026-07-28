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
    path: PathBuf,
}

#[allow(clippy::too_many_lines)] // a linear reporting script; splitting it hurts readability
fn main() {
    // `ROOT` points the sweep at a different corpus (e.g. a directory of
    // size-truncated copies, to check how cost scales with document length).
    let root = std::env::var("ROOT").map_or_else(
        |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp"),
        PathBuf::from,
    );
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
        // `COMPARE=1` also times the whole-file walk, so a document where the
        // incremental path is *slower* than the thing it replaces stands out.
        if std::env::var("COMPARE").is_ok() {
            let t0 = Instant::now();
            let _ = Analyser::new().analyse(&src, dialect);
            let full = t0.elapsed().as_secs_f64() * 1000.0;
            let t1 = Instant::now();
            let mut probe = Analyser::new();
            let _ = probe.analyse_per_item(&src, dialect);
            let incr = t1.elapsed().as_secs_f64() * 1000.0;
            println!(
                "  {:>6} lines  analyse {full:>9.0} ms  per_item {incr:>9.0} ms  x{:>6.1}  {:<20} {}",
                src.lines().count(),
                if full > 0.0 { incr / full } else { 0.0 },
                probe
                    .per_item_fallback
                    .map_or("FAST PATH", PerItemFallback::as_str),
                path.file_name().unwrap_or_default().to_string_lossy(),
            );
        }
        let mut a = Analyser::new();
        let start = Instant::now();
        let _ = a.analyse_per_item(&src, dialect);
        rows.push(Row {
            reason: a.per_item_fallback,
            lines: src.lines().count(),
            ms: start.elapsed().as_secs_f64() * 1000.0,
            path: path.clone(),
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

    // The tail is what a user actually feels: one pathological document costs
    // more per keystroke than a thousand ordinary ones.
    let mut slowest: Vec<&Row> = rows.iter().collect();
    slowest.sort_by(|a, b| b.ms.partial_cmp(&a.ms).unwrap_or(std::cmp::Ordering::Equal));
    println!("\n  slowest documents (per-keystroke cost):");
    for r in slowest.iter().take(8) {
        println!(
            "    {:>9.0} ms  {:>6} lines  {:<22} {}",
            r.ms,
            r.lines,
            r.reason.map_or("FAST PATH", PerItemFallback::as_str),
            r.path.file_name().unwrap_or_default().to_string_lossy(),
        );
    }

    if std::env::var("TK_AUDIT").is_ok() {
        tk_audit(&files, dialect);
    }
}

/// Audit the `tk-active` gate — the single most expensive fallback reason.
///
/// `tk_possibly_active` is three *independent* substring searches (`package`,
/// `require`, `Tk`, anywhere, in any order, comments included), and it does
/// double duty: it gates the per-item fallback *and* whether Tk diagnostics
/// accumulate at all.  Narrowing it is only safe if the files it currently
/// catches spuriously emit no Tk diagnostics — otherwise tightening the gate
/// silences real warnings.  This measures exactly that.
fn tk_audit(files: &[PathBuf], dialect: &str) {
    let (mut tripped, mut real_require, mut spurious_with_tk_diags) = (0usize, 0usize, 0usize);
    let mut genuine_tk_diags = 0usize;
    let mut spurious_examples: Vec<String> = Vec::new();
    for path in files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Mirrors `tk_checks::tk_possibly_active` (which is crate-private).
        let heuristic = src.contains("package") && src.contains("require") && src.contains("Tk");
        if !heuristic {
            continue;
        }
        tripped += 1;
        // What the gate is trying to detect: an actual `package require Tk`.
        let genuine = src.split("package").skip(1).any(|rest| {
            let rest = rest.trim_start();
            rest.strip_prefix("require").is_some_and(|r| {
                let r = r.trim_start();
                let r = r.strip_prefix("-exact").map_or(r, |x| x.trim_start());
                r.starts_with("Tk")
            })
        });
        let tk_diags = Analyser::new()
            .analyse(&src, dialect)
            .diagnostics
            .iter()
            .filter(|d| d.code.as_str().starts_with("TK"))
            .count();
        if genuine {
            real_require += 1;
            genuine_tk_diags += tk_diags;
            continue;
        }
        if tk_diags > 0 {
            spurious_with_tk_diags += 1;
            if spurious_examples.len() < 5 {
                spurious_examples.push(format!("{} ({tk_diags} TK diags)", path.display()));
            }
        }
    }
    println!("\n== tk-active gate audit ==");
    println!("  files tripping the 3-substring heuristic:      {tripped}");
    println!(
        "  ... with a genuine `package require Tk`:       {real_require} \
         (emitting {genuine_tk_diags} TK diagnostics in total)"
    );
    println!(
        "  ... spurious (no require) BUT emitting TK diags: {spurious_with_tk_diags}  \
         <- tightening the gate would change these"
    );
    for e in &spurious_examples {
        println!("      {e}");
    }
}
