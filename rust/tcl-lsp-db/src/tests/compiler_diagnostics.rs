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

//! The memoised compiler-check diagnostics against the uncached pass, under edits and for method and body units.

use super::*;

#[test]
fn compiler_check_o122_tailrec_memo_matches_uncached() {
    // An impure (side-effecting) tail-recursive proc fires O122
    // (recursion→loop). The per-proc optimise memo must reconstruct
    // `proc.body_source` as the *body text* — not the whole-command slice used
    // for span alignment — so the loop-conversion replacement wraps only the
    // body, not the entire `proc …` declaration.
    let dialect = "tcl8.6";
    let src = "proc countdown {n} {\n    puts $n\n    if {$n <= 0} { return }\n    countdown [expr {$n - 1}]\n}\n";
    let db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
    assert!(
        got.optimisations.iter().any(|o| o.code == DiagCode::O122),
        "expected O122 to fire on the tail-recursive proc"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "per-proc optimise memo diverged from the whole-module build"
    );
}

#[test]
fn original_standalone_compiler_document_keeps_root_and_later_bom_roles() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // The scalar diagnostics route shares this genuine fresh source builder.
    let source = "\u{feff}probe root\n\u{feff}probe later\n";
    for name in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
    ] {
        let environment = tcl_registry::model::ingress::resolve_known_environment(name)
            .expect("actual known source policy");
        let context = environment.default_context_registry();
        let unit = fresh_compiler_document_unit(source, context.commands(), name, None);
        assert_eq!(
            unit.ir_module.lexer_config,
            tcl_lexer::LexerConfig::for_file_grammar(environment.grammar()),
            "{name}"
        );
        assert_eq!(
            unit.ir_module.source,
            tcl_lexer::SourceImage::document(source),
            "{name}"
        );
        assert_eq!(unit.ir_module.top_level.statements.len(), 2, "{name}");
        let first = unit.ir_module.top_level.statements[0].tokens().unwrap();
        let expected = if environment.grammar().script_skips_leading_bom {
            "probe"
        } else {
            "\u{feff}probe"
        };
        assert_eq!(first.argv_texts[0], expected, "{name}");
        assert_eq!(&source[first.argv[0].as_range()], expected, "{name}");
        let later = unit.ir_module.top_level.statements[1].tokens().unwrap();
        assert_eq!(later.argv_texts[0], "\u{feff}probe", "{name}");
        assert_eq!(&source[later.argv[0].as_range()], "\u{feff}probe", "{name}");
    }
}

/// The salsa-native optimiser path must be byte-identical to a direct
/// (non-memoised) compiler-checks + optimiser build, over several dialects.
#[test]
fn compiler_check_diagnostics_matches_uncached() {
    let db = TclDatabase::default();
    for (src, dialect) in [
        ("proc a {x} { if {1} { set y 1 }\n return $y }\n", "tcl8.6"),
        ("set g 0\nproc inc {} { global g; incr g }\ninc\n", "tcl8.6"),
        (
            "proc f {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
            "tcl9.0",
        ),
        ("when HTTP_REQUEST { set u [HTTP::uri] }\n", "f5-irules"),
    ] {
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = compiler_check_diagnostics(
            &db,
            file,
            AnalyserConfig::new(
                &db,
                Vec::new(),
                NonAsciiMode::Default,
                Vec::new(),
                None,
                None,
                0,
                Vec::new(),
                Vec::new(),
            ),
        );
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "checks differ for ({dialect}):\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "optimisations differ for ({dialect}):\n{src}"
        );
    }
}

