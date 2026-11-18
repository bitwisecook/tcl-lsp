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

//! The checks-path solve: the interprocedural taint cascade, procedure summaries and the per-procedure checks.

use super::compilation_units::{
    LexerCfgKey, abstain, lattice_registry, supplied_unit_build_options, unit_build_options,
    unit_registry,
};
use super::declared_commands::declared_command_surface;
use super::document_analysis::{document_analyser, walked_document_input};
use super::function_lattices::{FnLatticeKey, function_lattice};
use super::memoised_units::{build_unit_with_keys, build_unit_with_keys_and_input, memo_key};
use super::optimisation_solve::solve_optimisations;
use super::{AnalyserConfig, CheckSolve, SourceFile, TclDb};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::compiler_checks::Diagnostic as CompilerCheck;
use tcl_compiler::ssa::ValueKey;
use tcl_compiler::taint::TaintLattice;
use tcl_compiler::taint_interproc::ProcTaintSummary as ReturnTaintSummary;
use tcl_registry::CommandRegistry;

/// Actual compilation owner for a per-body interprocedural taint demand.
#[salsa::interned]
pub struct TaintSummaryKey<'db> {
    #[returns(ref)]
    pub capsule: tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule,
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn taint_cascade<'db>(
    db: &'db dyn TclDb,
    summary_key: TaintSummaryKey<'db>,
) -> Arc<HashMap<ValueKey, TaintLattice>> {
    let capsule = summary_key.capsule(db);
    let registry = capsule.registry_snapshot().shared_registry();
    let taints = capsule.unit().interproc.as_ref().map_or_else(
        || (*capsule.function().taints).clone(),
        |analysis| {
            capsule
                .unit()
                .original_interproc_taints_for_body(Some(capsule.body_id()), &registry, analysis)
                .unwrap_or_else(|| (*capsule.function().taints).clone())
        },
    );
    Arc::new(taints)
}

/// Actual body taint request, retaining its effective dialect independently of
/// the source module's optional reporting profile. The legacy map memo above
/// remains available to its original callers without issuing source ownership.
#[salsa::interned]
pub struct OriginalTaintSummaryKey<'db> {
    #[returns(ref)]
    pub capsule: tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule,
    #[returns(copy)]
    pub effective_dialect: Option<tcl_dialect::DialectProfileKey>,
    /// Actual prepared worker context; None is a reporting-only purpose.
    #[returns(ref)]
    pub original_context: Option<tcl_compiler::value_transfer::AnalysisContextKey>,
}

/// One real taint worker pass and its sealed request correspondence. Missing
/// issuance keeps its reporting map and never reruns the worker to gain it.
#[salsa::tracked(lru = 512, returns(clone))]
pub fn original_taint_cascade<'db>(
    db: &'db dyn TclDb,
    key: OriginalTaintSummaryKey<'db>,
) -> Arc<tcl_compiler::body_analysis_capsule::OriginalInterproceduralTaintReport> {
    let capsule = key.capsule(db);
    let registry = capsule.registry_snapshot().shared_registry();
    let dialect = key.effective_dialect(db).map(|profile| profile.profile());
    Arc::new(capsule.original_interproc_taint_report(&registry, dialect))
}

/// Original fixpoint inputs with independently certified body correspondence.
#[salsa::interned]
pub struct SummaryDepsKey<'db> {
    #[returns(ref)]
    pub inputs: tcl_compiler::body_analysis_capsule::BodySummaryInputs,
    #[returns(ref)]
    pub dialect: String,
}

pub(super) fn summary_deps_key<'db>(
    db: &'db dyn TclDb,
    capsule: tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule,
    summaries: &HashMap<tcl_compiler::ir::ProcedureId, ReturnTaintSummary>,
    known: &HashSet<tcl_compiler::ir::ProcedureId>,
    dialect: &str,
) -> SummaryDepsKey<'db> {
    let inputs =
        tcl_compiler::body_analysis_capsule::BodySummaryInputs::retain(capsule, known, summaries)
            .expect("the solver supplies current original procedure coordinates and formals");
    SummaryDepsKey::new(db, inputs, dialect.to_owned())
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn proc_summary_cascade<'db>(
    db: &'db dyn TclDb,
    deps_key: SummaryDepsKey<'db>,
) -> Arc<ReturnTaintSummary> {
    let inputs = deps_key.inputs(db);
    let capsule = inputs.capsule();
    let known = inputs.known().iter().copied().collect();
    let summaries = inputs.summaries().iter().cloned().collect();
    let registry = capsule.registry_snapshot().shared_registry();
    Arc::new(
        tcl_compiler::taint_interproc::infer_proc_summary(
            capsule.unit(),
            capsule.body_id(),
            &registry,
            capsule.unit().interproc.as_ref(),
            capsule.unit().ir_module.dialect_profile,
            &known,
            &summaries,
        )
        .expect("capsule retains the genuine root declaration and function"),
    )
}

