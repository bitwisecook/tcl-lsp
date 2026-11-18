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

//! The taint cascade and the procedure-summary memo: parity with the uncached solve and reuse across unrelated edits.

use super::*;

#[test]
fn configured_taint_checks_consume_the_solve_original_module_owner() {
    let source = "proc pass {x} { return $x }\n\
                      proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
                      proc other {} { set z 1 }\n";
    let dialect = "tcl8.6";
    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
    let policy = cfg(&db);
    let varied = cfg(&db);
    salsa::Setter::to(
        varied.set_extra_commands(&mut db),
        vec!["churnpack::noop".to_owned()],
    );
    let key = lexer_cfg_key(&db, dialect);
    let input = document_analysis_input(&db, file, policy).unwrap();
    let solved = proc_taint_solve_for_config(&db, file, key, policy).unwrap();
    let owner = solved.unit.as_ref();
    assert_eq!(
        owner.ir_module.source,
        tcl_lexer::SourceImage::document(source)
    );
    assert_eq!(
        owner.ir_module.source_metadata_input.as_ref(),
        Some(input.as_ref())
    );
    assert_eq!(owner.ir_module.lexer_config, input.lexer_config());
    assert!(!solved.taints.proc_taints.is_empty());
    // Builds with equal lowering inputs share one lowered module (#2543), so
    // the shared unit may own the same ids; a build under a different input
    // owns a different module, and none of the solve's ids belong to it.
    let other = compilation_unit_for_config(&db, file, key, varied).unwrap();
    for (id, taints) in &solved.taints.proc_taints {
        assert!(owner.ir_module.procedure_ids.owns(*id));
        assert!(!other.ir_module.procedure_ids.owns(*id));
        let function = owner.function_for_body(Some(*id)).unwrap();
        assert!(std::ptr::eq(
            solved.taints.taints_for_function(owner, function),
            taints,
        ));
    }
    let registry = input.borrowed_context_registry().commands();
    let expected = compiler_check_diagnostics_uncached(source, registry, dialect, None, None);
    assert!(
        expected
            .checks
            .iter()
            .any(|check| check.code == DiagCode::T100)
    );
    let actual = compiler_check_diagnostics(&db, file, policy);
    assert_eq!(actual.checks, expected.checks);
    assert_eq!(actual.optimisations, expected.optimisations);
}

/// The interprocedural taint cascade (`taint_cascade`, backlog #1) must stay
/// byte-identical to the non-memoised `with_interprocedural` re-run **across
/// edits** — the cold corpus differential only proves the reconstruction is
/// complete, not that a stale cache can't survive an edit.  Drive a sequence
/// of edits (including one that flips a callee's passthrough/global-write
/// behaviour, which must invalidate its callers' cascades) and assert the
/// memoised diagnostics equal a fresh uncached build at every step.
#[test]
pub(super) fn taint_cascade_matches_uncached_under_edits() {
    use salsa::Setter as _;
    let dialect = "tcl8.6";
    // A passthrough callee feeding a destructive sink in a caller, plus an
    // unrelated proc — exercises return-passthrough taint transfer and the
    // reachable-global seeding the cascade depends on.
    let versions = [
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1 }\n",
        // Unrelated edit (other's body) — callers' cascades must be reused
        // yet still correct.
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
        // Flip the callee: no longer a passthrough (returns a constant) —
        // danger's cascade must recompute and drop the transferred taint.
        "proc pass {x} { return ok }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
        // Restore the passthrough — taint transfer must come back.
        "proc pass {x} { return $x }\n\
             proc danger {} { set u [gets stdin]; set p [pass $u]; exec $p }\n\
             proc other {} { set z 1; set z 2 }\n",
    ];
    // One warm db across the whole edit sequence — a fresh db per edit would
    // not exercise stale-cache reuse, which is the point.
    let mut db = TclDatabase::default();
    let registry = db.registry(dialect);
    let file = SourceFile::new(&db, versions[0].to_owned(), dialect.to_owned(), None);
    for src in versions {
        file.set_text(&mut db).to(src.to_owned());
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
        let want = compiler_check_diagnostics_uncached(src, registry, dialect, None, None);
        assert_eq!(
            got.checks, want.checks,
            "cascade checks diverge after edit to:\n{src}"
        );
        assert_eq!(
            got.optimisations, want.optimisations,
            "cascade optimisations diverge after edit to:\n{src}"
        );
    }
}

