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

/// Totals a set of rows contributes, for the three weightings the report
/// compares: documents, source lines, and measured milliseconds.
#[derive(Default, Clone, Copy)]
struct Totals {
    files: usize,
    lines: usize,
    ms: f64,
}

impl Totals {
    fn add(&mut self, row: &Row) {
        self.files += 1;
        self.lines += row.lines;
        self.ms += row.ms;
    }

    fn of<'a>(rows: impl IntoIterator<Item = &'a Row>) -> Self {
        let mut t = Self::default();
        for r in rows {
            t.add(r);
        }
        t
    }
}

/// Percentage of `whole`, via `u32` so the cast is provably lossless for
/// tallies of this size (and the pedantic cast lints stay quiet).
fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    f64::from(u32::try_from(part).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(whole).unwrap_or(u32::MAX))
        * 100.0
}

/// Percentage of a `f64` total (milliseconds), which has no integer form.
fn share_ms(part: f64, whole: f64) -> f64 {
    if whole > 0.0 {
        part / whole * 100.0
    } else {
        0.0
    }
}

/// Mean milliseconds across `rows`; `0.0` for an empty set.
fn mean_ms(rows: &[&Row]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|r| r.ms).sum::<f64>() / f64::from(u32::try_from(rows.len()).unwrap_or(1))
}

/// Analyse every corpus document once, recording which gate (if any) forced it
/// off the incremental path and what that cost.
fn collect_rows(files: &[PathBuf], dialect: &str, compare: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    for path in files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.trim().is_empty() {
            continue;
        }
        if compare {
            compare_paths(&src, dialect, path);
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
    rows
}

/// Time both paths for one document, so a case where the *incremental* path is
/// slower than the whole-file walk it replaces stands out per file.
fn compare_paths(src: &str, dialect: &str, path: &Path) {
    let t0 = Instant::now();
    let _ = Analyser::new().analyse(src, dialect);
    let full = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let mut probe = Analyser::new();
    let _ = probe.analyse_per_item(src, dialect);
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

/// One table row: absolute tallies plus each one's share of the corpus.
fn print_row(label: &str, part: Totals, whole: Totals) {
    println!(
        "  {label:<24} {:>7} {:>6.1}% {:>9} {:>6.1}% {:>9.0} {:>6.1}%",
        part.files,
        share(part.files, whole.files),
        part.lines,
        share(part.lines, whole.lines),
        part.ms,
        share_ms(part.ms, whole.ms),
    );
}

/// The fast-path/fallback split and the per-gate breakdown, each weighted three
/// ways — they rank the gates differently, and time is what a user feels.
fn print_breakdown(rows: &[Row], whole: Totals) {
    println!(
        "  {:<24} {:>7} {:>7}   {:>9} {:>7}   {:>9} {:>7}",
        "outcome", "files", "%", "lines", "%", "ms", "%"
    );
    print_row(
        "FAST PATH",
        Totals::of(rows.iter().filter(|r| r.reason.is_none())),
        whole,
    );
    print_row(
        "fell back (all)",
        Totals::of(rows.iter().filter(|r| r.reason.is_some())),
        whole,
    );
    println!();

    let mut by_reason: BTreeMap<&'static str, Totals> = BTreeMap::new();
    for row in rows {
        if let Some(reason) = row.reason {
            by_reason.entry(reason.as_str()).or_default().add(row);
        }
    }
    let mut ordered: Vec<_> = by_reason.into_iter().collect();
    ordered.sort_by(|a, b| {
        b.1.ms
            .partial_cmp(&a.1.ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (name, totals) in ordered {
        print_row(name, totals, whole);
    }
}

/// The slowest documents: the tail is what a user actually feels, since one
/// pathological document costs more per keystroke than a thousand ordinary ones.
fn print_slowest(rows: &[Row]) {
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
}

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

    let rows = collect_rows(&files, dialect, std::env::var("COMPARE").is_ok());
    let whole = Totals::of(&rows);
    println!(
        "corpus: {} files, {} lines, {:.0} ms total (dialect {dialect})\n",
        whole.files, whole.lines, whole.ms
    );

    print_breakdown(&rows, whole);

    let fast: Vec<&Row> = rows.iter().filter(|r| r.reason.is_none()).collect();
    let slow: Vec<&Row> = rows.iter().filter(|r| r.reason.is_some()).collect();
    println!(
        "\n  mean per file: fast {:.1} ms  |  fell back {:.1} ms",
        mean_ms(&fast),
        mean_ms(&slow),
    );

    print_slowest(&rows);

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
