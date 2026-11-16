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

//! The memoised per-procedure baseline lattice and the interned keys that identify it.

use super::TclDb;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::cfg_builder::{
    OriginalProcedureSummaryOwners, PreparedCfgContext, build_cfg_function_with_prepared_context,
};
use tcl_compiler::compilation_unit::{FunctionUnit, ModuleTraceFacts};
use tcl_registry::CommandRegistry;

/// Interned module-wide context (`upvar_procs` + `proc_params` +
/// `global_write_procs` from `prepare_cfg_context`) that a procedure body's
/// CFG is built under. Interned once per build and shared by every
/// [`FnLatticeKey`] so a procedure's key stays small and the per-build
/// interning cost is `O(procs)`, not `O(procs²)`. The entry vecs are sorted
/// by actual body coordinate before interning so an equal context (regardless of hash-map
/// iteration order) yields the same id.
#[salsa::interned]
pub struct CfgContext<'db> {
    #[returns(ref)]
    pub upvar_ctx: Vec<(tcl_compiler::ir::ProcedureId, UpvarInfo)>,
    #[returns(ref)]
    pub proc_params: Vec<(tcl_compiler::ir::ProcedureId, Vec<String>)>,
    #[returns(ref)]
    pub global_write_ctx: Vec<(tcl_compiler::ir::ProcedureId, GlobalWriteInfo)>,
    #[returns(ref)]
    pub command_bindings: tcl_compiler::body_analysis_capsule::BodyLatticeCommandBindings,
    /// Genuine Module/source owner of every procedure-effect coordinate.
    #[returns(ref)]
    pub procedure_summary_owners: OriginalProcedureSummaryOwners,
}

/// Exact selected compiler inputs shared by lattice and body memo identities.
/// Metadata labels are separate; consumers read this retained registry and
/// reconstruct policy from its full value snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompilerMemoSnapshot {
    pub profile: Option<tcl_dialect::DialectProfileKey>,
    pub registry: tcl_registry::RegistrySnapshot,
}

/// Interned identity of one procedure's **offset-0** baseline lattice
/// (salsa-native lattice graph).  Holds the procedure's post-inline IR body
/// normalised to offset 0 plus the CFG-determining module [`CfgContext`] +
/// params + dialect + command-dispatch mode — *not* its position — so a shifted-but-unchanged body
/// interns to the same key and reuses the cached [`function_lattice`] (the
/// builder rebases the result to the body's span).  Procedures with
/// interprocedural `param_constants` (caller-uniform-literal SCCP seeds) are
/// memoised too: the encoded seeds are part of the key, so such a procedure
/// reuses its cached lattice across edits and rebuilds only when a caller's
/// literal at that position changes (which re-interns to a new key).  The seeds
/// are position-independent (keyed by parameter name + SSA version), so they do
/// not break the offset-invariance of the body key.
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `body` is the procedure's whole post-inline IR, so an edit inside the body
/// mints a fresh id *and* a fresh `Script` behind it.  The collector's slot
/// reuse is what frees both, and it only reclaims `Durability::LOW` slots
/// interned inside a tracked query.  See the crate docs' "The interned garbage
/// collector is load-bearing"; pinned by `tests/interned_gc.rs`.
/// Conditional entry axes retained together in each function lattice key.
/// The descriptor is sealed by actual lowering; dispatch mode grants no event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnLatticeEntry {
    /// Actual original declaration-body role, independently of a callable entry.
    pub original_declaration_body: Option<tcl_compiler::ir::ProcedureId>,
    /// Trace-visible mode preserves ordinary dynamically replaceable dispatch.
    pub plain_command_dispatch: bool,
    /// Original availability purpose; explicit standalone and missing supplied
    /// metadata are distinct even when both lack a complete source input.
    pub metadata_context: tcl_compiler::registry_invocation::OwnedInvocationMetadataContext,
    /// Exact supplied source availability and command generation. Missing input
    /// declines metadata rather than rebuilding a profile-default context.
    pub source_metadata_input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
    /// Full original event producer, including source/configuration and Registry.
    pub irules_event_body: Option<Arc<tcl_compiler::ir::SourceIrulesEventBody>>,
    /// Full module mutation obligations; no untouched-world substitution.
    pub command_trust: tcl_compiler::command_binding::CommandTrustSnapshot,
}

