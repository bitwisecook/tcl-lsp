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

//! Salsa incremental query database for the Tcl LSP.
//!
//! A single memoised query graph replaces the server's
//! hand-maintained caches.  Inputs ([`SourceFile`], [`AnalyserConfig`]) feed
//! tracked queries that wrap the existing sync pure functions in
//! `tcl-compiler` / `tcl-lsp-core`; salsa owns memoisation and
//! dependency-tracked invalidation, so there is no manual cache eviction.
//!
//! Priorities, in order: correctness (queries are pure deterministic
//! functions; behaviour matches a from-scratch recompute), `O()` complexity
//! (incremental reuse), then memory (share via `Arc`, not deep clones).
//!
//! The command registry is *static* (built once, never mutated), so it is
//! carried as a durable field on the database and read via [`TclDb::registry`]
//! rather than modelled as a salsa input — reading an immutable value inside a
//! tracked query is sound and avoids requiring `CommandRegistry: PartialEq`.
//!
//! # Return modes (`returns(clone)` / `returns(copy)`)
//!
//! Salsa 0.28 returns *references into the memo table* by default, so a query
//! declared `-> Arc<T>` hands back `&'db Arc<T>` unless told otherwise.  Every
//! query here opts out, for two stacked reasons:
//!
//! - **Ownership.** The server runs each read on a **cloned, short-lived
//!   snapshot** inside `salsa::Cancelled::catch` on a blocking worker, and the
//!   result is moved out of that closure — past the point where the snapshot is
//!   borrowable.
//! - **Liveness, which is the binding one.** `returns(ref)` does not merely
//!   ask the caller to keep the borrow inside the closure; it forces every use
//!   of the value to happen *there*, inside the read.  That is exactly what
//!   this server must not do.  `set_text` takes salsa's global write
//!   exclusivity **while holding the server's db mutex**, so a read handle held
//!   across uncancellable work blocks the next edit's write — and, through that
//!   mutex, every other request, including ones that touch no document at all.
//!   See `docs/design/rust/lsp-performance.md`.  Work done
//!   inside the read must therefore stay minimal and cancellable; converting a
//!   result into its `lsp-types` wire shape is neither.
//!
//! So the split is by cost, not by borrow:
//!
//! - **`returns(clone)`** everywhere the value is a payload.  For the crate's
//!   usual `Arc<T>` shape this is the refcount bump the "share via `Arc`"
//!   priority above already assumed.  For the handful of bare-value queries
//!   ([`document_symbols`], [`folding_ranges`], [`semantic_tokens`],
//!   [`semantic_tokens_project`]) it is one deep copy, taken deliberately: it
//!   buys the caller the freedom to project *outside* the read region, and a
//!   copy is much cheaper than a stalled keystroke.
//! - **`returns(copy)` — scalar getters** on the input/interned structs
//!   (`bool` / `u64` / `NonAsciiMode` / a nested interned handle).  A `&bool`
//!   getter is strictly worse; the copy is free.
//!
//! Putting [`document_symbols`] and [`folding_ranges`] on `returns(ref)` and
//! moving the `lsp-types` projection into the worker closure to contain the
//! borrow would save a copy and cost liveness: the projection is a pure
//! allocation walk with no salsa cancellation checkpoint, so it would extend
//! the uncancellable tail of a read that `set_text` has to wait behind. That
//! shape reproduces as the VS Code suite wedging on a `didOpen` drain with
//! even document-free requests unanswered.
//!
//! # Deep-memo eviction (`lru = N`)
//!
//! Salsa has no input-deletion API and re-setting an input frees nothing:
//! an invalidated memo keeps its old value until the query re-executes and
//! *replaces* it, and setting identical text backdates and changes nothing at
//! all.  So a session that opens 150 files and closes them again retains
//! every deep memo those files accrued while open — measured at ~930 MB of
//! unreclaimed RSS across exactly that cycle.  Dropping the closed files'
//! `SourceFile` handles does not help;
//! the memos are keyed on the salsa ids, which stay in the tables.
//!
//! `#[salsa::tracked(lru = N)]` is the one mechanism that releases a memo's
//! payload: at each revision boundary the least-recently-used ids have
//! `memo.value` cleared while their dependency information is kept, so an
//! evicted query is simply recomputed on demand.  It is sound by construction
//! — salsa refuses to evict anything that is not `Derived` — and this crate
//! has no untracked reads (`registry()` is a process-wide immutable cache, and
//! the one mutable cache a query reads, the workspace's pack overlays, is read
//! behind the [`OverlayEpoch`] input), so every deep query qualifies.
//!
//! Two caps, sized by what the key counts:
//!
//! * **512** on the per-*item* queries (`item_body_analysis`,
//!   `function_lattice`, `lower_proc_body`, `taint_cascade`,
//!   `proc_summary_cascade`, `function_checks`, `proc_taint_solve`,
//!   `function_optimisations`, `compilation_unit`).  Their keys are interned
//!   per procedure body, so the live set is roughly *procedures per file ×
//!   open files*; 512 covers a realistic working set (say twenty open
//!   documents of twenty-five procedures) without holding a browsed-and-closed
//!   file's bodies for the session.
//! * **64** on the per-*file* queries (`file_analysis_incremental`,
//!   `compiler_check_diagnostics`, `document_compilation_unit`) — one entry
//!   per open document, and no reader walks them project-wide (the whole-file
//!   aggregates read the light `file_token_facts` / `item_sigs` tiers
//!   instead), so the cap cannot thrash on a large workspace.
//!
//! The light signature/structure tiers (`file_decls`, `item_sigs`,
//! `file_token_facts`, the `project_*` aggregates) are deliberately **not**
//! capped: they are small, they are what an unopened workspace file is meant
//! to be resolvable from, and evicting them would make the cross-file
//! resolution every open document depends on recompute from scratch.
//!
//! This composes correctly only because closed files never create deep memos
//! at all: the closed-file republish takes the uncached pipeline, so no
//! closed-file sweep can `record_use` on hundreds of closed files and evict
//! the *open* documents' units instead.
//!
//! # The interned garbage collector is load-bearing
//!
//! `lru = N` above bounds *memo payloads*.  It does **not** bound the
//! **interned tables**, and six of this crate's interned structs key on
//! content that changes on every keystroke inside a procedure body:
//!
//! | Interned struct | The per-revision field |
//! |---|---|
//! | [`ItemBodyKey`] | `body_text` — the procedure/method body source |
//! | [`FnLatticeKey`] | `body` — the whole post-inline procedure IR |
//! | [`ProcBodyKey`] | `body_text` |
//! | [`OptDepsKey`] | current compilation capsule |
//! | [`TaintSummaryKey`] | current compilation capsule |
//! | [`SummaryDepsKey`] | current compilation capsule + actual-ID summary inputs |
//!
//! Typing inside a body therefore mints a brand-new interned id for every one
//! of them on every keystroke, each holding a body's worth of `Arc` payload in
//! its memo table — the exact shape of an unbounded interning leak.  Measured
//! against a typing-session corpus the edit path is nonetheless flat (~0 bytes/edit
//! steady state, ~66.5 MB across 60–500 edits), and the reason is **salsa's
//! interned garbage collector**, not anything this crate does:
//!
//! * A cold intern walks the shard's LRU tail and reuses any slot that has not
//!   been interned for `DEFAULT_REVISIONS` (3) revisions: it overwrites the
//!   slot's fields and calls `clear_memos` on it.  That `clear_memos` is what
//!   drops the retained `Arc`s — so the collector, not `lru = N`, is what keeps
//!   a typing session bounded.
//! * A slot is eligible only when its recorded durability is exactly
//!   `Durability::LOW`.  Collecting anything more durable would require
//!   invalidating that durability's revision (`Database::synthetic_write`,
//!   which needs `&mut` on the database), so salsa refuses outright.
//! * A slot's durability is the *minimum* durability of the inputs the creating
//!   query had read when it interned — and with **no active query at all**
//!   salsa stamps `Durability::MAX` / `Revision::MAX`, i.e. an immortal slot.
//!
//! Two rules follow, and breaking either restores a KB-per-keystroke leak
//! while looking like an optimisation:
//!
//! 1. **[`SourceFile`], [`AnalyserConfig`], [`Project`], and
//!    [`EvaluatorEpoch`] stay at `Durability::LOW`** — salsa's default,
//!    for production inputs. Query durability is the minimum of its input
//!    reads; collection becomes unavailable when no `LOW` read remains.
//! 2. **The per-body keys are interned from inside a tracked query.**  That is
//!    why `memoised_compilation_unit` is crate-private and documented as
//!    tracked-query-only: reached from [`compilation_unit`] its interning
//!    inherits that query's `LOW` durability, whereas a direct call from
//!    untracked code mints immortal slots holding whole `CompilationUnit`
//!    lattices.
//!
//! [`LexerCfgKey`], [`OriginalCommandSlot`] and [`LogicalSourceCommandSlot`]
//! have different churn boundaries from these per-body keys. Production lexer
//! keys retain the resolved environment id; command keys retain exact original
//! source coordinates under their independently selected naming purpose.
//! New environments or symbols can add entries, but body text and revision
//! offsets do not enter these keys. They are not covered by the six-key growth
//! control below; [`compilation_unit`]'s signature takes a lexer key interned
//! by its caller.
//! [`CfgContext`] is content-keyed but signature-level, so a body edit leaves
//! it alone; it is collected by the same mechanism when a signature does move.
//!
//! `tests/interned_gc.rs` compares live slots after 24 and 96 edits through
//! the production queries. Each of the six ingredients must have a nonempty
//! short-session slot set and at most twice as many slots after the longer
//! session. Reuse counts (see [`TclDatabase::with_interned_reuse_logger`]) are
//! diagnostics: a bounded working set spread across many shards can have no
//! reuse events. The control sets [`SourceFile`], [`AnalyserConfig`] and
//! [`EvaluatorEpoch`] to `Durability::HIGH`, requires the four body-keyed
//! ingredients to exceed the same growth bound, and requires zero reuse for
//! all six. This guards against a corpus that cannot detect the leak.
//! A source-level test rejects production `with_durability` / input-builder
//! `durability` calls in this crate and `tcl-lsp-server`, because setter calls
//! outside the database's query graph can escape the behavioural controls.
//! The full contract is written up in
//! `docs/design/rust/salsa-interned-gc.md`.

