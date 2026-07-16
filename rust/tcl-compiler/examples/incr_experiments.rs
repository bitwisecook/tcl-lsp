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

//! Phase-0 experiments for per-item incremental analysis.
//! E3: walk vs cross-item-aggregate (tail) cost split.
//! E1: item-locality (perf heuristic) â a body edit should leave items defined
//!     *before* it byte-identical. Robust perturbation: insert a benign comment
//!     at the body start (always valid; only shifts offsets after the insert),
//!     then compare every item/diagnostic that ends before the insert point.

// Demonstration/experiment harness: percentage maths casts small counts to
// f64 and `main` drives several experiments inline — neither warrants the
// pedantic precision/length lints here.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::path::{Path, PathBuf};
use std::time::Instant;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::{AnalysisResult, Diagnostic};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::LexerConfig;

fn gather_tcl(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            gather_tcl(&p, out, cap);
        } else if p.extension().is_some_and(|x| x == "tcl") {
            out.push(p);
            if out.len() >= cap {
                return;
            }
        }
    }
}

fn analyse(src: &str, d: &str) -> AnalysisResult {
    Analyser::new().analyse(src, d)
}

fn diags_before(r: &AnalysisResult, ins: u32) -> Vec<&Diagnostic> {
    let mut v: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.span.end() <= ins)
        .collect();
    v.sort_by_key(|d| (d.span.start(), d.span.end(), d.code, d.message.clone()));
    v
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let dialect = "tcl8.6";

    // ---- E3: walk vs tail cost split on practcl ----
    let practcl = root.join("tcllib-2.0/modules/practcl/practcl.tcl");
    if let Ok(src) = std::fs::read_to_string(&practcl) {
        let cmds =
            segment_commands_with_offset_and_config(&src, 0, LexerConfig::for_dialect(dialect));
        let _ = Analyser::new().analyse_commands(&src, &cmds, dialect, true); // warm
        let t = |fin: bool| {
            let s = Instant::now();
            for _ in 0..3 {
                let _ = Analyser::new().analyse_commands(&src, &cmds, dialect, fin);
            }
            s.elapsed().as_secs_f64() * 1000.0 / 3.0
        };
        let (walk, full) = (t(false), t(true));
        println!("== E3 (practcl, {} lines) ==", src.lines().count());
        println!("  walk only:        {walk:7.1} ms");
        println!("  walk + tail:      {full:7.1} ms");
        println!(
            "  cross-item tail:  {:7.1} ms  ({:.0}% of full)  => {}",
            full - walk,
            (full - walk) / full * 100.0,
            if full - walk < 200.0 {
                "GO"
            } else {
                "investigate"
            }
        );
    }

    // ---- E1: item-locality (preceding items invariant under a body edit) ----
    let mut files = Vec::new();
    for v in ["tcl8.6.16", "tcllib-2.0/modules"] {
        gather_tcl(&root.join(v), &mut files, 600);
    }
    let (mut tested, mut clean, mut leaky) = (0usize, 0usize, 0usize);
    let mut examples: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.len() > 250_000 {
            continue;
        }
        let r0 = analyse(&src, dialect);
        // proc with the most preceding procs: the last-defined proc with a body
        let Some(p) = r0
            .all_procs
            .values()
            .filter(|p| p.body_span.end() > p.body_span.start())
            .max_by_key(|p| p.body_span.start())
        else {
            continue;
        };
        let ins = p.body_span.start();
        // need some items before the insert point to test against
        let preceding: Vec<_> = r0
            .all_procs
            .iter()
            .filter(|(_, d)| d.body_span.end() <= ins)
            .collect();
        if preceding.is_empty() {
            continue;
        }
        let probe = "# __probe__\n";
        let mut src2 = String::with_capacity(src.len() + probe.len());
        src2.push_str(&src[..ins as usize]);
        src2.push_str(probe);
        src2.push_str(&src[ins as usize..]);
        let r1 = analyse(&src2, dialect);
        tested += 1;
        let mut leak: Option<String> = None;
        for (q, def0) in &preceding {
            match r1.all_procs.get(*q) {
                Some(def1) if def1 == *def0 => {}
                Some(_) => {
                    leak = Some(format!("proc {q} CHANGED"));
                    break;
                }
                None => {
                    leak = Some(format!("proc {q} REMOVED"));
                    break;
                }
            }
        }
        if leak.is_none() && diags_before(&r0, ins) != diags_before(&r1, ins) {
            let c0: std::collections::BTreeSet<_> = diags_before(&r0, ins)
                .iter()
                .map(|d| (d.code, d.span.start()))
                .collect();
            let c1: std::collections::BTreeSet<_> = diags_before(&r1, ins)
                .iter()
                .map(|d| (d.code, d.span.start()))
                .collect();
            leak = Some(format!(
                "diag +{:?} -{:?}",
                c1.difference(&c0).take(2).collect::<Vec<_>>(),
                c0.difference(&c1).take(2).collect::<Vec<_>>()
            ));
        }
        if let Some(l) = leak {
            leaky += 1;
            if examples.len() < 8 {
                examples.push(format!(
                    "  LEAK {}: {}",
                    path.file_name().unwrap().to_string_lossy(),
                    l
                ));
            }
        } else {
            clean += 1;
        }
    }
    println!("\n== E1 item-locality (preceding items invariant under body edit) ==");
    println!(
        "  tested {tested} files: {clean} clean, {leaky} leaky ({:.1}% clean)",
        100.0 * clean as f64 / tested.max(1) as f64
    );
    for e in &examples {
        println!("{e}");
    }

    e2_offset_invariance(&files, dialect);
    e6_e7_lattice_costs(&root, dialect);
}

