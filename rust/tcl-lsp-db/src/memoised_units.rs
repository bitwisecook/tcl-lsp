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

//! A compilation unit assembled from the per-body memos: lowered procedure bodies, lattice keys and lattice candidates.

use super::check_solve::{OriginalTaintSummaryKey, original_taint_cascade};
use super::function_lattices::{
    CfgContext, CompilerMemoSnapshot, FnLatticeEntry, FnLatticeKey, FnLatticeModuleFacts,
    ValueTransferContext, function_lattice,
};
use super::{TclDb, evaluator_epoch};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
use tcl_compiler::cfg_builder::OriginalProcedureSummaryOwners;
use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::compilation_unit::{
    CompilationUnit, FunctionUnit, LatticeRequest, UnitBuildOptions,
};
use tcl_compiler::ir::Script;
use tcl_registry::CommandRegistry;

/// Interned identity of one top-level `proc`'s **offset-0** static body source
/// (incremental per-item IR *lowering*).  A `proc`
/// body is lowered against a clean slate (`lower_proc` pushes an empty const-map
/// frame, so the body inherits no tracked scalars from preceding code), so its
/// lowering is a pure function of `(body_text, namespace, dialect, config)` —
/// no position, no cross-item state.  An edit to one proc's body changes only
/// that proc's `ProcBodyKey`, so salsa reuses every other proc body's lowered
/// IR; an edit that merely *shifts* a body leaves its key unchanged (the caller
/// rebases the offset-0 `Script` back to the body's real offset).  Used only for
/// **context-free bodies** (the positioned binding owner proves cache eligibility)
/// where the isolated lowering is byte-identical to the in-place `lower_body`;
/// guarded by the corpus differential gates (`file_analysis_corpus` /
/// `compiler_check_corpus`).
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `body_text` changes on every keystroke inside the body; the lowered
/// `Arc<Script>` memo behind the stale id is released when the collector reuses
/// its slot, which happens only for `Durability::LOW` slots interned inside a
/// tracked query.  See the crate docs' "The interned garbage collector is
/// load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct ProcBodyKey<'db> {
    #[returns(ref)]
    pub body_text: String,
    #[returns(ref)]
    pub namespace: String,
    #[returns(ref)]
    pub dialect: String,
    /// Full profile and actual authored command surface at ingress.
    #[returns(ref)]
    pub snapshot: CompilerMemoSnapshot,
    /// Exact normalised grammar used by the original body lowering.
    #[returns(copy)]
    pub lexer_config: tcl_lexer::LexerConfig,
}

/// Memoised offset-0 isolated lowering of one top-level `proc` body.
/// Replicates the body-lowering setup `lower_proc`
/// performs for a static literal body (a fresh `Lowerer` at `proc_depth == 1`
/// with an empty const-map frame, lowering at offset 0).  Byte-identical to the
/// body the whole-file lowering produces for that procedure, normalised to
/// offset 0 — for the context-free files the caller gates on.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn lower_proc_body<'db>(db: &'db dyn TclDb, key: ProcBodyKey<'db>) -> Arc<Script> {
    Arc::new(original_lower_proc_body(db, key).body().clone())
}

/// One actual isolated lowering pass, retaining its complete worker recipe.
/// The compatibility Script query above projects this same immutable result.
#[salsa::tracked(lru = 512, returns(clone))]
pub fn original_lower_proc_body<'db>(
    db: &'db dyn TclDb,
    key: ProcBodyKey<'db>,
) -> tcl_compiler::lowering::OriginalBodyLoweringTemplate {
    let registry = key.snapshot(db).registry.registry();
    let profile = key.snapshot(db).profile.map(|snapshot| snapshot.profile());
    tcl_compiler::lowering::OriginalBodyLoweringTemplate::lower(
        key.body_text(db),
        key.namespace(db),
        registry,
        key.lexer_config(db),
        profile,
    )
}

/// Dependency-certified original source syntax request.
#[salsa::interned]
pub struct SourceProcBodyKey<'db> {
    #[returns(ref)]
    pub request: tcl_compiler::lowering::SourceBodySyntaxRequest,
}

/// The lowering inputs of one document build other than its text, by value.
#[salsa::interned]
pub struct ModuleLoweringKey<'db> {
    #[returns(ref)]
    pub inputs: tcl_compiler::compilation_unit::ModuleLoweringInputs,
}

