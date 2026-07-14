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

//! Per-edit tail profiler for the incremental diagnostic path on practcl.tcl.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use salsa::Setter as _;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::shimmer::{find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu};
use tcl_compiler::taint_interproc::solve_interprocedural_taints;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    file_analysis_incremental,
};

fn ms(d: std::time::Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn time<T>(label: &str, iters: u32, mut f: impl FnMut() -> T) -> f64 {
    let s = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(f());
    }
    let per = ms(s.elapsed()) / f64::from(iters);
    println!("  {label:<46} {per:8.1} ms");
    per
}

// A linear profiling script: setup, then one timed block per measured path.
#[allow(clippy::too_many_lines)]
fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
    let rel = std::env::var("FILE")
        .unwrap_or_else(|_| "tcllib-2.0/modules/practcl/practcl.tcl".to_owned());
    let path = root.join(&rel);
    let src = std::fs::read_to_string(&path).expect("read file");
    let dialect = "tcl8.6";
    println!("== {rel} ({} lines) ==", src.lines().count());

    // Is the per-item fast path taken, or does it fall back to the full walk?
    let t_analyse = time("analyse (full whole-file walk)", 3, || {
        Analyser::new().analyse(&src, dialect)
    });
    let t_per_item = time("analyse_per_item (no memo)", 3, || {
        Analyser::new().analyse_per_item(&src, dialect)
    });
    let t_structure = time("structure_only().analyse (no diagnostics)", 3, || {
        Analyser::new().structure_only().analyse(&src, dialect)
    });
    println!(
        "  -> fallback to full walk? {}  (diagnostic-emit cost ~= {:.0} ms)",
        if (t_per_item - t_analyse).abs() < t_analyse * 0.15 {
            "YES (per_item ~= analyse)"
        } else {
            "NO (fast path)"
        },
        t_analyse - t_structure,
    );

    println!("\n== full per-edit path (salsa, memoised) ==");
    let mut db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
    );
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);

    let edit_pos = src.find("\n    ").map_or(src.len() / 2, |p| p + 5);
    let mut edited = src.clone();
    edited.insert(edit_pos, ' ');
    let mut t = false;
    time("file_analysis_incremental (per edit)", 5, || {
        t = !t;
        file.set_text(&mut db)
            .to(if t { edited.clone() } else { src.clone() });
        file_analysis_incremental(&db, file, cfg)
    });
    let mut t2 = false;
    time("compiler_check_diagnostics (per edit)", 5, || {
        t2 = !t2;
        file.set_text(&mut db)
            .to(if t2 { edited.clone() } else { src.clone() });
        compiler_check_diagnostics(&db, file, cfg)
    });
    // Production shape: the server demands BOTH queries after one didChange, so
    // the shared `compilation_unit` build is paid once per edit (the second
    // consumer's demand is a same-revision cache hit when configs coincide).
    let mut t3 = false;
    time("BOTH queries per edit (production)", 5, || {
        t3 = !t3;
        file.set_text(&mut db)
            .to(if t3 { edited.clone() } else { src.clone() });
        let a = file_analysis_incremental(&db, file, cfg);
        let c = compiler_check_diagnostics(&db, file, cfg);
        (a, c)
    });

    println!("\n== compiler-check tail breakdown (whole-file, no memo) ==");
    let registry = db.registry(dialect);
    let cfg_lexer = tcl_lexer::LexerConfig::for_dialect(dialect);
    let build = || {
        CompilationUnit::build_for_with_config(&src, &registry, false, cfg_lexer)
            .with_interprocedural(&registry, Some(dialect))
    };
    time("CompilationUnit::build_for + interproc", 3, build);
    let cu = build();
    time("run_all_checks", 3, || {
        tcl_compiler::compiler_checks::run_all_checks(&cu, &registry, Some(dialect))
    });
    time("optimise_unit", 3, || {
        tcl_compiler::optimiser::optimise_unit(&cu, &registry, Some(dialect))
    });
    println!("  (functions in unit: {})", cu.functions().count());

    // Decompose the run_all_checks tail across phases:
    // per-function checks (GVN, shimmer/thunking) reuse cleanly under a per-proc
    // memo; the whole-unit interproc taint solve does not.
    println!("\n== run_all_checks phase decomposition (whole-file, no memo) ==");
    let t_gvn = time("GVN redundancy/partial/loop (per-fn)", 3, || {
        find_redundancies_for_cu(&cu, &registry, Some(dialect)).len()
            + find_partial_redundancies_for_cu(&cu, &registry, Some(dialect)).len()
            + find_loop_invariants_for_cu(&cu, &registry, Some(dialect)).len()
    });
    let t_shimmer = time("shimmer + thunking (per-fn)", 3, || {
        find_shimmer_warnings_for_cu(&cu, &registry).len()
            + find_thunking_warnings_for_cu(&cu).len()
    });
    let t_taint = time("solve_interprocedural_taints (whole-unit)", 3, || {
        solve_interprocedural_taints(&cu, &registry, Some(dialect))
    });
    println!(
        "  per-function (GVN+shimmer) ~ {:.0} ms  |  whole-unit taint solve ~ {:.0} ms",
        t_gvn + t_shimmer,
        t_taint,
    );

    // The warm cost of a SECOND `memoised_compilation_unit` build on the
    // same db — exactly what a checks-path `proc_taint_solve` query would re-run to
    // recover the offset-0 `FnLatticeKey`s locally (they cannot be threaded out of
    // the shared `compilation_unit`: a salsa tracked return must be `'static`). The
    // per-proc lattice demands route through the already-hot `function_lattice` /
    // `taint_cascade` memos, so this measures only the whole-file structural
    // reassembly (lex + segment + IR skeleton + interproc summary) the second build
    // cannot avoid. This delta is the duplicate-build cost the checks-path memo
    // must pay against its ~120 ms summary-floor saving.
    println!("\n== 2b gate: warm duplicate-build cost (function_lattice cache hot) ==");
    {
        let dialect_opt = Some(dialect);
        let cfg_d = tcl_lexer::LexerConfig::for_dialect(dialect);
        // Warm the shared build (populates function_lattice / taint_cascade) on the
        // *edited* text, mirroring the per-edit state proc_taint_solve runs in.
        file.set_text(&mut db).to(edited.clone());
        let _ = compiler_check_diagnostics(&db, file, cfg);
        let dup = time("memoised_compilation_unit (2nd build, warm)", 5, || {
            tcl_lsp_db::memoised_compilation_unit(
                &db,
                &edited,
                &registry,
                false,
                cfg_d,
                dialect_opt,
            )
        });
        println!(
            "  -> duplicate-build delta ~{dup:.0} ms (cold whole-file build was ~59 ms above); \
             2b is worth its machinery only if (summary-floor ~120 ms − {dup:.0} ms) is a clear win",
        );
    }

    // Direct measurement of per-edit re-execution breadth (counts salsa
    // `WillExecute` events per query, cold build vs. one body edit).
    rerun_breadth(&src, dialect, edit_pos, cu.functions().count());
}