// Retained analysis receipts require deeper auto-trait evaluation.
#![recursion_limit = "256"]
// Long linear fail-closed guard chains and scenario tests; splitting them is
// tracked as follow-up debt (Luna-41 notes) rather than done mid-merge.

#[cfg(test)]
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[cfg(test)]
use tcl_compiler::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
#[cfg(test)]
use tcl_compiler::cfg_builder::OriginalProcedureSummaryOwners;
#[cfg(test)]
use tcl_compiler::command_binding::ModuleCommandBindings;
use tcl_compiler::compilation_unit::CompilationUnit;
#[cfg(test)]
use tcl_compiler::compilation_unit::{LatticeRequest, UnitBuildOptions};
#[cfg(test)]
use tcl_compiler::compiler_checks::DiagCode;
use tcl_compiler::compiler_checks::Diagnostic as CompilerCheck;
#[cfg(test)]
use tcl_compiler::interprocedural::ProcSummary;
#[cfg(test)]
use tcl_compiler::ir::Script;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::unit_scope::CallSiteEvidence;
// The compiler's per-proc return-taint summary (the colour-aware transfer
// function the interprocedural fixpoint converges) — aliased to avoid clashing
// with this crate's `ProcTaintSummary` (the *interproc-analysis* projection in
// `TaintSummaryKey`).  Used to memoise the summary fixpoint per procedure.
use tcl_compiler::taint_interproc::InterprocTaintResult;
#[cfg(test)]
use tcl_compiler::taint_interproc::ProcTaintSummary as ReturnTaintSummary;