/// One document's lowered module under one set of lowering inputs, shared by
/// every build of that document with equal inputs, so an unchanged document
/// rebuilt under a revisited configuration keeps its procedure identities and
/// source proofs and every lattice key built from them recurs (#2543).
///
/// Keyed by the document and its inputs other than the text, which is read
/// here: an edit re-executes the one entry per (document, inputs) in place, so
/// a superseded text's module is dropped rather than held until eviction, and
/// the entries are the live (document, configuration) pairs. The cap matches
/// the per-configuration unit memo's (`compilation_unit_for_config`). `None`
/// when lowering reported a body-cache integrity failure; the build then
/// lowers for itself.
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn lowered_document_module<'db>(
    db: &'db dyn TclDb,
    file: crate::SourceFile,
    key: ModuleLoweringKey<'db>,
) -> Option<Arc<tcl_compiler::compilation_unit::LoweredModule>> {
    let inputs = key.inputs(db).with_source(file.text(db));
    let retained_key = (salsa::plumbing::AsId::as_id(&file), key.inputs(db).clone());
    if let Some(module) = retained_modules()
        .lock()
        .expect("retained module table")
        .get(&retained_key)
        .filter(|module| module.inputs() == &inputs)
    {
        return Some(Arc::clone(module));
    }
    let (body_memo, source_body_memo) = lowering_body_memos(
        db,
        inputs.dialect(),
        inputs.registry(),
        inputs.config().normalized(),
    );
    let module = tcl_compiler::compilation_unit::LoweredModule::lower(
        &inputs,
        &body_memo,
        &source_body_memo,
    )
    .map(Arc::new)?;
    let mut retained = retained_modules().lock().expect("retained module table");
    if retained.len() >= RETAINED_MODULE_CAPACITY && !retained.contains_key(&retained_key) {
        retained.clear();
    }
    retained.insert(retained_key, Arc::clone(&module));
    Some(module)
}

type RetainedModuleKey = (
    salsa::Id,
    tcl_compiler::compilation_unit::ModuleLoweringInputs,
);

/// The memo's own capacity: one entry per live (document, configuration).
const RETAINED_MODULE_CAPACITY: usize = 512;

/// The lowered module of each (document, inputs other than the text), held
/// apart from salsa. Salsa's interned collector reclaims a
/// [`ModuleLoweringKey`] untouched for a few revisions, and its memo with it;
/// a server settles several queries per pack reload, so a document revisited
/// under a configuration would be lowered again with new procedure
/// identities and every lattice key built from them would be new (#2543).
/// An entry is returned only for inputs, text included, equal to the ones it
/// was lowered from, so it is the module a fresh lowering would produce; an
/// edit replaces the document's entry.
fn retained_modules() -> &'static std::sync::Mutex<
    HashMap<RetainedModuleKey, Arc<tcl_compiler::compilation_unit::LoweredModule>>,
> {
    static RETAINED: std::sync::OnceLock<
        std::sync::Mutex<
            HashMap<RetainedModuleKey, Arc<tcl_compiler::compilation_unit::LoweredModule>>,
        >,
    > = std::sync::OnceLock::new();
    RETAINED.get_or_init(Default::default)
}

/// Lower neutral call syntax from genuine pre-lowering source dependencies.
#[salsa::tracked(lru = 512, returns(clone))]
pub fn lower_proc_body_source<'db>(
    db: &'db dyn TclDb,
    key: SourceProcBodyKey<'db>,
) -> tcl_compiler::lowering::SourceBodySyntaxTemplate {
    key.request(db).lower_neutral()
}

/// Build a `CompilationUnit` (with interprocedural summary applied) whose
/// per-procedure baseline lattices are memoised by the salsa-native
/// [`function_lattice`] query.
///
/// Shared by the analyser's CFG/SSA diagnostic tail
/// ([`file_analysis_incremental`]) and the optimiser's compiler-checks pass
/// ([`compiler_check_diagnostics`]). Admitted baseline lattices and neutral
/// source syntax use tracked per-body queries; current input, entry state,
/// grammar and retained source owners independently govern reuse and restoration.
/// Unsupported source templates use the exact full-world lattice key or fresh
/// lowering. The resulting unit receives the interprocedural summary.
///
/// # Must be called from inside a tracked query
///
/// This is crate-private, and stays crate-private, because it interns the
/// per-body keys ([`FnLatticeKey`], [`ProcBodyKey`], [`TaintSummaryKey`],
/// [`SummaryDepsKey`], [`OptDepsKey`]) directly.  Salsa stamps an interned slot
/// with the minimum durability of the inputs the *active* query has read; with
/// no active query it stamps `Durability::MAX`, and only `Durability::LOW`
/// slots are ever garbage-collected.  An untracked call therefore mints
/// immortal slots, each pinning a full procedure IR plus its lattice memos —
/// a KB-per-keystroke leak if it ever landed on the edit path.
///
/// The sanctioned entry point is the tracked [`compilation_unit`] query, which
/// reads [`SourceFile`] (`LOW`) before building.  See the crate docs' "The
/// interned garbage collector is load-bearing".
#[must_use]
pub(crate) fn memoised_compilation_unit(
    db: &dyn TclDb,
    document: (Option<crate::SourceFile>, &str),
    options: UnitBuildOptions<'_>,
) -> CompilationUnit {
    build_unit_with_keys(db, document, options).0
}