/// E2: the whole analyser is offset-shift-invariant â prepend K blank lines and
/// every fact must reappear shifted by exactly K bytes / K lines. Validates that
/// per-item facts can be produced at offset 0 and rebased.
fn e2_offset_invariance(files: &[PathBuf], dialect: &str) {
    const K: u32 = 10; // blank lines; 1 byte each
    let (mut tested, mut ok, mut bad) = (0usize, 0usize, 0usize);
    let mut examples: Vec<String> = Vec::new();
    for path in files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.len() > 250_000 || src.is_empty() {
            continue;
        }
        let r0 = analyse(&src, dialect);
        let src2 = format!("{}{}", "\n".repeat(K as usize), src);
        let r1 = analyse(&src2, dialect);
        tested += 1;
        // proc spans, un-shifted, must match
        let procs0: std::collections::BTreeSet<_> = r0
            .all_procs
            .values()
            .map(|p| {
                (
                    p.qualified_name.clone(),
                    p.name_span.start(),
                    p.body_span.start(),
                    p.body_span.end(),
                )
            })
            .collect();
        let procs1: std::collections::BTreeSet<_> = r1
            .all_procs
            .values()
            .map(|p| {
                (
                    p.qualified_name.clone(),
                    p.name_span.start().saturating_sub(K),
                    p.body_span.start().saturating_sub(K),
                    p.body_span.end().saturating_sub(K),
                )
            })
            .collect();
        // diagnostics (code, start), un-shifted
        let d0: std::collections::BTreeSet<_> = r0
            .diagnostics
            .iter()
            .map(|d| (d.code, d.span.start()))
            .collect();
        let d1: std::collections::BTreeSet<_> = r1
            .diagnostics
            .iter()
            .map(|d| (d.code, d.span.start().saturating_sub(K)))
            .collect();
        if procs0 == procs1 && d0 == d1 {
            ok += 1;
        } else {
            bad += 1;
            if examples.len() < 6 {
                let dd: Vec<_> = d1.symmetric_difference(&d0).take(3).collect();
                examples.push(format!(
                    "  SHIFT-DIFF {}: procs_eq={} diag_delta={:?}",
                    path.file_name().unwrap().to_string_lossy(),
                    procs0 == procs1,
                    dd
                ));
            }
        }
    }
    println!("\n== E2 offset-invariance (prepend {K} blank lines, un-shift, compare) ==");
    println!(
        "  tested {tested}: {ok} invariant, {bad} differ ({:.1}% invariant)",
        100.0 * ok as f64 / tested.max(1) as f64
    );
    for e in &examples {
        println!("{e}");
    }
}

/// E6/E7: `CompilationUnit` + interproc + optimiser costs (the lattice layer), and
/// the redundant-build saving a shared `CompilationUnit` would recover.
fn e6_e7_lattice_costs(root: &Path, dialect: &str) {
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;
    use tcl_compiler::optimiser::optimise_with_dialect;
    let practcl = root.join("tcllib-2.0/modules/practcl/practcl.tcl");
    let Ok(src) = std::fs::read_to_string(&practcl) else {
        return;
    };
    let mut reg = tcl_registry::CommandRegistry::build_default();
    if let Some(d) = tcl_dialect::DialectSet::parse(dialect) {
        reg.load_dialect(d);
    }
    let n = 3;
    let time = |f: &dyn Fn()| {
        f();
        let s = Instant::now();
        for _ in 0..n {
            f();
        }
        s.elapsed().as_secs_f64() * 1000.0 / f64::from(n)
    };
    let cfg = LexerConfig::for_dialect(dialect);
    let t_cu = time(&|| {
        let _ = CompilationUnit::build_for_with_config(&src, &reg, false, cfg);
    });
    let t_cu_ip = time(&|| {
        let _ = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
            .with_interprocedural(&reg, Some(dialect));
    });
    let t_checks = time(&|| {
        let cu = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
            .with_interprocedural(&reg, Some(dialect));
        let _ = run_all_checks(&cu, &reg, Some(dialect));
    });
    let t_opt = time(&|| {
        let _ = optimise_with_dialect(&src, &reg, Some(dialect));
    });
    let nprocs = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
        .procedures
        .len();
    println!("\n== E6 lattice costs (practcl, {nprocs} procs) ==");
    println!("  CompilationUnit build:        {t_cu:7.1} ms");
    println!(
        "  + with_interprocedural:       {t_cu_ip:7.1} ms  (interproc {:.1} ms)",
        t_cu_ip - t_cu
    );
    println!("  + run_all_checks:             {t_checks:7.1} ms");
    println!("  optimise_with_dialect:        {t_opt:7.1} ms  (rebuilds its own CU+interproc)");
    println!(
        "== E7 shared-CompilationUnit saving ~= redundant CU+interproc build = {:.1} ms => {}",
        t_cu_ip,
        if t_cu_ip > 30.0 { "GO" } else { "marginal" }
    );
}