#[cfg(test)]
use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::{
    AnalysisResult, ClassHierarchy, FileDecls, ItemSig, ItemTree, NonAsciiMode,
};
use tcl_lsp_core::semantic_tokens::{SemanticTokens, VarNameArgRoles};
use tcl_registry::CommandRegistry;
use tcl_registry::model::{DeclaredSurface, KeyedVersions, OverlayMiss};

mod arity_diagnostics;
mod check_solve;
mod command_signatures;
mod compilation_units;
mod compiler_diagnostics;
mod declarations;
mod declared_commands;
mod display_queries;
mod document_analysis;
mod function_lattices;
mod memoised_units;
mod optimisation_solve;
mod project_calls;

use arity_diagnostics::project_callback_diagnostics_with_mode;
#[cfg(test)]
use arity_diagnostics::{apply_original_callback_arity, project_source_diagnostics_for_analysis};
pub use arity_diagnostics::{
    project_callback_diagnostics, project_callback_diagnostics_for_analysis, project_diagnostics,
    project_diagnostics_for_inputs,
};
#[cfg(test)]
use check_solve::summary_deps_key;
pub use check_solve::{
    OriginalTaintSummaryKey, SummaryDepsKey, TaintSummaryKey, function_checks,
    original_taint_cascade, proc_summary_cascade, proc_taint_solve, proc_taint_solve_for_config,
    taint_cascade,
};
#[cfg(test)]
use command_signatures::{LogicalSourceCommandSlot, logical_source_command_arity};
pub use command_signatures::{
    OriginalCommandSlot, original_command_arity, original_command_signatures,
    original_lookup_command_arities, original_lookup_command_signatures,
    project_original_command_arities, project_original_command_signatures,
};
use command_signatures::{
    merge_original_command_signatures, original_lookup_command_signatures_using,
    select_original_command_signatures,
};
pub use compilation_units::{
    LexerCfgKey, compilation_unit, compilation_unit_for_config, document_compilation_unit,
    document_compilation_unit_for,
};
#[cfg(test)]
use compilation_units::{lexer_cfg_key, supplied_unit_build_options, unit_build_options};
use compiler_diagnostics::same_optimisation_provenance;
pub use compiler_diagnostics::{
    CompilerDiagnostics, compilation_unit_uncached_from_analysis, compiler_check_diagnostics,
    compiler_check_diagnostics_if_available, compiler_check_diagnostics_uncached,
    compiler_check_diagnostics_uncached_from_analysis,
};
#[cfg(test)]
use compiler_diagnostics::{abstain_diagnostics, fresh_compiler_document_unit};
use declarations::merge_class_factories;
pub use declarations::{
    FileOriginalProcedureFacts, file_class_factories, file_decls, file_original_procedure_facts,
    item_sigs, item_tree, project_class_factories, project_original_procedure_facts,
    project_proc_names,
};
pub use declared_commands::{
    declared_command_surface, merged_workspace_procedure_declarations,
    world_preserving_procedure_declarations, world_preserving_procedure_stubs,
};
pub use display_queries::{
    FileTokenFacts, document_symbols, file_token_facts, folding_ranges, project_class_index,
    project_named_instance_index, project_proc_var_index, semantic_tokens, semantic_tokens_project,
};
use display_queries::{
    merge_project_classes, merge_project_named_instances, token_facts_from_analysis,
};
pub(crate) use document_analysis::walked_document_input;
pub use document_analysis::{
    ItemBodyKey, document_analysis_input, file_analysis, file_analysis_incremental,
    item_body_analysis,
};
use document_analysis::{document_analyser, document_analyser_without_factory_oracle};
pub use function_lattices::{
    CfgContext, CompilerMemoSnapshot, FnLatticeEntry, FnLatticeKey, FnLatticeModuleFacts,
    ValueTransferContext, function_lattice,
};
#[cfg(test)]
use memoised_units::lattice_request_key;
pub use memoised_units::{
    ProcBodyKey, RetainedModuleTable, RetainedModules, SourceProcBodyKey, lower_proc_body,
    lower_proc_body_source, original_lower_proc_body,
};
#[cfg(test)]
use memoised_units::{build_unit_with_keys, build_unit_with_keys_and_input};
#[cfg(test)]
pub use optimisation_solve::{ConstReturnKey, OptCalleeSummary};
pub use optimisation_solve::{OptDepsKey, function_optimisations};
#[cfg(test)]
use optimisation_solve::{module_has_trace_facts, opt_callee_from_summary, opt_callee_to_summary};
pub use project_calls::{
    DispatchComponents, FileSourceTarget, file_call_site_evidence, file_dispatch_reach,
    file_external_call_sites, file_link_targets, file_source_targets, project_call_site_evidence,
    project_dispatch_components,
};
use project_calls::{conditional_source_links, merge_dispatch_components};