/// Return the issued body's retained lattice key when its memo does not read
/// another procedure of the module. Module-reading lattices use the current
/// unit's result for checks, rewrites and the taint cascade.
pub(super) fn memo_key<'db>(
    db: &'db dyn TclDb,
    keys: &HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
    body_id: Option<tcl_compiler::ir::ProcedureId>,
) -> Option<FnLatticeKey<'db>> {
    keys.get(&body_id?)
        .copied()
        .filter(|&key| !function_lattice(db, key).sccp.reads_module)
}

/// Build the unit and retain its issued body-ID to [`FnLatticeKey`] mapping.
///
/// [`proc_taint_solve`] uses these keys for the interprocedural summary fixpoint
/// ([`proc_summary_cascade`]). A salsa tracked return must be `'static`, while
/// `FnLatticeKey<'db>` is interned for `'db`, so this mapping cannot leave the
/// shared [`compilation_unit`] query. The checks path builds its current unit
/// and requests the tracked memos independently; a hit requires the actual
/// query key and successful current-owner restoration.
pub(super) fn build_unit_with_keys<'db>(
    db: &'db dyn TclDb,
    document: (Option<crate::SourceFile>, &str),
    options: UnitBuildOptions<'_>,
) -> (
    CompilationUnit,
    HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
) {
    build_unit_with_keys_and_input(db, document, options, None, None)
}

fn lattice_memo_context<'db>(db: &'db dyn TclDb, req: &LatticeRequest<'_>) -> CfgContext<'db> {
    let mut upvar: Vec<(tcl_compiler::ir::ProcedureId, UpvarInfo)> = req
        .upvar_procs
        .iter()
        .map(|(id, value)| (*id, value.clone()))
        .collect();
    upvar.sort_by_key(|a| a.0);
    let mut proc_params: Vec<(tcl_compiler::ir::ProcedureId, Vec<String>)> = req
        .proc_params
        .iter()
        .map(|(id, value)| (*id, value.clone()))
        .collect();
    proc_params.sort_by_key(|a| a.0);
    let mut global_write: Vec<(tcl_compiler::ir::ProcedureId, GlobalWriteInfo)> = req
        .global_write_procs
        .iter()
        .map(|(id, value)| (*id, value.clone()))
        .collect();
    global_write.sort_by_key(|a| a.0);
    CfgContext::new(
        db,
        upvar,
        proc_params,
        global_write,
        BodyLatticeCommandBindings::from(req.command_bindings.clone()),
        req.procedure_summary_owners.clone(),
    )
}

type LatticeMemoState<'a, 'db> = (
    &'a mut Option<CfgContext<'db>>,
    &'a mut HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
    &'a mut HashMap<tcl_compiler::ir::ProcedureId, FunctionUnit>,
);