/// Memoised per-procedure **non-taint** compiler checks (SCCP constant branches,
/// GVN redundancies, shimmer / thunking / byte-array) for one procedure's
/// offset-0 baseline — the `function_lattice` analogue for the checks pass.
/// Returns spans at **offset 0** (it computes on the
/// offset-0 [`function_lattice`] unit, *before* `rebase_function_unit`); the
/// caller adds the procedure's `body_offset`.  A body edit re-runs only the
/// edited procedure's checks; every other proc is a cache hit.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn function_checks<'db>(db: &'db dyn TclDb, key: FnLatticeKey<'db>) -> Arc<Vec<CompilerCheck>> {
    let fu = function_lattice(db, key);
    let dialect_opt = key.snapshot(db).profile.map(|profile| profile.profile());
    let registry = lattice_registry(db, key);
    let registry: &CommandRegistry = &registry;
    // Per-procedure memo — procs have no implicit instance variables.
    Arc::new(tcl_compiler::compiler_checks::function_nontaint_checks(
        &fu,
        registry,
        dialect_opt,
        None::<&std::collections::HashSet<String>>,
    ))
}

/// The checks-path memoised solve for one document.
///
/// Runs on the **checks path only** (demanded by [`compiler_check_diagnostics`],
/// never the analyser walk / `semantic_tokens`), so it cannot regress
/// time-to-first-tokens.  Re-derives the offset-0 [`FnLatticeKey`]s with its own
/// [`build_unit_with_keys`] (they cannot be shared from [`compilation_unit`] —
/// salsa returns must be `'static`; the duplicate build is mostly
/// `function_lattice` cache hits, ~28 ms warm), then from that one build produces
/// both:
/// * the interprocedural taint solve via
///   [`tcl_compiler::taint_interproc::solve_interprocedural_taints_with`] with an
///   `infer` deferring to [`proc_summary_cascade`] (collapses the ~120 ms pass-1
///   floor);
/// * the per-procedure non-taint checks via [`function_checks`] (an
///   unchanged proc's checks are a cache hit), each rebased here by the
///   procedure's `body_offset` (`ir_module.procedures[qname].span.start()` — the
///   same delta `rebase_function_unit` applies in the whole-module build, since
///   `function_checks` returns offset-0 spans).
///
/// Rebase one memoised offset-0 check onto its procedure's `body_offset` —
/// the [`proc_taint_solve`] twin of the whole-module build's
/// `rebase_function_unit` delta.
///
/// The diagnostic's own span rebases only when real: the `(0, 0)` "unknown
/// span" sentinel (an O100 constant branch whose `cb.span` is `None`) renders
/// to `(0, 0)` in *both* paths — the whole-module build rebases the
/// `Option<Span>` (so `None` stays `None`) *before* the `None → (0,0)`
/// lowering, so the offset must not be added here.  A fix's edit span is
/// always a real location on the offset-0 unit (never the sentinel), so it
/// rebases unconditionally — the same parity `compiler_checks::shift` keeps
/// on the whole-module path.  No per-function check carries fixes today, but
/// the first one that gains a quick fix must not silently edit at an
/// unrebased offset.
fn rebase_check(mut d: CompilerCheck, body_offset: u32) -> CompilerCheck {
    if d.span.start() != 0 || d.span.end() != 0 {
        d.span = tcl_lexer::Span::new(d.span.start() + body_offset, d.span.end() + body_offset);
    }
    for fix in &mut d.fixes {
        fix.span =
            tcl_lexer::Span::new(fix.span.start() + body_offset, fix.span.end() + body_offset);
    }
    d
}