/// Database trait exposing the durable (non-salsa) command registry to
/// tracked queries.
#[salsa::db]
pub trait TclDb: salsa::Database {
    /// The dialect-loaded command registry (built once per canonical dialect
    /// key, then shared).  Immutable for the process lifetime.
    fn registry(&self, dialect: &str) -> &'static CommandRegistry;

    /// An owning registry handle for a workspace pack generation already
    /// installed by the server. Only `tcl-spectcl` can construct the
    /// overlay's contents, so a generation nothing installed is an
    /// [`OverlayMiss`], not the plain profile under the pack's key: what a
    /// query does without the packs is that query's decision — the queries
    /// that build a unit decline ([`compilation_unit`]), and the ones that
    /// only advise read the plain registry and say so.
    ///
    /// # Errors
    ///
    /// [`OverlayMiss`] when `overlay` is non-zero and no pack-carrying
    /// registry has been installed under it for `dialect`.
    fn registry_with_overlay(
        &self,
        dialect: &str,
        overlay: u64,
    ) -> Result<Arc<CommandRegistry>, OverlayMiss>;
}

/// The Tcl LSP query database.
///
/// Cloneable so a worker thread can run queries against a handle while the
/// main thread sets inputs (the rust-analyzer snapshot pattern).  The command
/// registry it hands out is the process-wide per-profile cache in
/// `tcl-registry`, not per-snapshot state.
#[salsa::db]
#[derive(Clone)]
pub struct TclDatabase {
    storage: salsa::Storage<Self>,
}

/// How many resolved overlay generations the process's queries hold.
const HELD_OVERLAYS: usize = 64;

/// The overlay generations the process's queries have resolved, and the
/// misses they have abstained on.
///
/// The process-wide registry cache retires an overlay generation once it
/// indexes too many, and a query can still be running — or being re-verified
/// — at a key it retired. A per-procedure query that looked the generation up
/// again would find it gone and disagree with the unit that keyed it, so the
/// queries hold what they have resolved and answer from that first. A
/// generation is content-addressed by its key, so serving one the cache no
/// longer indexes is never stale. It is process-wide because the cache it
/// backs up is, and every database — the server's snapshots, a test's own —
/// resolves the same key to the same generation.
///
/// It also keeps each distinct miss once, for the host to report
/// ([`take_overlay_misses`]): a workspace whose packs are not installed
/// misses on every query, and one line says so.
#[derive(Default)]
struct OverlayGenerations {
    held: Mutex<VecDeque<(OverlayKey, Arc<CommandRegistry>)>>,
    misses: Mutex<MissLog>,
}

/// An environment's canonical id and a pack overlay: what names a generation.
type OverlayKey = (String, u64);

/// The misses seen, and the ones not yet reported.
#[derive(Default)]
struct MissLog {
    seen: HashSet<OverlayKey>,
    unreported: Vec<OverlayMiss>,
}

impl OverlayGenerations {
    fn held(&self, key: &OverlayKey) -> Option<Arc<CommandRegistry>> {
        self.held
            .lock()
            .expect("overlay generation mutex")
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, registry)| Arc::clone(registry))
    }

    fn hold(&self, key: OverlayKey, registry: Arc<CommandRegistry>) {
        let mut held = self.held.lock().expect("overlay generation mutex");
        if held.iter().any(|(existing, _)| *existing == key) {
            return;
        }
        // A key that installs is no longer a miss on record: if it is retired
        // and misses again, that is a new miss for the host to report, not
        // one it has already heard about.
        self.misses
            .lock()
            .expect("overlay miss mutex")
            .seen
            .remove(&key);
        held.push_back((key, registry));
        while held.len() > HELD_OVERLAYS {
            held.pop_front();
        }
    }

    fn record(&self, miss: &OverlayMiss) {
        let mut log = self.misses.lock().expect("overlay miss mutex");
        if log.seen.insert((miss.environment.clone(), miss.overlay)) {
            log.unreported.push(miss.clone());
        }
    }
}

#[salsa::db]
impl salsa::Database for TclDatabase {}

impl Default for TclDatabase {
    fn default() -> Self {
        Self::with_storage(salsa::Storage::default())
    }
}

impl TclDatabase {
    /// A database over `storage`, its [`EvaluatorEpoch`] created at 0 before
    /// any query can run: a query that read the epoch's absence would record
    /// no dependency, and a later epoch would never reach it.
    fn with_storage(storage: salsa::Storage<Self>) -> Self {
        let db = Self { storage };
        let _epoch = EvaluatorEpoch::new(&db, 0);
        let _overlays = OverlayEpoch::new(&db, 0);
        let _modules = memoised_units::RetainedModules::new(&db, Arc::default());
        db
    }