/// The taint cascade memoises: a body edit to a procedure that no other
/// procedure's taint depends on must **not** re-execute the unrelated
/// procedures' `taint_cascade` (they reuse their cached taints), whereas the
/// edited procedure's own cascade does re-run.  Proves backlog #1 actually
/// skips the per-procedure `propagate_taints` re-run that
/// `with_interprocedural` did unconditionally.
#[test]
fn taint_cascade_reused_on_unrelated_edit() {
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
    let cascades = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("taint_cascade"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    // Three procedures with no taint-relevant call edges between them.
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(
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
    assert_eq!(
        cascades(&log),
        3,
        "cold build: every procedure's taint cascade runs"
    );

    // Edit only `b`'s body — `a`/`c` are unaffected and their cascades are
    // cache hits; only `b`'s re-runs.
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 99999999 }\n\
             proc c {} { set z 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(
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
    assert_eq!(
        cascades(&log),
        1,
        "unrelated body edit -> exactly ONE taint cascade recomputes"
    );
}

/// The interprocedural summary fixpoint memo (`proc_summary_cascade`)
/// must skip an unrelated procedure's `infer_proc_summary`
/// across edits: a body edit to one proc re-keys only that proc's summary
/// (its `FnLatticeKey` changed), while procedures it does not feed are cache
/// hits — this is what collapses the worklist's whole-unit pass-1 floor to the
/// edited proc's caller cascade.  Three procedures with no taint-relevant call
/// edges, so each is its own cascade root.
#[test]
fn proc_summary_cascade_reused_on_unrelated_edit() {
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
    let summaries = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("proc_summary_cascade"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = compiler_check_diagnostics(
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
    assert_eq!(
        summaries(&log),
        3,
        "cold build: every procedure's summary inference runs"
    );

    // Edit only `b`'s body — `a`/`c` summaries are cache hits; only `b`'s
    // `proc_summary_cascade` re-executes when its original summary inputs change.
    file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 99999999 }\n\
             proc c {} { set z 33333 }\n"
        .to_owned());
    let _ = compiler_check_diagnostics(
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
    assert_eq!(
        summaries(&log),
        1,
        "unrelated body edit -> exactly ONE summary inference recomputes"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryDependencyProbe {
    caller_name: String,
    caller_body: String,
    provider: ReturnTaintSummary,
    dependencies: Vec<ReturnTaintSummary>,
    caller: ReturnTaintSummary,
}

#[salsa::tracked(returns(clone))]
fn summary_dependency_probe<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    analysis_config: AnalyserConfig,
    caller_offset: usize,
    caller: String,
) -> SummaryDependencyProbe {
    let dialect = file.dialect(db);
    let input =
        walked_document_input(db, file, analysis_config).expect("actual selected document input");
    let registry = input.borrowed_context_registry().commands();
    let declared = declared_command_surface(db, file, registry);
    let options = supplied_unit_build_options(db, file, cfg, registry, None, &declared, &input)
        .expect("the document's original grammar and Registry");
    let (cu, _) = build_unit_with_keys_and_input(
        db,
        (Some(file), file.text(db)),
        options,
        Some(&input),
        None,
    );
    let source_bodies: Vec<_> = cu
        .ir_module
        .procedures
        .iter()
        .chain(cu.ir_module.body_units.iter())
        .filter(|(id, _)| cu.original_procedure_for_body(**id).is_some())
        .collect();
    let known: HashSet<_> = source_bodies.iter().map(|(id, _)| **id).collect();
    let mut summaries: HashMap<_, _> = source_bodies
        .into_iter()
        .map(|(id, procedure)| {
            (
                *id,
                ReturnTaintSummary::untainted(&procedure.qualified_name, &procedure.params),
            )
        })
        .collect();
    let provider_id = original_test_body_id(&cu, 0);
    let caller_id = original_test_body_id(&cu, caller_offset);
    let provider = tcl_compiler::taint_interproc::infer_proc_summary(
        &cu,
        provider_id,
        registry,
        cu.interproc.as_ref(),
        tcl_lsp_core::stated_profile_for_dialect(dialect),
        &known,
        &summaries,
    )
    .expect("actual provider source unit");
    summaries.insert(provider_id, provider.clone());
    let body = cu
        .original_procedure_for_body(caller_id)
        .unwrap()
        .body_source
        .clone()
        .expect("actual caller source body");
    let capsule = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
        Arc::new(cu),
        caller_id,
        registry,
    )
    .expect("actual current compiler owners");
    let dependencies = summary_deps_key(db, capsule, &summaries, &known, dialect);
    let summary = proc_summary_cascade(db, dependencies);
    SummaryDependencyProbe {
        caller_name: caller,
        caller_body: body,
        provider,
        dependencies: dependencies
            .inputs(db)
            .summaries()
            .iter()
            .map(|(_, summary)| summary.clone())
            .collect(),
        caller: (*summary).clone(),
    }
}

fn summary_memo_source(provider: &str) -> String {
    format!(
        "proc producer {{}} {{{provider}}}\n\
             proc quoted {{}} {{return [\"producer\"]}}\n\
             proc braced {{}} {{return [{{producer}}]}}\n"
    )
}

fn summary_memo_callers(text: &str) -> [(&'static str, usize); 2] {
    [
        ("::quoted", text.find("proc quoted ").unwrap()),
        ("::braced", text.find("proc braced ").unwrap()),
    ]
}

fn summary_memo_run_count(log: &Arc<Mutex<Vec<String>>>) -> usize {
    std::mem::take(&mut *log.lock().unwrap())
        .into_iter()
        .filter(|query| query.contains("proc_summary_cascade"))
        .count()
}

fn summary_memo_probes<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    config: LexerCfgKey<'db>,
    analysis_config: AnalyserConfig,
) -> Vec<SummaryDependencyProbe> {
    summary_memo_callers(file.text(db))
        .into_iter()
        .map(|(caller, offset)| {
            summary_dependency_probe(db, file, config, analysis_config, offset, caller.to_owned())
        })
        .collect()
}

#[test]
fn summary_memo_observes_callee_edits_through_original_quoted_and_braced_heads() {
    // naming.compiler.original-summary-dependency-source-heads
    // docs/design/analysis/name-resolution-proofs/compiler-original-summary-dependency-source-heads.md
    // Actual document units feed the tracked summary query. This proves
    // dependency currency, not that a procedure or substitution executes.
    use salsa::Setter as _;
    let log = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let log = Arc::clone(&log);
        move |event: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = event.kind {
                log.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let initial = summary_memo_source("return CLEAN");
    let changed = summary_memo_source("return [gets stdin]");
    let file = SourceFile::new(&db, initial, "tcl".to_owned(), None);
    let config = lexer_cfg_key(&db, "tcl");
    let analysis_config = cfg(&db);
    let before = summary_memo_probes(&db, file, config, analysis_config);
    assert_eq!(
        summary_memo_run_count(&log),
        2,
        "two actual caller summary queries"
    );
    for (caller, offset) in summary_memo_callers(file.text(&db)) {
        let _ = summary_dependency_probe(
            &db,
            file,
            config,
            analysis_config,
            offset,
            caller.to_owned(),
        );
    }
    assert_eq!(
        summary_memo_run_count(&log),
        0,
        "unchanged document retains the summary memos"
    );
    file.set_text(&mut db).to(changed.clone());
    let config = lexer_cfg_key(&db, "tcl");
    let after = summary_memo_probes(&db, file, config, analysis_config);
    assert_eq!(
        summary_memo_run_count(&log),
        2,
        "callee summary edit invalidates both caller queries"
    );
    for (old, new) in before.iter().zip(&after) {
        assert_eq!(
            old.caller_body, new.caller_body,
            "caller source did not change"
        );
        assert_ne!(
            old.provider, new.provider,
            "the actual callee summary changed"
        );
        assert!(old.dependencies.contains(&old.provider));
        assert!(new.dependencies.contains(&new.provider));
        assert_ne!(old.dependencies, new.dependencies);
    }
    let fresh = TclDatabase::default();
    let fresh_file = SourceFile::new(&fresh, changed, "tcl".to_owned(), None);
    let fresh_config = lexer_cfg_key(&fresh, "tcl");
    let fresh_analysis_config = cfg(&fresh);
    for ((caller, offset), expected) in summary_memo_callers(fresh_file.text(&fresh))
        .into_iter()
        .zip(after)
    {
        assert_eq!(
            summary_dependency_probe(
                &fresh,
                fresh_file,
                fresh_config,
                fresh_analysis_config,
                offset,
                caller.to_owned()
            ),
            expected
        );
    }
}