/// The value-transfer analysis context of one module, interned once per
/// distinct value so a per-procedure [`FnLatticeKey`] carries an id rather
/// than the context's fields: every procedure of a module shares it, an edit
/// that changes no binding evidence re-interns nothing, and the per-revision
/// key stays as small as before the context joined it (the memory-growth
/// plateau `tests/memory_growth.rs` pins).
#[salsa::interned]
pub struct ValueTransferContext<'db> {
    /// The context's hashable identity, as the compiler computes it per
    /// module.
    #[returns(ref)]
    pub key: tcl_compiler::value_transfer::AnalysisContextKey,
    /// The command trust the key's snapshot stands for, rebuilt once per
    /// distinct context rather than once per procedure: the shared lattice
    /// folds every route under it with the `ObservedBindings` stance, as a
    /// whole-module build does, so the memoised lattice and a fresh one
    /// answer every head alike.
    #[returns(ref)]
    pub mutations: tcl_compiler::command_binding::ModuleCommandMutations,
    /// The [`EvaluatorEpoch`] the context was built under, so a new epoch —
    /// a plan reload, a quarantine — is a new context and every memoised
    /// lattice under the old one is re-keyed.
    #[returns(copy)]
    pub evaluator_epoch: u64,
}

impl<'db> ValueTransferContext<'db> {
    /// The interned context for `key` under evaluator epoch `epoch`, with the
    /// command trust rebuilt from its snapshot.
    pub fn of(
        db: &'db dyn TclDb,
        key: tcl_compiler::value_transfer::AnalysisContextKey,
        epoch: u64,
    ) -> Self {
        let mutations = key.bindings.to_mutations();
        Self::new(db, key, mutations, epoch)
    }
}

/// Whole-module facts shared by each function lattice. Each original field
/// participates structurally in equality and hashing of the interned key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnLatticeModuleFacts {
    /// Fully-qualified names of every class in the compilation unit (sorted) —
    /// a whole-unit fact identical for every procedure, folded into the key so
    /// adding/removing a class anywhere invalidates each procedure's lattice (a
    /// new class can change a body's constructor typing).  Threaded into the
    /// type-propagation pass in [`function_lattice`].
    pub known_classes: Vec<String>,
    /// Literal variable-trace target names (sorted) from
    /// [`tcl_compiler::ir::Module::traced_variables`] — a whole-module fact
    /// identical for every procedure, folded into the key exactly like
    /// `known_classes`: SCCP's trace-safety gate (see
    /// [`tcl_compiler::sccp::sccp`]) treats a name in this set as never a
    /// compile-time constant, so a trace installed anywhere in the module can
    /// change any procedure's cached lattice.
    pub traced_variables: Vec<String>,
    /// [`tcl_compiler::ir::Module::has_dynamic_variable_trace`] — `true` when
    /// a variable-trace install/remove call targets a non-literal name
    /// anywhere in the module. Folded into the key alongside
    /// `traced_variables`.
    pub has_dynamic_variable_trace: bool,
}