    /// Construct a database that forwards, for every salsa `WillExecute` event,
    /// the `database_key` of the query about to run (its `Debug` string) to
    /// `logger`.  Lets a profiler count per-query re-executions across an edit
    /// without exposing `salsa::Event` in the public API.  See the
    /// `tail_profile` example's re-execution-breadth tier.
    #[must_use]
    pub fn with_event_logger(logger: impl Fn(String) + Send + Sync + 'static) -> Self {
        let storage = salsa::Storage::new(Some(Box::new(move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                logger(format!("{database_key:?}"));
            }
        })));
        Self::with_storage(storage)
    }

    /// Construct a database that forwards, for every interned-slot **reuse**
    /// (salsa's `DidReuseInternedValue`), the reused slot's `database_key`
    /// `Debug` string — `"<IngredientName>(Id(..))"` — to `logger`.
    ///
    /// That event is the only observable face of the interned garbage
    /// collector described under "The interned garbage collector is
    /// load-bearing": salsa fires it exactly when a cold intern recycles a
    /// stale LRU slot and clears its memo table.  A slot that is not
    /// collectable — durability above `Durability::LOW`, or interned with no
    /// active query — is never in the LRU list, so it can never produce this
    /// event.  Counting the events per ingredient is therefore a direct,
    /// machine-independent read of whether the collector is still working;
    /// `tests/interned_gc.rs` is built on it.
    #[must_use]
    pub fn with_interned_reuse_logger(logger: impl Fn(String) + Send + Sync + 'static) -> Self {
        let storage = salsa::Storage::new(Some(Box::new(move |ev: salsa::Event| {
            if let salsa::EventKind::DidReuseInternedValue { key, .. } = ev.kind {
                logger(format!("{key:?}"));
            }
        })));
        Self::with_storage(storage)
    }
}

/// The overlay generations the process's queries hold.
fn overlay_generations() -> &'static OverlayGenerations {
    static GENERATIONS: std::sync::OnceLock<OverlayGenerations> = std::sync::OnceLock::new();
    GENERATIONS.get_or_init(OverlayGenerations::default)
}

/// The overlay misses the process's queries have answered without the packs
/// since the last call, each distinct one once: the host reports them, so a
/// workspace whose packs are not installed is one line in its log rather than
/// one per query.
#[must_use]
pub fn take_overlay_misses() -> Vec<OverlayMiss> {
    std::mem::take(
        &mut overlay_generations()
            .misses
            .lock()
            .expect("overlay miss mutex")
            .unreported,
    )
}

/// The process's evaluator epoch as the database last took it
/// (`tcl_registry::pack_hooks::evaluator_epoch`): it moves when a hook plan
/// is published or a hook is quarantined, on any thread.
///
/// The memoised lattices are shared by every worker, while the hosts that
/// run declared implementations are per thread, so what a thread's host can
/// answer is invisible to a memo unless an input says it changed. This is
/// that input, and the smallest one that does it: [`compilation_unit`] and
/// [`proc_taint_solve`] read it, and every per-procedure
/// [`ValueTransferContext`] carries it, so a new epoch re-keys every
/// memoised lattice. The
/// language server sets it with [`set_evaluator_epoch`] where it reloads
/// packs and after each diagnostics pass, which is where it first sees a
/// quarantine on the worker that ran the pass.
///
/// At salsa's default `Durability::LOW`, as every input here is, though it
/// moves only at a reload or a quarantine: the per-body keys are interned by
/// the queries that read it (the crate docs' "The interned garbage collector
/// is load-bearing").
#[salsa::input(singleton)]
pub struct EvaluatorEpoch {
    #[returns(copy)]
    pub generation: u64,
}

/// The database's evaluator epoch, as a tracked read; `0` for a database
/// built without one.
fn evaluator_epoch(db: &dyn TclDb) -> u64 {
    EvaluatorEpoch::try_get(db).map_or(0, |epoch| epoch.generation(db))
}

/// The process's overlay epoch as the database last took it
/// (`tcl_registry::overlay_epoch`): it moves when an overlay generation is
/// installed or retired, on any thread.
///
/// The cache the overlays live in is process-wide state, so a query that found
/// no generation for an overlay has no input to say one has since been
/// installed, and a query that resolved one has none to say it was retired.
/// This is that input, taken the way [`EvaluatorEpoch`] is: every query that
/// resolves a pack overlay reads it ([`unit_registry`]), so what was answered
/// against the old state — a unit built, or none built for want of the packs —
/// is asked again when the host moves it. The language server sets it with
/// [`set_overlay_epoch`] where it publishes the pack key.
///
/// At salsa's default `Durability::LOW`, as every input here is: the per-body
/// keys are interned by the queries that read it (the crate docs' "The
/// interned garbage collector is load-bearing").
#[salsa::input(singleton)]
pub struct OverlayEpoch {
    #[returns(copy)]
    pub generation: u64,
}

/// The database's overlay epoch, as a tracked read; `0` for a database built
/// without one.
fn overlay_epoch(db: &dyn TclDb) -> u64 {
    OverlayEpoch::try_get(db).map_or(0, |epoch| epoch.generation(db))
}

/// Take `generation` as the database's overlay epoch, returning whether it
/// moved. A value it already holds writes nothing, so a sync that finds the
/// overlays unchanged invalidates no memo.
pub fn set_overlay_epoch(db: &mut TclDatabase, generation: u64) -> bool {
    use salsa::Setter as _;
    match OverlayEpoch::try_get(db) {
        Some(epoch) if epoch.generation(db) == generation => false,
        Some(epoch) => {
            epoch.set_generation(db).to(generation);
            true
        }
        None => {
            let _epoch = OverlayEpoch::new(db, generation);
            true
        }
    }
}

/// Take `generation` as the database's evaluator epoch, returning whether it
/// moved. A value it already holds writes nothing, so a sync that finds the
/// evaluators unchanged invalidates no memo.
pub fn set_evaluator_epoch(db: &mut TclDatabase, generation: u64) -> bool {
    use salsa::Setter as _;
    match EvaluatorEpoch::try_get(db) {
        Some(epoch) if epoch.generation(db) == generation => false,
        Some(epoch) => {
            epoch.set_generation(db).to(generation);
            true
        }
        None => {
            let _epoch = EvaluatorEpoch::new(db, generation);
            true
        }
    }
}

