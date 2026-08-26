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

//! `object_lattice_cost` — measurement gate **M1** for issue #994's C5a.
//!
//! The object-handle lattice ([`tcl_compiler::object_types`]) rides on a
//! [`CompilationUnit`] the analyser already builds, so the only question C5a
//! has to answer is whether the lattice — and the scope-keying C5b's consumers
//! need — is cheap *relative to the unit it rides on*.  This harness measures
//! exactly that, per file:
//!
//! * **CU build ms** — `build_for` + `with_interprocedural`, the pass the
//!   lattice rides on.
//! * **lattice ms** — [`object_handle_classes`], the union-only map that ships
//!   today.
//! * **+scope-keying ms** — [`object_handle_facts`] minus the above: the owner
//!   attribution, owner-span index, collection map, and exported sub-facts.
//! * **fixpoint rounds** and **bindings by VTA edge kind**.
//! * **fast-path delta** — the same lattice with and without the empty-seed
//!   fast path, so the win on a file with no object seeds is visible.
//!
//! Gates (from the C5a design): median lattice ≤ 5 % of CU build, p99 ≤ 15 %,
//! and no single file over 25 ms of lattice time.
//!
//! Usage:
//!
//! ```text
//! cargo run --release -p tcl-compiler --example object_lattice_cost -- \
//!     experiments/corpus/georgtree experiments/corpus/adversarial \
//!     experiments/corpus/vendor experiments/corpus/repo-fixtures
//! ```
//!
//! `REPEATS=n` (default 3) runs each timed phase `n` times and keeps the
//! minimum, which suppresses scheduler noise on the small files.

use std::path::{Path, PathBuf};
use std::time::Instant;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::object_types::{
    LatticeStats, object_handle_classes, object_handle_classes_full_walk, object_handle_facts,
};
use tcl_registry::CommandRegistry;

/// Lossless-for-realistic-counts `usize`→`f64`, as `mro_eval` does it.
fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// One measured file.
struct Row {
    path: PathBuf,
    lines: usize,
    cu_ms: f64,
    lattice_ms: f64,
    scope_ms: f64,
    fast_walk_ms: f64,
    stats: LatticeStats,
    handles: usize,
    scoped_handles: usize,
}

impl Row {
    /// Lattice time as a percentage of the CU build it rides on.
    fn ratio(&self) -> f64 {
        if self.cu_ms <= 0.0 {
            0.0
        } else {
            100.0 * self.lattice_ms / self.cu_ms
        }
    }
}

/// Recursively collect `.tcl` / `.test` files under `root`.
fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                continue;
            }
            collect_tcl_files(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "tcl" || ext == "test")
        {
            out.push(path);
        }
    }
}

/// Run `f` `repeats` times, returning the fastest elapsed milliseconds.
fn best_ms(repeats: usize, mut f: impl FnMut()) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..repeats.max(1) {
        let t0 = Instant::now();
        f();
        best = best.min(t0.elapsed().as_secs_f64() * 1000.0);
    }
    best
}

/// Measure one file; `None` on read error / oversize / panic.
fn measure(path: &Path, reg: &CommandRegistry, repeats: usize) -> Option<Row> {
    let src = std::fs::read_to_string(path).ok()?;
    if src.len() > 2_000_000 {
        return None;
    }
    let lines = src.lines().count();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let cu_ms = best_ms(repeats, || {
            let cu = CompilationUnit::build_for(&src, reg, false).with_interprocedural(
                reg,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                ),
            );
            std::hint::black_box(&cu);
        });
        let cu = CompilationUnit::build_for(&src, reg, false).with_interprocedural(
            reg,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        let lattice_ms = best_ms(repeats, || {
            std::hint::black_box(object_handle_classes(&cu, reg));
        });
        let full_ms = best_ms(repeats, || {
            std::hint::black_box(object_handle_facts(&cu, reg));
        });
        let fast_walk_ms = best_ms(repeats, || {
            std::hint::black_box(object_handle_classes_full_walk(&cu, reg));
        });
        let (facts, stats) = tcl_compiler::object_types::object_handle_facts_instrumented(&cu, reg);
        Row {
            path: path.to_path_buf(),
            lines,
            cu_ms,
            lattice_ms,
            scope_ms: (full_ms - lattice_ms).max(0.0),
            fast_walk_ms,
            stats,
            handles: facts.any_scope.len(),
            scoped_handles: facts.by_scope.len(),
        }
    }))
    .ok()
}