/// Byte-identical to a bare `run_all_checks`, guarded by the `compiler_check`
/// corpus differential + the debug fixpoint guard.
///
/// `None` when the workspace's packs are not installed under `overlay`
/// ([`compilation_unit`] says why nothing is built without them).
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn proc_taint_solve<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    overlay: u64,
) -> Option<Arc<CheckSolve>> {
    let dialect = file.dialect(db).clone();
    let Ok(registry) = unit_registry(db, &dialect, overlay) else {
        return abstain();
    };
    let registry: &CommandRegistry = &registry;
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file, registry);
    let (cu, lattice_keys) = build_unit_with_keys(
        db,
        (Some(file), file.text(db)),
        unit_build_options(db, file, cfg, registry, external.as_deref(), &declared),
    );
    Some(solve_checks_for_unit(
        db,
        &dialect,
        registry,
        cu,
        &lattice_keys,
        tcl_lsp_core::stated_profile_for_dialect(&dialect),
    ))
}

fn solve_checks_for_unit<'db>(
    db: &'db dyn TclDb,
    dialect: &str,
    registry: &CommandRegistry,
    cu: CompilationUnit,
    lattice_keys: &HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
    dialect_opt: Option<&'static tcl_dialect::DialectProfile>,
) -> Arc<CheckSolve> {
    let retained = Arc::new(cu);
    let cu = retained.as_ref();
    let taints = tcl_compiler::taint_interproc::solve_interprocedural_taints_with(
        cu,
        registry,
        dialect_opt,
        &mut |actual, body_id, analysis, known, summaries| {
            let fresh = || {
                tcl_compiler::taint_interproc::infer_proc_summary(
                    actual,
                    body_id,
                    registry,
                    analysis,
                    dialect_opt,
                    known,
                    summaries,
                )
                .expect("solver supplied its actual procedure and function")
            };
            if analysis != retained.interproc.as_ref() {
                return fresh();
            }
            let Some(capsule) = tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
                Arc::clone(&retained),
                body_id,
                registry,
            ) else {
                return fresh();
            };
            let dependencies = summary_deps_key(db, capsule, summaries, known, dialect);
            (*proc_summary_cascade(db, dependencies)).clone()
        },
    );

    // Per-procedure non-taint checks.  The memoised [`function_checks`] returns
    // **offset-0** spans (it runs on the offset-0 `function_lattice` unit), so
    // [`rebase_check`] adds the procedure's `body_offset` here — the same rebase
    // the whole-module build's `rebase_function_unit` applies.  A proc without a
    // lattice key (e.g. the top level, or a complexity-guarded body) falls back
    // to the direct per-function computation on the *already-rebased* built unit
    // (no offset add).
    let mut fn_checks: Vec<CompilerCheck> = Vec::new();
    for fu in cu.analysable_body_function_units() {
        match memo_key(db, lattice_keys, fu.body_id) {
            Some(key) => {
                let body_offset = cu
                    .original_procedure_for_body(
                        fu.body_id
                            .expect("memo selection retained the exact body ID"),
                    )
                    .expect("memo selection retained the original declaration")
                    .span
                    .start();
                fn_checks.extend(
                    function_checks(db, key)
                        .iter()
                        .map(|d| rebase_check(d.clone(), body_offset)),
                );
            }
            None => {
                // The built unit's fallback fus (complexity-guarded / top level,
                // or built afresh) carry **absolute** spans already
                // (`base_offset == 0`), so the per-function checks need no
                // rebase.
                for d in tcl_compiler::compiler_checks::function_nontaint_checks(
                    fu,
                    registry,
                    dialect_opt,
                    cu.method_instance_vars(fu.body_id),
                ) {
                    fn_checks.push(d);
                }
            }
        }
    }

    let optimisations = solve_optimisations(db, cu, lattice_keys, registry, dialect_opt);
    Arc::new(CheckSolve {
        unit: retained,
        taints,
        fn_checks,
        optimisations,
    })
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn proc_taint_solve_for_config<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    config: AnalyserConfig,
) -> Option<Arc<CheckSolve>> {
    let input = walked_document_input(db, file, config).ok()?;
    let registry = input.borrowed_context_registry().commands();
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file, registry);
    let options = supplied_unit_build_options(
        db,
        file,
        cfg,
        registry,
        external.as_deref(),
        &declared,
        &input,
    )?;
    let settings = document_analyser(db, file, config).source_compilation_settings();
    let (cu, keys) = build_unit_with_keys_and_input(
        db,
        (Some(file), file.text(db)),
        options,
        Some(&input),
        Some(&settings),
    );
    Some(solve_checks_for_unit(
        db,
        file.dialect(db),
        registry,
        cu,
        &keys,
        Some(input.unit_profile()),
    ))
}