#[salsa::interned]
pub struct FnLatticeKey<'db> {
    #[returns(ref)]
    pub body: tcl_compiler::body_analysis_capsule::BodyLatticeInputs,
    #[returns(ref)]
    pub qname: String,
    #[returns(ref)]
    pub params: Vec<String>,
    #[returns(copy)]
    pub context: CfgContext<'db>,
    /// Exact offset-zero lexer configuration that lowered the procedure body.
    /// Kept in the identity separately from the dialect because document and
    /// test hosts may apply grammar overrides to one registry profile.
    #[returns(copy)]
    pub lexer_config: tcl_lexer::LexerConfig,
    #[returns(ref)]
    pub dialect: String,
    /// Full profile and actual authored command surface at ingress.
    #[returns(ref)]
    pub snapshot: CompilerMemoSnapshot,
    /// Encoded interprocedural SCCP seeds (`(param, version, string)`, sorted);
    /// empty means none.  Decoded by
    /// [`tcl_compiler::compilation_unit::decode_param_constants`] in [`function_lattice`].
    #[returns(ref)]
    pub param_constants: Vec<(String, u32, String)>,
    /// Exact whole-module class and variable-trace facts.
    #[returns(ref)]
    pub module_facts: FnLatticeModuleFacts,
    /// Exact dispatch mode and sealed source entry, compared structurally.
    #[returns(ref)]
    pub entry: FnLatticeEntry,
    /// Actual module value-transfer context and evaluator revision.
    #[returns(copy)]
    pub analysis_context: ValueTransferContext<'db>,
}

/// Memoised offset-0 baseline lattice (CFG → SSA → def-use → SCCP → type →
/// rendered → intra-procedural taint) for one procedure, built from its interned
/// offset-0 body + context.  A body-only edit changes only that procedure's
/// `FnLatticeKey`, so salsa reuses every other procedure's lattice; a shifted
/// body interns to the same key (cache hit). Event bodies additionally retain
/// their full original event descriptor and source; a changed producer gets a
/// different key even when executable body bytes agree. Exact supplied source
/// availability also participates in identity, independently of the profile.
/// Rebuilds the CFG via the same `build_cfg_function_with_prepared_context`
/// call `build_cfg` makes per procedure, so the
/// result equals the whole-module build's unit (modulo offset).  SCCP is seeded
/// with the key's interprocedural `param_constants` (caller-uniform-literal
/// folds), decoded back to the seed map `build_for_inner` would pass on the
/// fresh path — so a procedure with such seeds memoises instead of bypassing the
/// cache, and rebuilds only when a caller's literal at that position changes (a
/// new key).  The interprocedural taint re-run still happens at aggregation time
/// (`with_interprocedural`). The lowering request retains its exact command
/// store and supplied source availability; missing or foreign input declines
/// conditional CFG metadata.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn function_lattice<'db>(db: &'db dyn TclDb, key: FnLatticeKey<'db>) -> Arc<FunctionUnit> {
    let context = key.context(db);
    let upvar: HashMap<tcl_compiler::ir::ProcedureId, UpvarInfo> =
        context.upvar_ctx(db).iter().cloned().collect();
    let proc_params: HashMap<tcl_compiler::ir::ProcedureId, Vec<String>> =
        context.proc_params(db).iter().cloned().collect();
    let global_write_procs: HashMap<tcl_compiler::ir::ProcedureId, GlobalWriteInfo> =
        context.global_write_ctx(db).iter().cloned().collect();
    let registry = key.snapshot(db).registry.registry();
    // The request's exact grammar, not one reconstructed from the registry or
    // environment name: grammar overrides are part of the memo identity.
    let config = key.lexer_config(db);
    let original_inputs = tcl_compiler::compilation_unit::OriginalFunctionLatticeInputs {
        qname: key.qname(db),
        params: key.params(db),
        upvar_procs: &upvar,
        proc_params: &proc_params,
        global_write_procs: &global_write_procs,
        command_bindings: context.command_bindings(db),
        procedure_summary_owners: context.procedure_summary_owners(db),
        lexer_config: config,
        profile: key.snapshot(db).profile,
        dialect: key.dialect(db),
        param_constants: key.param_constants(db),
        known_classes: &key.module_facts(db).known_classes,
        traced_variables: &key.module_facts(db).traced_variables,
        has_dynamic_variable_trace: key.module_facts(db).has_dynamic_variable_trace,
        original_declaration_body: key.entry(db).original_declaration_body,
        plain_command_dispatch: key.entry(db).plain_command_dispatch,
        metadata_context: &key.entry(db).metadata_context,
        source_metadata_input: key.entry(db).source_metadata_input.as_ref(),
        event_body: key.entry(db).irules_event_body.as_ref(),
        command_trust: &key.entry(db).command_trust,
        analysis_context: key.analysis_context(db).key(db),
    };
    if let Some(unit) = key
        .body(db)
        .original_function_lattice(&original_inputs, registry)
    {
        return Arc::new(unit);
    }
    function_lattice_fallback(
        db,
        key,
        context,
        registry,
        config,
        (upvar, proc_params, global_write_procs),
    )
}