/// The `num`/`den` quantile of an already-sorted slice, by nearest rank.
/// Integer arithmetic throughout, so there is no float→index cast to justify.
fn quantile(sorted: &[f64], num: usize, den: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let last = sorted.len() - 1;
    sorted[((last * num) + den / 2) / den]
}

/// Print the per-file table (the slowest `n` rows plus every row over the
/// per-file ceiling).
fn print_per_file(rows: &[&Row], limit: usize) {
    println!("\n## Per-file (slowest {limit} by lattice ms)\n");
    println!(
        "| file | lines | CU build ms | lattice ms | lattice % of CU | +scope-keying ms | rounds | handles | scoped |"
    );
    println!("|---|--:|--:|--:|--:|--:|--:|--:|--:|");
    for r in rows.iter().take(limit) {
        println!(
            "| `{}` | {} | {:.3} | {:.3} | {:.2} % | {:.3} | {} | {} | {} |",
            r.path.display(),
            r.lines,
            r.cu_ms,
            r.lattice_ms,
            r.ratio(),
            r.scope_ms,
            r.stats.rounds,
            r.handles,
            r.scoped_handles,
        );
    }
}

/// Print the aggregate gate verdicts.
fn print_gates(rows: &[Row]) {
    let mut ratios: Vec<f64> = rows.iter().map(Row::ratio).collect();
    ratios.sort_by(f64::total_cmp);
    let median = quantile(&ratios, 1, 2);
    let p99 = quantile(&ratios, 99, 100);
    let worst_file = rows
        .iter()
        .max_by(|a, b| a.lattice_ms.total_cmp(&b.lattice_ms));
    let worst_ms = worst_file.map_or(0.0, |r| r.lattice_ms);

    let verdict = |ok: bool| if ok { "PASS" } else { "**FAIL**" };
    println!("\n## Gate verdicts\n");
    println!("| gate | threshold | measured | verdict |");
    println!("|---|--:|--:|---|");
    println!(
        "| median lattice / CU build | ≤ 5 % | {median:.2} % | {} |",
        verdict(median <= 5.0)
    );
    println!(
        "| p99 lattice / CU build | ≤ 15 % | {p99:.2} % | {} |",
        verdict(p99 <= 15.0)
    );
    println!(
        "| worst-file lattice time | ≤ 25 ms | {worst_ms:.3} ms | {} |",
        verdict(worst_ms <= 25.0)
    );
    if let Some(w) = worst_file {
        println!(
            "\nWorst file: `{}` — {:.3} ms lattice on a {:.3} ms CU build ({:.2} %), {} lines, {} rounds.",
            w.path.display(),
            w.lattice_ms,
            w.cu_ms,
            w.ratio(),
            w.lines,
            w.stats.rounds,
        );
    }
}