/// The database-wide revision used to reject results computed by an older
/// snapshot before applying them to the live database.
///
/// Salsa exposes this through its plumbing API rather than [`salsa::Database`];
/// keep that dependency inside the database owner instead of leaking it into
/// the LSP's publication protocol.
#[must_use]
pub fn database_revision(db: &TclDatabase) -> salsa::Revision {
    salsa::plumbing::current_revision(db)
}

#[salsa::db]
impl TclDb for TclDatabase {
    fn registry(&self, dialect: &str) -> &'static CommandRegistry {
        // The environment's **registry generation**, not a locally-assembled
        // registry: the name resolves through the one ingress seam (so an
        // alias, an editor language id, or a typo lands on the right
        // environment), and the generation's command store is the shared
        // per-`(environment, overlay)` `Arc` — the profile's base layers and
        // EDA packs loaded, and — the part a hand-rolled `build_default +
        // load_dialect` silently dropped — the *profile stamped* on the
        // registry. Every registry-derived behaviour query keys off that
        // stamp: without it `FoldPolicy::from_registry` reads the LSP's
        // iRules documents as plain Tcl and declines every word-operator fold.
        tcl_lsp_core::registry_for_dialect(dialect)
    }

    fn registry_with_overlay(
        &self,
        dialect: &str,
        overlay: u64,
    ) -> Result<Arc<CommandRegistry>, OverlayMiss> {
        let environment = tcl_lsp_core::environment_for_dialect(dialect);
        let generations = overlay_generations();
        let key = (environment.id().to_owned(), overlay);
        if let Some(held) = generations.held(&key) {
            return Ok(held);
        }
        match environment.context_registry(&KeyedVersions::default(), overlay) {
            Ok(generation) => {
                let commands = Arc::clone(generation.commands());
                generations.hold(key, Arc::clone(&commands));
                Ok(commands)
            }
            Err(miss) => {
                generations.record(&miss);
                Err(miss)
            }
        }
    }
}

/// A source document: text plus the dialect it is analysed under.
///
/// `set_text` (generated) is the single write on an edit — salsa cascades
/// invalidation to every query that read it.
///
/// # Durability must stay `LOW`
///
/// Write this input with a plain `set_*(…).to(…)`, never with
/// `Setter::with_durability` or the builder's `durability` / `*_durability`
/// methods.  Every per-body interned key in this crate is minted by a query
/// that has read this input, so the key inherits its durability — and salsa's
/// interned garbage collector only ever reclaims `Durability::LOW` slots.
/// Raising it looks like a revalidation win and is in fact a
/// KB-per-keystroke leak.  See the crate docs' "The interned garbage collector
/// is load-bearing"; enforced by `tests/interned_gc.rs`.
#[salsa::input(constructor = with_call_site_evidence)]
pub struct SourceFile {
    #[returns(ref)]
    pub text: String,
    #[returns(ref)]
    pub dialect: String,
    /// Source-file path (from the document URI), or `None` for in-memory
    /// text.  Path-keyed analysis behaviour: `pkgIndex.tcl` suppresses
    /// dead-store/unused hints on the loader-supplied `$dir`, and a file
    /// with a registry whole-file scoped environment (`tclpkg.tcl`
    /// manifests) is analysed with that environment ambient.
    #[returns(ref)]
    pub path: Option<String>,
    /// Call sites in **other** project files that reach the procedures this
    /// file defines — the cross-file half of the interprocedural constant
    /// seed.
    ///
    /// `None` means "no workspace view": the compilation unit is then on its
    /// own, and any registry-declared unit boundary in the file — including
    /// `source` / `package require`, whose loaded unit a workspace normally
    /// *does* contain — disables the seed rather than trusting an unprovable
    /// "every caller is in this file".  `Some` is the host asserting it
    /// enumerated the project, so the merged evidence is the whole picture;
    /// `Some` of an *empty* evidence set is the meaningful statement "no
    /// other file calls into this one".  A file that *publishes* commands
    /// (`package provide`, `namespace export`) declines either way — no
    /// project enumeration bounds another checkout's `package require`.
    ///
    /// Set by the server from [`project_call_site_evidence`] whenever the
    /// slice relevant to *this* file changes (compare-then-set), so a
    /// keystroke in an unrelated file leaves it — and every query reading it
    /// — untouched.
    #[returns(ref)]
    pub external_call_sites: Option<Arc<CallSiteEvidence>>,
    /// The workspace's user-defined `TclOO` **class factories** (metaclasses),
    /// keyed by qualified name — the oracle that lets this file's walk
    /// classify `::tk::Megawidget create IconList …` when `::tk::Megawidget`
    /// is written in another document.
    ///
    /// `None` means "no workspace view", and the walk abstains on every such
    /// call — the deliberate behaviour every standalone consumer keeps.
    ///
    /// Set by the server from [`project_class_factories_for_inputs`] (compare-then-set),
    /// the same discipline [`Self::external_call_sites`] uses: an index that
    /// does not move invalidates nothing, and almost no edit moves it, since
    /// almost no document declares a metaclass.
    #[returns(ref)]
    pub workspace_class_factories: Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
    /// Instance methods dispatchable on some workspace **descendant** of each
    /// class, keyed by ancestor qualified name — the cross-file half of the
    /// template-method W308 abstention: a base class calling
    /// `my Render` is refuted by a subclass in a sibling document, which the
    /// per-file analysis cannot see.
    ///
    /// `None` means "no workspace view", and the warning fires on the
    /// per-file evidence alone — the behaviour every standalone consumer
    /// keeps.
    ///
    /// Set by the server from the workspace index (compare-then-set), the
    /// same discipline as its two siblings above: a view that does not move
    /// invalidates nothing.  Unlike the factories this feeds only the
    /// *analysis* queries, never [`item_tree`] — which methods a subclass
    /// provides changes no declaration structure.
    #[returns(ref)]
    pub workspace_subclass_methods: Option<Arc<tcl_compiler::analyser::SubclassProvidedMethods>>,
    /// Monotonic revision of workspace sidecar stubs. The server advances this
    /// input for every `<dialect>.tcl.stubs` create/change/delete, so Salsa
    /// invalidates analyses that read an external declaration bundle.
    #[returns(copy)]
    pub sidecar_stubs_epoch: u64,
    /// Procedures other workspace documents declare whose bodies leave the
    /// command world unchanged, as declarations: a call to one keeps the
    /// rest of this document proved. `None` means "no workspace view".
    /// Set by the server (compare-then-set) like the evidence fields above.
    #[returns(ref)]
    pub workspace_declared_commands: Option<Arc<DeclaredSurface>>,
    /// Sibling files this document may auto-load: each offers the classes it
    /// defines, sourced in full when the document calls one while absent.
    #[returns(ref)]
    pub workspace_source_modules:
        Option<Arc<Vec<tcl_compiler::command_binding::TrustedSourceModuleLoader>>>,
}