fn build_lattice_memo_candidate<'db>(
    db: &'db dyn TclDb,
    req: &LatticeRequest<'_>,
    registry: &CommandRegistry,
    registry_snapshot: &tcl_registry::RegistrySnapshot,
    epoch: u64,
    state: LatticeMemoState<'_, 'db>,
) -> FunctionUnit {
    let (context, lattice_keys, returned_candidates) = state;
    let context = *context.get_or_insert_with(|| lattice_memo_context(db, req));
    let template = BodyLatticeInputs::retain_original_worker_template(req, registry);
    if let Some(template) = template {
        let template_context = CfgContext::new(
            db,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            template.command_bindings(),
            OriginalProcedureSummaryOwners::default(),
        );
        let key = lattice_request_key(
            db,
            req,
            template.body().clone(),
            template_context,
            registry_snapshot,
            epoch,
        );
        let unit = template.rebind_cached(&function_lattice(db, key));
        if let Some(unit) = unit {
            lattice_keys.insert(req.body_id, key);
            returned_candidates.insert(req.body_id, unit.clone());
            return unit;
        }
    }
    if let Some(inputs) =
        tcl_compiler::body_analysis_capsule::BodyLatticeInputs::retain(req, registry)
        && let Some(bindings) = inputs.command_bindings()
    {
        let analytical_context = CfgContext::new(
            db,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            bindings,
            OriginalProcedureSummaryOwners::default(),
        );
        let key = lattice_request_key(
            db,
            req,
            inputs.clone(),
            analytical_context,
            registry_snapshot,
            epoch,
        );
        if let Some(unit) = inputs.rebind_cached(key.body(db), &function_lattice(db, key)) {
            lattice_keys.insert(req.body_id, key);
            returned_candidates.insert(req.body_id, unit.clone());
            return unit;
        }
    }
    // Exact full-world identity is the fallback when template admission or
    // restoration cannot establish equivalence. No partially relocated
    // artifact crosses this boundary.
    let key = lattice_request_key(
        db,
        req,
        BodyLatticeInputs::retain_original_worker_request(req, registry)
            .unwrap_or_else(|| BodyLatticeInputs::from(req.body.clone())),
        context,
        registry_snapshot,
        epoch,
    );
    lattice_keys.insert(req.body_id, key);
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_FUNCTION_LATTICE").is_some() {
        eprintln!(
            "ORIGINAL_FUNCTION_LATTICE qname={} event={} untouched={}",
            req.qname,
            req.irules_event_body.is_some(),
            req.command_trust.agrees_with_untouched_bindings()
        );
    }
    let unit = (*function_lattice(db, key)).clone();
    returned_candidates.insert(req.body_id, unit.clone());
    unit
}

type BodyMemos<'db> = (
    Box<dyn Fn(&str, &str) -> tcl_compiler::lowering::OriginalBodyLoweringTemplate + 'db>,
    Box<
        dyn Fn(
                &tcl_compiler::lowering::SourceBodySyntaxRequest,
            ) -> tcl_compiler::lowering::SourceBodySyntaxTemplate
            + 'db,
    >,
);

/// The per-body lowering memos of one dialect, registry and body grammar.
fn lowering_body_memos<'db>(
    db: &'db dyn TclDb,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    registry: &tcl_registry::RegistrySnapshot,
    body_config: tcl_lexer::LexerConfig,
) -> BodyMemos<'db> {
    // Values, rather than an environment-name round trip, own memo identity.
    let dialect_key = dialect.map_or("", |profile| profile.name);
    let profile_key = dialect.map(tcl_dialect::DialectProfile::cache_key);
    let registry = registry.clone();
    (
        Box::new(move |body_text: &str, namespace: &str| {
            let key = ProcBodyKey::new(
                db,
                body_text.to_owned(),
                namespace.to_owned(),
                dialect_key.to_owned(),
                CompilerMemoSnapshot {
                    profile: profile_key,
                    registry: registry.clone(),
                },
                body_config,
            );
            original_lower_proc_body(db, key)
        }),
        Box::new(
            move |request: &tcl_compiler::lowering::SourceBodySyntaxRequest| {
                let key = SourceProcBodyKey::new(db, request.clone());
                lower_proc_body_source(db, key)
            },
        ),
    )
}