/// A `TclOO` method body / `namespace eval` body / `apply` lambda is not a
/// procedure, so it never gets an offset-0 `FnLatticeKey` and is invisible to
/// [`proc_taint_solve`]'s main `analysable_functions` loop — it is reached
/// only by that query's explicit top-up over
/// `analysable_methods_and_body_units`.  That top-up must run the whole
/// `function_nontaint_checks` family, not just its `shimmer_family_checks`
/// half: running only the shimmer half would silently drop every SCCP
/// constant-branch (`O100`) and GVN full / partial / loop-invariant
/// (`O105`/`O106`) finding inside a method or a body unit from the
/// memoised path while the direct [`compiler_check_diagnostics_uncached`]
/// build reports it — 20 missing hints on one large `TclOO` corpus file.
///
/// The existing corpus differential could not see this: it sweeps the
/// procedural `tmp/tcl*/library` + `tcllib` trees, which define almost no
/// `TclOO` methods.  This fixture pins the shape directly, and asserts the
/// counts are **non-zero** so the equality cannot pass vacuously.
#[test]
fn compiler_check_memo_covers_method_and_body_unit_checks() {
    let dialect = "tcl8.6";
    // Each body carries an `O100` (SCCP-folded existence branch) *and* an
    // `O106` (loop-invariant `lindex`), once inside a `TclOO` method, once
    // inside a `destructor`, once inside an `apply` lambda, and once inside a
    // `namespace eval` body — the four function kinds outside
    // `analysable_functions`.
    // Absolute command heads keep the TclOO bodies eligible for lexical
    // analysis: their execution namespace is receiver-selected, so a
    // relative command may be shadowed at runtime and is deliberately
    // complexity-guarded by the compiler.
    let body = "::if {[::info exists Missing]} { ::puts no }\n\
                    ::for {::set i 0} {$i < [::llength $vals]} {::incr i 2} {\n\
                        ::set v [::lindex $vals [::expr {$i+1}]]\n\
                        ::puts $v\n\
                    }\n";
    let src = format!(
        "oo::class create K {{\n\
                 variable vals\n\
                 method m {{}} {{\n{body}}}\n\
                 destructor {{\n{body}}}\n\
             }}\n\
             apply {{{{vals}} {{\n{body}}}}} {{a b}}\n\
             namespace eval ns {{\n set vals {{a b}}\n{body}}}\n"
    );

    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);

    let o1xx = |ds: &[CompilerCheck]| {
        ds.iter()
            .filter(|d| matches!(d.code, DiagCode::O100 | DiagCode::O105 | DiagCode::O106))
            .count()
    };
    assert!(
        o1xx(&want.checks) >= 4,
        "fixture must actually produce O1xx checks in the non-proc bodies, got {:?}",
        want.checks
    );
    assert_eq!(
        got.checks, want.checks,
        "memoised checks drop findings inside TclOO method / body-unit bodies (#1117)"
    );
    assert_eq!(
        o1xx(&got.checks),
        o1xx(&want.checks),
        "memoised O1xx hint count must equal the fresh build's"
    );
    assert_eq!(got.optimisations, want.optimisations);
}

/// `[info exists x]` where `x` is `TclOO` instance state
/// must not fold on **either** compiler-check path.
///
/// A class-level `variable x` binds `x` in every method frame with no
/// binding command in the body, so a naive existence fold would see a
/// never-defined local and produce an "always false" constant branch —
/// an `O100` hint (and its `I230` twin) on code that runs.  Oracle,
/// identical on tclsh 9.0.4 and 8.6.14: `oo::class create C { variable x;
/// constructor {} { set x 1 }; method m {} { puts [info exists x] } }`
/// then `[C new] m` → `1`.
///
/// The 17 corpus false positives were identical on the memoised and the
/// cold path (a fold-logic bug, not a memoisation one), so this pins both
/// halves: the instance-variable guard yields no `O100` anywhere, a
/// non-instance local in the same body still does, and the two paths
/// agree byte-for-byte.
#[test]
fn instance_variable_existence_guard_does_not_fold_on_either_check_path() {
    let dialect = "tcl8.6";
    let src = "oo::class create C {\n\
                       variable x\n\
                       constructor {} { ::set x 1 }\n\
                       method m {} {\n\
                           ::if {[::info exists x]} { ::puts got }\n\
                           ::if {[::info exists zzz]} { ::puts never }\n\
                       }\n\
                   }\n";

    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);

    // The O100 message names CFG blocks, not the source condition, so
    // identify the folded guard by slicing the span out of the source.
    let folded = |ds: &[CompilerCheck]| -> Vec<String> {
        ds.iter()
            .filter(|d| d.code == DiagCode::O100)
            .map(|d| src[d.span.start() as usize..d.span.end() as usize].to_owned())
            .collect()
    };
    assert_eq!(
        got.checks, want.checks,
        "memoised and cold compiler checks must agree on the instance-var fold"
    );
    let hints = folded(&got.checks);
    assert_eq!(
        hints.len(),
        1,
        "exactly the never-set `zzz` guard may fold, got {hints:?}"
    );
    assert!(
        hints[0].contains("zzz") && !hints[0].contains("exists x"),
        "the folded guard must be `zzz`, not the instance variable, got {hints:?}"
    );
    assert_eq!(folded(&want.checks), hints, "cold path must match");
}