impl SourceFile {
    /// Create a source file with **no** cross-file view (`external_call_sites
    /// = None`) — the shape every standalone consumer (tests, examples, the
    /// CLI's single-file paths) wants.  Hosts with a workspace call
    /// [`Self::with_call_site_evidence`] instead, or set the field later.
    pub fn new(
        db: &dyn salsa::Database,
        text: String,
        dialect: String,
        path: Option<String>,
    ) -> Self {
        Self::with_call_site_evidence(db, text, dialect, path, None, None, None, 0, None, None)
    }
}

/// Analyser configuration mirrored from the editor (covers the server's
/// `disabled_diagnostics` / `non_ascii_mode` state).  One input
/// instance shared by every file's analysis; setting it recomputes all
/// analyses.
///
/// # Durability must stay `LOW`
///
/// This is the input a "it only changes when the user edits settings"
/// argument most tempts one to mark `Durability::HIGH`.  Do not: an analysis
/// query that has read this config interns [`ItemBodyKey`] under the minimum
/// durability of everything it read, so a `HIGH` config makes those body keys
/// uncollectable.  Same contract as [`SourceFile`] — see the crate docs' "The
/// interned garbage collector is load-bearing".
#[salsa::input]
pub struct AnalyserConfig {
    #[returns(ref)]
    pub disabled_diagnostics: Vec<String>,
    #[returns(copy)]
    pub non_ascii_mode: NonAsciiMode,
    /// User-declared extra command names (`tclLsp.extraCommands`) treated as
    /// known by the unknown-command (W123) check.
    #[returns(ref)]
    pub extra_commands: Vec<String>,
    /// Generic `static::` variable-name patterns for IRULE4002
    /// (`tclLsp.diagnostics.genericVariablePatterns`). `None` selects the
    /// built-in default set; `Some(list)` replaces it (an empty list disables
    /// the check).
    #[returns(ref)]
    pub generic_variable_patterns: Option<Vec<String>>,
    /// Target BIG-IP release (`tclLsp.bigipVersion`) for the keyed
    /// library-version axis; `None` = the oldest-supported default.
    #[returns(ref)]
    pub bigip_version: Option<String>,
    /// The workspace's loaded `SpecTcl` pack set, by content identity
    /// (`PackSet::key`; `0` = no packs) — `Analyser::with_pack_overlay`.
    ///
    /// An input, not an ambient read, so a pack edit invalidates exactly the
    /// analyses that depend on it. It is not optional configuration: the EDA
    /// vendor libraries ship as bundled loadables
    /// (`docs/design/registry/spec-packs.md`), so this number is what decides whether
    /// `synth_design` is a known command.
    #[returns(copy)]
    pub spec_pack_key: u64,
    /// Declared version-target ranges (`tclLsp.targets`) as
    /// `(provider, range)` pairs — `("tcl",
    /// "8.5-9.0")`, `("Tk", "8.5-8.6")` — feeding
    /// `Analyser::with_declared_targets`. Empty — the default — leaves
    /// range mode off; a source-level `# tcl-lsp: supports` directive
    /// overrides the configured pair for the same provider.
    #[returns(ref)]
    pub targets: Vec<(String, String)>,
    /// Declared "requiring this package also loads these" edges
    /// (`tclLsp.packages.provides` / `.tcl-lsp.ini` `[packages.provides]`),
    /// feeding `Analyser::with_package_provides`. A binary extension whose
    /// `Init` calls `Tcl_PkgRequire` / `Tk_InitStubs` leaves nothing in any
    /// Tcl source to scan for, so the dependency can only be declared.
    /// Empty — the default — changes nothing.
    #[returns(ref)]
    pub package_provides: Vec<(String, Vec<String>)>,
    /// The encoding the target reads scripts in (`tclLsp.sourceEncoding`).
    /// `Auto` — the default — leaves the release to decide, so 8.x folds
    /// over non-ASCII text decline; a declared encoding is also the one the
    /// driver decoded the document with.
    #[default]
    #[returns(copy)]
    pub script_encoding: tcl_registry::value_transfer::ScriptEncoding,
}