/// Print the empty-seed fast-path win and the edge-kind roll-up.
fn print_fast_path_and_edges(rows: &[Row]) {
    let hits: Vec<&Row> = rows.iter().filter(|r| r.stats.fast_path_hit).collect();
    let with: f64 = hits.iter().map(|r| r.lattice_ms).sum();
    let without: f64 = hits.iter().map(|r| r.fast_walk_ms).sum();
    println!("\n## Empty-seed fast path\n");
    println!(
        "| files | lattice ms (fast path) | lattice ms (pre-#994 walk) | saved | saved / file |"
    );
    println!("|--:|--:|--:|--:|--:|");
    let saved = without - with;
    println!(
        "| {} of {} | {with:.3} | {without:.3} | {saved:.3} ms ({:.1} %) | {:.4} ms |",
        hits.len(),
        rows.len(),
        if without > 0.0 {
            100.0 * saved / without
        } else {
            0.0
        },
        saved / as_f64(hits.len().max(1)),
    );
    if let Some(w) = hits
        .iter()
        .max_by(|a, b| (a.fast_walk_ms - a.lattice_ms).total_cmp(&(b.fast_walk_ms - b.lattice_ms)))
    {
        println!(
            "\nBiggest single-file win: `{}` ({} lines) — {:.3} ms → {:.3} ms.",
            w.path.display(),
            w.lines,
            w.fast_walk_ms,
            w.lattice_ms,
        );
    }

    let sum = |f: fn(&LatticeStats) -> usize| rows.iter().map(|r| f(&r.stats)).sum::<usize>();
    println!("\n## Bindings by VTA edge kind (corpus total)\n");
    println!("| edge | bindings |");
    println!("|---|--:|");
    println!("| seed (harvest) | {} |", sum(|s| s.seeds));
    println!("| aliasing `set A $B` | {} |", sum(|s| s.alias_bindings));
    println!(
        "| proc return `set A [f]` | {} |",
        sum(|s| s.return_bindings)
    );
    println!(
        "| proc parameter `f $obj` | {} |",
        sum(|s| s.param_bindings)
    );
    println!(
        "| constructor parameter `C new $obj` | {} |",
        sum(|s| s.ctor_param_bindings)
    );
    let mut rounds: Vec<usize> = rows.iter().map(|r| r.stats.rounds as usize).collect();
    rounds.sort_unstable();
    println!(
        "\nFixpoint rounds: max {}, median {}, {} files ran none (fast path).",
        rounds.last().copied().unwrap_or(0),
        rounds.get(rounds.len() / 2).copied().unwrap_or(0),
        rows.iter().filter(|r| r.stats.rounds == 0).count(),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let roots: Vec<PathBuf> = if args.is_empty() {
        ["experiments/corpus"].iter().map(PathBuf::from).collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let repeats: usize = std::env::var("REPEATS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let mut files = Vec::new();
    for r in &roots {
        collect_tcl_files(r, &mut files);
    }
    files.sort();
    if files.is_empty() {
        eprintln!("no Tcl files under {roots:?}");
        std::process::exit(1);
    }

    println!("# object_lattice_cost — issue #994 C5a measurement gate M1");
    println!(
        "\ncorpus roots: {:?}\nfiles: {}, repeats: {repeats}",
        roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
        files.len(),
    );

    let reg = CommandRegistry::build_default();
    let rows: Vec<Row> = files
        .iter()
        .filter_map(|p| measure(p, &reg, repeats))
        .collect();

    let total_lines: usize = rows.iter().map(|r| r.lines).sum();
    let cu_total: f64 = rows.iter().map(|r| r.cu_ms).sum();
    let lattice_total: f64 = rows.iter().map(|r| r.lattice_ms).sum();
    let scope_total: f64 = rows.iter().map(|r| r.scope_ms).sum();
    let measured = rows.len();
    let lattice_pct = 100.0 * lattice_total / cu_total.max(1e-9);
    let scope_pct = 100.0 * scope_total / cu_total.max(1e-9);
    println!(
        "measured {measured} files, {total_lines} lines\ntotals: CU build {cu_total:.1} ms, lattice {lattice_total:.1} ms ({lattice_pct:.2} % of CU), +scope-keying {scope_total:.1} ms ({scope_pct:.2} % of CU)"
    );

    print_gates(&rows);
    let mut by_cost: Vec<&Row> = rows.iter().collect();
    by_cost.sort_by(|a, b| b.lattice_ms.total_cmp(&a.lattice_ms));
    print_per_file(&by_cost, 15);
    print_fast_path_and_edges(&rows);
}
