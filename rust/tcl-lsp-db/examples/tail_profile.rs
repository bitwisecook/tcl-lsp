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
    AnalyserConfig, LexerCfgKey, SourceFile, TclDatabase, TclDb, compilation_unit,
    compiler_check_diagnostics, file_analysis_incremental,
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
    time("analyse_per_item (no memo)", 3, || {
        Analyser::new().analyse_per_item(&src, dialect)
    });
    let t_structure = time("structure_only().analyse (no diagnostics)", 3, || {
        Analyser::new().structure_only().analyse(&src, dialect)
    });
    // Ask the analyser, don't infer it from the clock.  This used to compare
    // `t_per_item` against `t_analyse` with a 15% band and call them equal,
    // which made the verdict a function of whatever else happened to be slow:
    // removing the per-CFG-build `CommandRegistry::build_default()` (issue
    // #1035) moved the two timings apart and flipped `09_long_code.tcl` from
    // "fallback" to "fast path" — while the analyser's behaviour was unchanged,
    // and *falling back* is the true answer for that file either way.  Two
    // paths costing about the same never implied they were the same path.
    //
    // `took_fast_path` is set by `analyse_per_item_with` itself: `true` only
    // when the run completed on the incremental path, `false` on every fallback
    // (incomplete script, stub overlay, Tk, ghost recovery, a partial command,
    // an E-code).  Costs stay measured; the structural claim gets asked.
    let mut probe = Analyser::new();
    let _ = probe.analyse_per_item(&src, dialect);
    println!(
        "  -> fallback to full walk? {}  (diagnostic-emit cost ~= {:.0} ms)",
        if probe.took_fast_path {
            "NO (fast path)"
        } else {
            "YES (analyse_per_item fell back)"
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
        None,
        0,
        Vec::new(),
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
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let build = || {
        CompilationUnit::build_for_with_config(&src, registry, false, cfg_lexer)
            .with_interprocedural(registry, Some(profile))
    };
    time("CompilationUnit::build_for + interproc", 3, build);
    let cu = build();
    time("run_all_checks", 3, || {
        tcl_compiler::compiler_checks::run_all_checks(&cu, registry, Some(profile))
    });
    time("optimise_unit", 3, || {
        tcl_compiler::optimiser::optimise_unit(&cu, registry, Some(profile))
    });
    println!("  (functions in unit: {})", cu.functions().count());

    // Decompose the run_all_checks tail across phases:
    // per-function checks (GVN, shimmer/thunking) reuse cleanly under a per-proc
    // memo; the whole-unit interproc taint solve does not.
    println!("\n== run_all_checks phase decomposition (whole-file, no memo) ==");
    let t_gvn = time("GVN redundancy/partial/loop (per-fn)", 3, || {
        find_redundancies_for_cu(&cu, registry, Some(profile)).len()
            + find_partial_redundancies_for_cu(&cu, registry, Some(profile)).len()
            + find_loop_invariants_for_cu(&cu, registry, Some(profile)).len()
    });
    let t_shimmer = time("shimmer + thunking (per-fn)", 3, || {
        find_shimmer_warnings_for_cu(&cu, registry).len()
            + find_thunking_warnings_for_cu(&cu, registry).len()
    });
    let t_taint = time("solve_interprocedural_taints (whole-unit)", 3, || {
        solve_interprocedural_taints(&cu, registry, Some(profile))
    });
    println!(
        "  per-function (GVN+shimmer) ~ {:.0} ms  |  whole-unit taint solve ~ {:.0} ms",
        t_gvn + t_shimmer,
        t_taint,
    );

    // The warm cost of a SECOND whole-file unit build on the
    // same db — exactly what a checks-path `proc_taint_solve` query would re-run to
    // recover the offset-0 `FnLatticeKey`s locally (they cannot be threaded out of
    // the shared `compilation_unit`: a salsa tracked return must be `'static`). The
    // per-proc lattice demands route through the already-hot `function_lattice` /
    // `taint_cascade` memos, so this measures only the whole-file structural
    // reassembly (lex + segment + IR skeleton + interproc summary) the second build
    // cannot avoid. This delta is the duplicate-build cost the checks-path memo
    // must pay against its ~120 ms summary-floor saving.
    //
    // Measured through the tracked `compilation_unit` query, one fresh
    // `SourceFile` per iteration: the same text under a distinct query key, so
    // each iteration really rebuilds instead of hitting the memo. It does not
    // call the crate's unit builder directly — that is crate-private precisely
    // because its per-body interning is only garbage-collectable inside a
    // tracked query (see tcl-lsp-db's "The interned garbage collector is
    // load-bearing").
    println!("\n== 2b gate: warm duplicate-build cost (function_lattice cache hot) ==");
    {
        const DUP_ITERS: u32 = 5;
        // Warm the shared build (populates function_lattice / taint_cascade) on the
        // *edited* text, mirroring the per-edit state proc_taint_solve runs in.
        file.set_text(&mut db).to(edited.clone());
        let _ = compiler_check_diagnostics(&db, file, cfg);
        let cfg_key = LexerCfgKey::new(&db, dialect.to_owned());
        let mut dup_files = (0..DUP_ITERS)
            .map(|_| SourceFile::new(&db, edited.clone(), dialect.to_owned(), None))
            .collect::<Vec<_>>()
            .into_iter();
        let dup = time("compilation_unit (2nd build, warm)", DUP_ITERS, || {
            compilation_unit(
                &db,
                dup_files.next().expect("one source file per iteration"),
                cfg_key,
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

/// Count salsa `WillExecute` events per query for one edit, against a db already
/// warm on `src`.  Returns `(cold, warm)` event logs.
fn breadth_for_edit(src: &str, dialect: &str, edited: &str) -> (Vec<String>, Vec<String>) {
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&log);
    let mut db = TclDatabase::with_event_logger(move |k| sink.lock().unwrap().push(k));
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
    );
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let cold: Vec<String> = std::mem::take(&mut *log.lock().unwrap());
    file.set_text(&mut db).to(edited.to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let warm: Vec<String> = std::mem::take(&mut *log.lock().unwrap());
    (cold, warm)
}

/// Count salsa `WillExecute` events per query across several *kinds* of
/// single-character edit, on a warm db.
///
/// The interesting comparison is between an edit that changes a body's content
/// and one that changes only where every body *sits*.  The per-proc memo keys
/// (`ProcBodyKey` / `FnLatticeKey` / `ItemBodyKey`) are offset-normalised — they
/// hold the body's text and context but not its position — so a pure shift
/// should re-intern to the same keys and hit every memo, while a content edit
/// should rebuild exactly one procedure.  Anything that re-executes on the shift
/// row is work the offset-invariance was supposed to have removed, i.e. rework
/// caused by a position that failed to be inherited rather than stored.
fn rerun_breadth(src: &str, dialect: &str, fallback_pos: usize, n_functions: usize) {
    println!("\n== salsa re-execution breadth (one edit, warm db) ==");
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

    let mut body_edit = src.to_owned();
    body_edit.insert(body_pos, 'Z');
    // Pure shift: a fresh comment line at the very top. Not one byte of any
    // procedure's text changes — every body just moves down by 8 bytes.
    let shift_edit = format!("# shift\n{src}");

    let scenarios = [
        ("body token edit", body_edit),
        ("prepend line (pure shift)", shift_edit),
    ];
    let mut cold_once: Option<Vec<String>> = None;
    let mut warms: Vec<(&str, Vec<String>)> = Vec::new();
    for (label, edited) in &scenarios {
        let (cold, warm) = breadth_for_edit(src, dialect, edited);
        cold_once.get_or_insert(cold);
        warms.push((label, warm));
    }
    let cold = cold_once.unwrap_or_default();

    println!(
        "  {:<34} {:>6}  {:>16}  {:>16}",
        "query", "cold", scenarios[0].0, scenarios[1].0
    );
    let row = |q: &str| {
        let c = cold.iter().filter(|s| s.contains(q)).count();
        let a = warms[0].1.iter().filter(|s| s.contains(q)).count();
        let b = warms[1].1.iter().filter(|s| s.contains(q)).count();
        println!("  {q:<34} {c:>6}  {a:>16}  {b:>16}");
    };
    println!("  per-proc memoised (should rebuild only the edited procedure):");
    row("item_body_analysis");
    row("function_lattice");
    row("taint_cascade");
    println!("  whole-file (re-run in full regardless of which procedure changed):");
    row("compilation_unit");
    row("compiler_check_diagnostics");
    let cc = warms[0]
        .1
        .iter()
        .filter(|s| s.contains("compiler_check_diagnostics"))
        .count();
    println!(
        "  -> run_all_checks runs inside each of the {cc} compiler_check_diagnostics \
         re-execution(s), over all {n_functions} functions, every edit.",
    );
}