/// The method/body-unit top-up must also **invalidate**: an edit inside a `TclOO`
/// method body has to move the memoised path's O1xx hints with it (and drop
/// them when the construct goes away), not serve a stale `proc_taint_solve`
/// result — the opposite failure direction from the missing-hints bug.
#[test]
fn method_body_checks_invalidate_on_edit() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    let with_hint = |pad: &str| {
        format!(
            "oo::class create K {{\n{pad}\
                 method m {{}} {{\n\
                     ::if {{[::info exists Missing]}} {{ ::puts no }}\n\
                 }}\n\
                 }}\n"
        )
    };
    // Same class with the constant-branch guard removed — no O100 at all.
    let without_hint = "oo::class create K {\n\
             method m {} {\n\
                 ::puts no\n\
             }\n\
             }\n";

    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, with_hint(""), dialect.to_owned(), None);

    let o100_spans = |ds: &[CompilerCheck]| -> Vec<(u32, u32)> {
        ds.iter()
            .filter(|d| d.code == DiagCode::O100)
            .map(|d| (d.span.start(), d.span.end()))
            .collect()
    };

    let mut seen: Vec<Vec<(u32, u32)>> = Vec::new();
    // 1. baseline, 2. body shifted by a prepended line (spans must move),
    // 3. the guard deleted (hint must disappear), 4. restored.
    for src in [
        with_hint(""),
        with_hint("\n\n"),
        without_hint.to_owned(),
        with_hint(""),
    ] {
        file.set_text(&mut db).to(src.clone());
        let got = compiler_check_diagnostics(&db, file, cfg(&db));
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        assert_eq!(got.checks, want.checks, "stale memo after edit to:\n{src}");
        seen.push(o100_spans(&got.checks));
    }
    assert_eq!(seen[0].len(), 1, "baseline must have the method-body O100");
    assert_ne!(
        seen[0], seen[1],
        "prepending lines must shift the method-body O100 span, not reuse the stale one"
    );
    assert!(
        seen[2].is_empty(),
        "deleting the guard must drop the method-body O100, got {:?}",
        seen[2]
    );
    assert_eq!(
        seen[3], seen[0],
        "restoring the guard must restore the span"
    );
}

/// Random-edit differential fuzzer for the **whole** memoised checks path.
/// The cold corpus differential and the
/// hand-written `taint_cascade_matches_uncached_under_edits` prove the memo is
/// complete and correct on a fixed edit script; this drives a **randomised**
/// sequence of incremental edits — body swaps, signature changes, and proc
/// add/remove across an interprocedural call graph — over **one warm db**,
/// asserting the memoised `compiler_check_diagnostics` (per-proc
/// `function_lattice` / `function_checks` / `proc_taint_solve` /
/// `proc_summary_cascade`) stays byte-identical (checks **and** optimisations)
/// to a from-scratch `compiler_check_diagnostics_uncached` after every edit.
/// Catches a stale per-proc cache or summary edge a fixed script would miss.
#[test]
#[allow(clippy::cast_possible_truncation)] // index modulo tiny arrays
fn compiler_check_incremental_matches_fresh_under_edits() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    // Four interdependent slots, each with several variants (index 0 = the
    // proc is absent).  `caller` invokes `pass`/`leaf`, so flipping a callee's
    // passthrough / global-write / arity must cascade into the caller's solve;
    // the `calc` slot drives SCCP const-branch + loop optimisations (O1xx).
    let pass_variants = [
        "",
        "proc pass {x} { return $x }\n", // passthrough (taint transfer)
        "proc pass {x} { return ok }\n", // constant (drops taint)
        "proc pass {x y} { return $x$y }\n", // signature change (arity 2)
    ];
    let leaf_variants = [
        "",
        "proc leaf {} { global g; incr g }\n", // global write
        "proc leaf {} { set z 1 }\n",          // pure
    ];
    let caller_variants = [
        "",
        "proc caller {} { set u [gets stdin]; set p [pass $u]; exec $p }\n", // tainted sink
        "proc caller {} { leaf; set v [pass ok]; return $v }\n",             // benign
        "proc caller {} { if {1} { return 1 }\n return [pass 2] }\n",        // const branch
    ];
    let calc_variants = [
        "",
        "proc calc {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
        "proc calc {n} { if {1} { set y 1 }\n return $y }\n",
    ];

    let assemble = |s: &[usize]| -> String {
        format!(
            "{}{}{}{}",
            pass_variants[s[0]], leaf_variants[s[1]], caller_variants[s[2]], calc_variants[s[3]],
        )
    };
    let lens = [
        pass_variants.len(),
        leaf_variants.len(),
        caller_variants.len(),
        calc_variants.len(),
    ];

    // xorshift64 — deterministic, reproducible run-to-run.
    let mut rng = 0xfeed_face_cafe_d00d_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let mut state = [1usize, 1, 1, 1];
    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, assemble(&state), dialect.to_owned(), None);

    for iter in 0..250 {
        let slot = (next() as usize) % state.len();
        state[slot] = (next() as usize) % lens[slot];
        let src = assemble(&state);
        file.set_text(&mut db).to(src.clone());

        let got = compiler_check_diagnostics(&db, file, cfg(&db));
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "iter {iter}: checks diverge from fresh build for state {state:?}:\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "iter {iter}: optimisations diverge from fresh build for state {state:?}:\n{src}"
        );
    }
}