/// Count salsa `WillExecute` events per query across one single-procedure body
/// edit.  Proves the per-proc memo rebuilds ONE procedure (`function_lattice` /
/// `item_body_analysis` / `taint_cascade`) while `compiler_check_diagnostics`
/// re-executes wholesale — so `run_all_checks`, which runs inside it and is not
/// itself a salsa query, re-checks every function every edit.
fn rerun_breadth(src: &str, dialect: &str, fallback_pos: usize, n_functions: usize) {
    println!("\n== salsa re-execution breadth (one body edit, warm db) ==");
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let mut db = TclDatabase::with_event_logger(move |k| sink.lock().unwrap().push(k));
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
    );
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let cold: Vec<String> = std::mem::take(&mut *log.lock().unwrap());
    // A single-character body edit that changes a *token* (a letter inserted after
    // the first alphanumeric character of the largest proc body) — not whitespace,
    // which the offset-invariant key normalises away.  Exactly one procedure's body
    // changes; every other procedure shifts but is a cache hit.
    let body_pos = Analyser::new()
        .analyse(src, dialect)
        .all_procs
        .values()
        .map(|p| p.body_span)
        .filter(|s| s.end() > s.start() + 4)
        .max_by_key(|s| s.end() - s.start())
        .and_then(|s| {
            let (lo, hi) = (s.start() as usize, s.end() as usize);
            src[lo..hi]
                .find(|c: char| c.is_ascii_alphanumeric())
                .map(|off| lo + off + 1)
        })
        .unwrap_or(fallback_pos);
    let mut edited_tok = src.to_owned();
    edited_tok.insert(body_pos, 'Z');
    file.set_text(&mut db).to(edited_tok);
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let warm: Vec<String> = std::mem::take(&mut *log.lock().unwrap());
    let row = |q: &str| {
        let c = cold.iter().filter(|s| s.contains(q)).count();
        let w = warm.iter().filter(|s| s.contains(q)).count();
        println!("  {q:<34} cold {c:>5}   1-edit {w:>5}");
    };
    println!("  per-proc memoised (rebuild only the edited procedure):");
    row("item_body_analysis");
    row("function_lattice");
    row("taint_cascade");
    println!("  whole-file (re-run in full regardless of which procedure changed):");
    row("compilation_unit");
    row("compiler_check_diagnostics");
    let cc = warm
        .iter()
        .filter(|s| s.contains("compiler_check_diagnostics"))
        .count();
    println!(
        "  -> run_all_checks runs inside each of the {cc} compiler_check_diagnostics \
         re-execution(s), over all {n_functions} functions, every edit.",
    );
}