type LatticeFallbackMaps = (
    HashMap<tcl_compiler::ir::ProcedureId, UpvarInfo>,
    HashMap<tcl_compiler::ir::ProcedureId, Vec<String>>,
    HashMap<tcl_compiler::ir::ProcedureId, GlobalWriteInfo>,
);

fn function_lattice_fallback<'db>(
    db: &'db dyn TclDb,
    key: FnLatticeKey<'db>,
    context: CfgContext<'db>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    maps: LatticeFallbackMaps,
) -> Arc<FunctionUnit> {
    let (upvar, proc_params, global_write_procs) = maps;
    let prepared = PreparedCfgContext::from_metadata_input(
        (
            upvar,
            proc_params,
            global_write_procs,
            context.command_bindings(db).bindings().clone(),
            context.procedure_summary_owners(db).clone(),
        ),
        registry,
        tcl_compiler::registry_invocation::InvocationMetadataInput::Retained(
            &key.entry(db).metadata_context,
        ),
    );
    let cfg = build_cfg_function_with_prepared_context(
        key.qname(db),
        key.body(db).body(),
        true,
        registry,
        key.entry(db).plain_command_dispatch,
        &prepared,
        config,
    );
    let param_constants =
        tcl_compiler::compilation_unit::decode_param_constants(key.param_constants(db));
    let known_classes: HashSet<String> =
        key.module_facts(db).known_classes.iter().cloned().collect();
    let traced_variables: BTreeSet<String> = key
        .module_facts(db)
        .traced_variables
        .iter()
        .cloned()
        .collect();
    let command_trust = key.entry(db).command_trust.to_mutations();
    let trace_facts = ModuleTraceFacts {
        traced_variables: &traced_variables,
        has_dynamic_variable_trace: key.module_facts(db).has_dynamic_variable_trace,
        deferred_writes: &key.analysis_context(db).key(db).deferred_writes,
    };
    let inputs = tcl_compiler::compilation_unit::FunctionLatticeInputs {
        dialect: tcl_compiler::compilation_unit::UnitDialect {
            registry,
            config,
            source_metadata_input: key.entry(db).source_metadata_input.as_ref(),
        },
        param_constants: param_constants.as_ref(),
        known_classes: &known_classes,
        trace_facts,
        analysis_context: key.analysis_context(db).key(db),
        command_trust: &command_trust,
        event_body: key
            .entry(db)
            .irules_event_body
            .as_ref()
            .map(|event| (event, key.body(db).body())),
    };
    let unit = if let Some(body_id) = key
        .entry(db)
        .original_declaration_body
        .or_else(|| key.body(db).certified_readonly_source_body_id())
    {
        FunctionUnit::build_for_original_declaration_lattice(
            body_id,
            key.qname(db),
            cfg,
            key.params(db),
            inputs,
        )
    } else {
        FunctionUnit::build_for_lattice(key.qname(db), cfg, key.params(db), inputs)
    };
    Arc::new(unit.with_retained_semantic_analysis(
        registry,
        Some(key.body(db).body()),
        // A procedure body runs only after arbitrary interposed history,
        // so its dispatch proofs start from an unknown world.
        tcl_compiler::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
    ))
}