/// Direct measurement of per-edit *check* breadth:
/// a one-procedure body edit rebuilds exactly ONE `function_lattice` (the
/// per-proc memo works), but `compiler_check_diagnostics` re-executes wholesale
/// — `run_all_checks` is not a per-proc salsa query (it emits no `WillExecute`
/// event of its own), so it re-checks every procedure on every edit.  The
/// assertions pin the current breadth.
#[test]
fn check_diagnostics_rerun_whole_file_on_body_edit() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // Four independent procedures; we edit `b`'s body and leave a, c, d alone.
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    log.lock().unwrap().clear();

    // Length-changing body edit to ONE procedure (`b`); a/c/d shift but their
    // offset-0 lattice keys are unchanged (cache hits).
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 222222222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
        .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let _ = compiler_check_diagnostics(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    let count = |q: &str| after.iter().filter(|s| s.contains(q)).count();

    // Per-proc memo: exactly one procedure's lattice rebuilds.
    assert_eq!(
        count("function_lattice"),
        1,
        "one-proc body edit -> ONE function_lattice rebuild: {after:?}"
    );
    // Checks re-run wholesale: compiler_check_diagnostics re-executes, and
    // run_all_checks (not a salsa query) re-checks every procedure inside it.
    assert_eq!(
        count("compiler_check_diagnostics"),
        1,
        "checks re-run whole-file every edit (no per-proc check memo): {after:?}"
    );
    assert_eq!(
        count("run_all_checks"),
        0,
        "run_all_checks is not a salsa query, so it has no WillExecute event: {after:?}"
    );
}

#[test]
fn fresh_compiler_document_retains_actual_input_and_selected_registry() {
    let db = TclDatabase::default();
    let analysis_config = cfg(&db);
    for dialect in ["tcl", "tcl8.6"] {
        let registry = db.registry(dialect);
        let file = SourceFile::new(&db, "set value 1".to_owned(), dialect.to_owned(), None);
        let input = document_analysis_input(&db, file, analysis_config).unwrap();
        let fresh = fresh_compiler_document_unit(file.text(&db), registry, dialect, None);
        assert_eq!(
            fresh.ir_module.source_metadata_input.as_ref(),
            Some(input.as_ref())
        );
        assert!(fresh.ir_module.source_entry.native_entry.is_none());
        assert!(
            !fresh
                .ir_module
                .source_entry
                .metadata_context
                .is_standalone()
        );
        assert_eq!(
            fresh
                .ir_module
                .source_metadata_input
                .as_ref()
                .unwrap()
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key(),
            registry.snapshot().semantic_key()
        );
    }
    let registry = db.registry("tcl8.6");
    let profile = registry.profile().unwrap();
    let mut changed = registry.project_for_profile(profile);
    let mut spec = changed.get("puts").unwrap().clone();
    spec.traits.insert(tcl_registry::Traits::EVALUATES_CODE);
    changed.insert(spec);
    let fresh = fresh_compiler_document_unit("puts VALUE", &changed, "tcl8.6", None);
    assert_eq!(
        fresh
            .ir_module
            .source_metadata_input
            .as_ref()
            .unwrap()
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        changed.snapshot().semantic_key()
    );
    assert_ne!(
        changed.snapshot().semantic_key(),
        registry.snapshot().semantic_key()
    );
}

#[test]
fn compiler_taint_replay_status_is_retained_by_memo_and_direct_paths() {
    let db = TclDatabase::default();
    let source = "set value 1";
    let dialect = "tcl8.6";
    let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
    let memo = compiler_check_diagnostics(&db, file, cfg(&db));
    let registry = db.registry(dialect);
    let fresh = compiler_check_diagnostics_uncached(source, registry, dialect, None, None);
    assert_eq!(memo.checks, fresh.checks);
    assert_eq!(memo.taint_replays, fresh.taint_replays);
    let unavailable = abstain_diagnostics();
    assert_eq!(unavailable.checks.len(), 0);
    assert!(!unavailable.taint_replays.is_complete());
    assert!(matches!(
        unavailable.taint_replays,
        tcl_compiler::taint::TaintReplayAnalysis::Unavailable { .. }
    ));
}