/// Immutable values of the actual selected diagnostic configuration.
/// Editing input identity is independent: options such as declared target
/// ranges change findings without changing the lexer or Registry generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticConfigurationSnapshot {
    disabled_diagnostics: Vec<String>,
    non_ascii_mode: NonAsciiMode,
    extra_commands: Vec<String>,
    generic_variable_patterns: Option<Vec<String>>,
    bigip_version: Option<String>,
    spec_pack_key: u64,
    targets: Vec<(String, String)>,
    package_provides: Vec<(String, Vec<String>)>,
    script_encoding: tcl_registry::value_transfer::ScriptEncoding,
}

impl AnalyserConfig {
    /// Retain every actual configuration field independently of the selected
    /// editing input. This grants no analysis completion or Native policy.
    #[must_use]
    pub fn diagnostic_report_configuration(
        self,
        db: &dyn salsa::Database,
    ) -> DiagnosticConfigurationSnapshot {
        DiagnosticConfigurationSnapshot {
            disabled_diagnostics: self.disabled_diagnostics(db).clone(),
            non_ascii_mode: self.non_ascii_mode(db),
            extra_commands: self.extra_commands(db).clone(),
            generic_variable_patterns: self.generic_variable_patterns(db).clone(),
            bigip_version: self.bigip_version(db).clone(),
            spec_pack_key: self.spec_pack_key(db),
            targets: self.targets(db).clone(),
            package_provides: self.package_provides(db).clone(),
            script_encoding: self.script_encoding(db),
        }
    }
}

/// The set of files in a workspace/project — the salsa-native replacement for the
/// off-graph [`tcl_lsp_core::workspace_index::WorkspaceIndex`] file set.
/// Lifting the project onto the salsa graph is what
/// lets cross-file queries (below) get *precise reverse-dependency invalidation*
/// for free: editing one file recomputes only the cross-file facts that actually
/// read it.  Setting `files` (open/close) recomputes the project aggregates;
/// editing a file's text does not touch this input.
///
/// # Durability must stay `LOW`
///
/// The file *set* really does change rarely, which is exactly why this input
/// attracts a durability bump.  It carries the same prohibition as
/// [`SourceFile`] and [`AnalyserConfig`]: a cross-file query that has read it
/// interns under its durability, and only `Durability::LOW` slots are
/// garbage-collected.  See the crate docs' "The interned garbage collector is
/// load-bearing".
#[salsa::input(constructor = with_token_configurations)]
pub struct Project {
    /// The project's files (workspace + open documents).
    #[returns(ref)]
    pub files: Vec<SourceFile>,
    /// Actual per-file settings captured by the document driver. Missing
    /// mappings withhold supplied project source facts; standalone queries are separate.
    #[returns(ref)]
    pub token_configurations: Option<Vec<(SourceFile, AnalyserConfig)>>,
}

impl Project {
    /// Construct a project for the explicit standalone aggregate queries.
    pub fn new(db: &dyn salsa::Database, files: Vec<SourceFile>) -> Self {
        Self::with_token_configurations(db, files, None)
    }
}

#[derive(Clone, Copy)]
enum ProjectSourceInputMode {
    Standalone,
    Supplied,
}

/// The checks-path memoised solve for one document: the interprocedural taint
/// result **and** the rebased per-procedure non-taint checks, both
/// gathered from a single re-derived [`build_unit_with_keys`] so the duplicate
/// build is paid once for both halves.  `PartialEq` for salsa early-cutoff.
#[derive(Clone)]
pub struct CheckSolve {
    /// Actual immutable compilation whose Module-issued IDs key this solve.
    unit: Arc<CompilationUnit>,
    /// Interprocedural taint solve (`proc_summary_cascade`-memoised summaries).
    pub taints: InterprocTaintResult,
    /// Per-procedure non-taint checks (`function_checks`-memoised), already
    /// rebased to each procedure's real position.
    pub fn_checks: Vec<CompilerCheck>,
    /// The document's optimisations, assembled from the per-procedure
    /// [`function_optimisations`] memo + the whole-module `finalise_optimisations`
    /// tail (or the whole-module `optimise_unit` fallback).  Byte-identical to a
    /// bare `optimise_unit`.
    pub optimisations: Vec<Optimisation>,
}

impl PartialEq for CheckSolve {
    fn eq(&self, other: &Self) -> bool {
        self.unit == other.unit
            && self.taints == other.taints
            && self.fn_checks == other.fn_checks
            && self.optimisations == other.optimisations
            && same_optimisation_provenance(&self.optimisations, &other.optimisations)
    }
}

mod project_call_inputs;
pub use project_call_inputs::{
    file_external_call_sites_for_inputs, file_source_targets_for_inputs,
};
mod project_source_inputs;
pub use project_source_inputs::{
    item_sigs_for_config, item_tree_for_config,
    project_callback_diagnostics_for_analysis_with_inputs, project_callback_diagnostics_for_inputs,
    project_class_factories_for_inputs, project_factory_header_census_for_inputs,
    project_original_command_signatures_for_inputs,
};

mod project_token_inputs;
pub use project_token_inputs::{
    file_token_facts_for_config, project_class_index_for_inputs,
    project_named_instance_index_for_inputs, project_proc_var_index_for_inputs,
    semantic_tokens_project_for_inputs,
};

#[cfg(test)]
mod test_overlays;

#[cfg(test)]
mod value_transfer_parity;

#[cfg(test)]
mod original_uncached_metadata_tests;

#[cfg(test)]
mod original_tracked_metadata_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod compiler_snapshot_memo_tests;

#[cfg(test)]
mod original_project_callback_projection_tests;

#[cfg(test)]
mod readonly_source_optimisation_owner_tests;

#[cfg(test)]
mod original_optimiser_memo_transport_tests;

#[cfg(test)]
tcl_registry::testing::install_shipped!();
