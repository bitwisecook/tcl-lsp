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

//! Experiments to decide whether a per-procedure `optimise_unit` memo (backlog
//! #2) is worth building.  Run: `cargo run --release -p tcl-compiler --example
//! optimise_memo_experiments` (reads the `tmp/` corpus).
//!
//! Three measurements per file:
//! - **E1 savings ceiling** — `optimise_unit` time (the most a per-proc memo
//!   could remove on a warm edit, minus the always-run arbitration).
//! - **E2 memo hit rate** — insert a benign char inside each procedure body,
//!   rebuild the interprocedural summary, and check whether it stays
//!   byte-identical (a *whole-`interproc`-key* memo hits) and, when it changes,
//!   how many procedure summaries changed (a *reachable-key* memo's locality).
//! - **E3 key-build cost** — time to serialise `interproc` into a hashable key.

// Demonstration/experiment harness: percentage and ratio maths casts small
// counts to f64, and `main` drives several experiments inline — neither the
// pedantic precision-loss nor the length lint is worth acting on here.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::optimiser::optimise_unit;
use tcl_dialect::DialectProfile;
use tcl_registry::CommandRegistry;

fn gather(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            gather(&p, out, cap);
        } else if p.extension().is_some_and(|x| x == "tcl") {
            out.push(p);
            if out.len() >= cap {
                return;
            }
        }
    }
}

fn registry(dialect: &str) -> CommandRegistry {
    let mut r = CommandRegistry::build_default();
    if let Some(d) = tcl_registry::prelude::DialectSet::parse(dialect) {
        r.load_dialect(d);
    }
    r
}

/// A deterministic, hashable serialisation of the interprocedural summary — what
/// a whole-`interproc` memo key would have to intern per build.  Returns the
/// encoded length (bytes) as a coarse proxy for key size.
fn encode_interproc(cu: &CompilationUnit) -> usize {
    let Some(ia) = &cu.interproc else { return 0 };
    let mut names: Vec<&String> = ia.procedures.keys().collect();
    names.sort();
    let mut bytes = 0usize;
    for n in names {
        let s = &ia.procedures[n];
        bytes += n.len()
            + s.params.iter().map(String::len).sum::<usize>()
            + s.calls.iter().map(String::len).sum::<usize>()
            + 8;
    }
    bytes
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let dialect = "tcl8.6";
    let profile = DialectProfile::by_name(dialect);
    let reg = registry(dialect);
    let cfg = tcl_lexer::LexerConfig::for_dialect(dialect);

    // A spread of files: a few named large ones + a corpus sample.
    let named = [
        "tcllib-2.0/modules/pki/pki.tcl",
        "tcllib-2.0/modules/page/parse_lemon.tcl",
        "tcl8.6.16/library/init.tcl",
        "tcl8.6.16/library/safe.tcl",
    ];
    let mut files: Vec<PathBuf> = named.iter().map(|f| root.join(f)).collect();
    let mut corpus = Vec::new();
    for v in ["tcl8.6.16/library", "tcllib-2.0/modules"] {
        gather(&root.join(v), &mut corpus, 400);
    }

    // ---- Per-file detail for the named files ----
    println!("== E1 (savings ceiling) + E3 (key cost), named files ==");
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let cu = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
            .with_interprocedural(&reg, Some(profile));
        let n_procs = cu.procedures.len();
        let iters = 20u32;
        let t0 = Instant::now();
        for _ in 0..iters {
            std::hint::black_box(optimise_unit(&cu, &reg, Some(profile)));
        }
        let opt_ms = t0.elapsed().as_secs_f64() * 1000.0 / f64::from(iters);
        let t1 = Instant::now();
        for _ in 0..iters {
            std::hint::black_box(encode_interproc(&cu));
        }
        let key_ms = t1.elapsed().as_secs_f64() * 1000.0 / f64::from(iters);
        let rel = path.file_name().unwrap().to_string_lossy();
        println!(
            "  {rel:<22} procs={n_procs:<4} optimise_unit={opt_ms:6.2} ms  \
             key_build={key_ms:6.3} ms  per-proc≈{:.3} ms",
            if n_procs > 0 {
                opt_ms / n_procs as f64
            } else {
                0.0
            }
        );
    }

    // ---- E2 (memo hit rate) over the corpus sample ----
    println!("\n== E2 (interproc stability under per-proc body edits) ==");
    files.extend(corpus);
    let mut total_edits = 0usize;
    let mut interproc_unchanged = 0usize; // whole-key would hit
    let mut summaries_changed_total = 0usize; // reachable-key locality
    let mut proc_total = 0usize;
    let mut files_done = 0usize;
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.is_empty() || src.len() > 200_000 {
            continue;
        }
        let cu = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
            .with_interprocedural(&reg, Some(profile));
        let Some(base_ia) = &cu.interproc else {
            continue;
        };
        if cu.procedures.is_empty() {
            continue;
        }
        files_done += 1;
        proc_total += cu.procedures.len();
        // Edit inside each procedure body: insert a benign space just before the
        // procedure's closing brace (`proc.span.end() - 1`), which is well inside
        // the body — not in the `proc` keyword or signature.
        let mut ends: Vec<u32> = cu
            .ir_module
            .procedures
            .values()
            .map(|p| p.span.end())
            .collect();
        ends.sort_unstable();
        for end in ends {
            let pos = (end as usize).saturating_sub(1).min(src.len());
            if pos == 0 || !src.is_char_boundary(pos) {
                continue;
            }
            let mut edited = src.clone();
            edited.insert(pos, ' ');
            let cu2 = CompilationUnit::build_for_with_config(&edited, &reg, false, cfg)
                .with_interprocedural(&reg, Some(profile));
            let Some(ia2) = &cu2.interproc else { continue };
            total_edits += 1;
            if base_ia == ia2 {
                interproc_unchanged += 1;
            } else {
                // Count how many procedure summaries differ (reachable-key
                // would invalidate only callers of these).
                let mut changed = 0usize;
                for (name, s) in &base_ia.procedures {
                    match ia2.procedures.get(name) {
                        Some(s2) if s2 == s => {}
                        _ => changed += 1,
                    }
                }
                changed += ia2
                    .procedures
                    .keys()
                    .filter(|k| !base_ia.procedures.contains_key(*k))
                    .count();
                summaries_changed_total += changed;
            }
        }
    }
    let pct = |n: usize| 100.0 * n as f64 / total_edits.max(1) as f64;
    println!("  files={files_done}  procs={proc_total}  body-edits simulated={total_edits}");
    println!(
        "  interproc byte-identical after edit: {interproc_unchanged}/{total_edits} = {:.1}%  \
         (whole-key memo HIT rate)",
        pct(interproc_unchanged)
    );
    let changed_edits = total_edits - interproc_unchanged;
    println!(
        "  edits that changed interproc: {changed_edits}  (avg summaries changed when it did: {:.2})",
        if changed_edits > 0 {
            summaries_changed_total as f64 / changed_edits as f64
        } else {
            0.0
        }
    );
    println!(
        "\n  Interpretation: whole-key memo hits on {:.0}% of body edits; on the rest, a\n  \
         reachable-key memo would still reuse all procs except the ~{:.1} whose summary moved\n  \
         (and their callers).",
        pct(interproc_unchanged),
        if changed_edits > 0 {
            summaries_changed_total as f64 / changed_edits as f64
        } else {
            0.0
        }
    );
}