pub(super) fn build_unit_with_keys_and_input<'db>(
    db: &'db dyn TclDb,
    (document, source): (Option<crate::SourceFile>, &str),
    options: UnitBuildOptions<'_>,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
    settings: Option<&Arc<tcl_compiler::analyser::OriginalAnalyserSourceSettings>>,
) -> (
    CompilationUnit,
    HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
) {
    let UnitBuildOptions { registry, .. } = options;
    let dialect = options.dialect;
    let dialect_opt = dialect;
    let registry_snapshot = registry.snapshot();
    let body_config = options.config.normalized();
    let epoch = evaluator_epoch(db);
    // The module CFG context is the same for every procedure in this build;
    // intern it once on the first request and reuse the id (O(procs), not
    // O(procs²)).
    let mut context: Option<CfgContext<'db>> = None;
    // Record each memoised procedure's interned `FnLatticeKey` so the
    // interprocedural taint pass below can demand the procedure's offset-0
    // baseline (`function_lattice`) again to layer the `taint_cascade` memo on
    // top — without re-deriving the offset-0 body/context.
    let mut lattice_keys: HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>> =
        HashMap::new();
    let mut returned_candidates: HashMap<tcl_compiler::ir::ProcedureId, FunctionUnit> =
        HashMap::new();
    let mut lattice_memo = |req: &LatticeRequest<'_>| -> FunctionUnit {
        build_lattice_memo_candidate(
            db,
            req,
            registry,
            &registry_snapshot,
            epoch,
            (&mut context, &mut lattice_keys, &mut returned_candidates),
        )
    };
    // Isolated-body and readonly source-syntax caches have separate request
    // owners. Each admitted hit must restore its positioned carriers against
    // the current input and entry state before the unit can retain it.
    let cu = {
        // The isolated-body caller positions this owned baseline Script and
        // restores current binding proofs. Source-only syntax uses its separate
        // genuine pre-lowering request below.
        let (body_memo, source_body_memo) =
            lowering_body_memos(db, dialect, &registry_snapshot, body_config);
        let module_memo = |inputs: &tcl_compiler::compilation_unit::ModuleLoweringInputs| {
            let file = document.filter(|file| file.text(db).as_str() == inputs.source())?;
            lowered_document_module(
                db,
                file,
                ModuleLoweringKey::new(db, inputs.without_source()),
            )
        };
        match input.zip(settings) {
            Some(input) => {
                CompilationUnit::build_memoized_with_original_body_lowering_and_source_settings(
                    source,
                    options,
                    &mut lattice_memo,
                    (&body_memo, &source_body_memo, Some(&module_memo)),
                    None,
                    input,
                )
            }
            None => CompilationUnit::build_memoized_with_original_body_lowering(
                source,
                options,
                &mut lattice_memo,
                (&body_memo, &source_body_memo, Some(&module_memo)),
                None,
                input,
            ),
        }
    };
    // Validate the candidate returned after current source restoration, rather
    // than its cached donor. Rejected callbacks cannot seed downstream memos.
    lattice_keys.retain(|id, _| {
        returned_candidates.get(id).is_some_and(|candidate| {
            cu.accepts_original_procedure_lattice(*id, candidate, registry)
        })
    });
    drop(returned_candidates);
    // The private summary producer owns this request. Memoisation retains the
    // actual body report, not a publicly cloned CU with a substituted summary.
    let unit = cu.with_original_interprocedural_memoized(
        registry,
        dialect_opt,
        &mut |request: &tcl_compiler::compilation_unit::OriginalInterproceduralTaintRequest,
              body_id: tcl_compiler::ir::ProcedureId,
              _display: &str| {
            memo_key(db, &lattice_keys, Some(body_id))?;
            let capsule = request.body_capsule(body_id, registry)?;
            let original_context =
                capsule.original_optimiser_analysis_context(registry, dialect_opt);
            let key = OriginalTaintSummaryKey::new(
                db,
                capsule,
                dialect_opt.map(tcl_dialect::DialectProfile::cache_key),
                original_context,
            );
            Some((*original_taint_cascade(db, key)).clone())
        },
    );
    (unit, lattice_keys)
}

pub(super) fn lattice_request_key<'db>(
    db: &'db dyn TclDb,
    req: &LatticeRequest<'_>,
    body: tcl_compiler::body_analysis_capsule::BodyLatticeInputs,
    context: CfgContext<'db>,
    registry: &tcl_registry::RegistrySnapshot,
    epoch: u64,
) -> FnLatticeKey<'db> {
    let certified_readonly_source = body.certified_readonly_source_body_id().is_some();
    FnLatticeKey::new(
        db,
        body,
        req.qname.to_owned(),
        req.params.to_vec(),
        context,
        req.lexer_config,
        req.dialect.map_or("", |profile| profile.name).to_owned(),
        CompilerMemoSnapshot {
            profile: req.dialect.map(tcl_dialect::DialectProfile::cache_key),
            registry: registry.clone(),
        },
        req.param_constants.to_vec(),
        FnLatticeModuleFacts {
            known_classes: req.known_classes.to_vec(),
            traced_variables: req.traced_variables.to_vec(),
            has_dynamic_variable_trace: req.has_dynamic_variable_trace,
        },
        FnLatticeEntry {
            original_declaration_body: (!certified_readonly_source)
                .then(|| {
                    req.original_source_procedure()
                        .filter(|procedure| {
                            procedure.body_kind
                                == tcl_compiler::ir::ProcedureBodyKind::OriginalDeclaration
                        })
                        .map(|_| req.body_id)
                })
                .flatten(),
            plain_command_dispatch: req.plain_command_dispatch,
            metadata_context: req.original_metadata_context(registry.registry()),
            source_metadata_input: req.source_metadata_input.cloned(),
            irules_event_body: req.irules_event_body.cloned(),
            command_trust: req.command_trust.snapshot(),
        },
        ValueTransferContext::of(db, req.analysis_context.clone(), epoch),
    )
}
