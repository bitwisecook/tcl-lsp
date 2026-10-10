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
//! | [`OptDepsKey`] | `body_source` + `proc_body_source` |
//! | [`TaintSummaryKey`] | `reachable` — shifts as the projections move |
//! | [`SummaryDepsKey`] | `interproc_reachable` + `callee_summaries` |
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
//!    which this workspace never overrides.  Marking the document text or
//!    the analyser config `HIGH` "because it rarely changes" would stamp
//!    every key in the table above non-`LOW`, and none would ever be
//!    collected again.
//! 2. **The per-body keys are interned from inside a tracked query.**  That is
//!    why `memoised_compilation_unit` is crate-private and documented as
//!    tracked-query-only: reached from [`compilation_unit`] its interning
//!    inherits that query's `LOW` durability, whereas a direct call from
//!    untracked code mints immortal slots holding whole `CompilationUnit`
//!    lattices.
//!
//! Interned structs with a *bounded* key space are outside this contract:
//! [`LexerCfgKey`] is two booleans and [`CommandTail`] one command name, so
//! minting them from untracked host code (as [`compilation_unit`]'s own
//! signature requires) can retain only a handful of tiny slots.
//! [`CfgContext`] is content-keyed but signature-level, so a body edit leaves
//! it alone; it is collected by the same mechanism when a signature does move.
//!
//! Neither rule is expressible in the type system, so both are pinned
//! behaviourally by `tests/interned_gc.rs`.  It drives a typing session through
//! the production queries and counts salsa's `DidReuseInternedValue` events per
//! ingredient (see [`TclDatabase::with_interned_reuse_logger`]) — the event the
//! collector fires when it recycles a slot, and one that a non-`LOW` or
//! untracked slot can never produce.  A control session repeats the run with
//! both inputs at `Durability::HIGH` and asserts the collector goes silent,
//! which is what keeps the first test from going vacuous.  A third test fails
//! on any `with_durability` / input-builder `durability` call site in this
//! crate or in `tcl-lsp-server` (its only dependant), because durability is
//! chosen at the setter call site where the behavioural tests cannot see it.
//! The full contract is written up in
//! `docs/design/rust/salsa-interned-gc.md`.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::cfg_builder::{PreparedCfgContext, build_cfg_function_with_prepared_context};
use tcl_compiler::command_binding::ModuleCommandBindings;
use tcl_compiler::compilation_unit::{
    CompilationUnit, FunctionUnit, LatticeRequest, ModuleTraceFacts, UnitBuildOptions,
};
use tcl_compiler::compiler_checks::{DiagCode, Diagnostic as CompilerCheck};
use tcl_compiler::interprocedural::{InterproceduralAnalysis, ProcSummary};
use tcl_compiler::ir::Script;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::ssa::ValueKey;
use tcl_compiler::taint::TaintLattice;
use tcl_compiler::unit_scope::CallSiteEvidence;
// The compiler's per-proc return-taint summary (the colour-aware transfer
// function the interprocedural fixpoint converges) — aliased to avoid clashing
// with this crate's `ProcTaintSummary` (the *interproc-analysis* projection in
// `TaintSummaryKey`).  Used to memoise the summary fixpoint per procedure.
use tcl_compiler::taint_interproc::{InterprocTaintResult, ProcTaintSummary as ReturnTaintSummary};

use tcl_compiler::analyser::per_item::{BodyFragment, DeferredBody, analyse_proc_body_isolated};
use tcl_compiler::analyser::{
    Analyser, AnalysisResult, ClassDef, ClassHierarchy, FileDecls, ItemSig, ItemTree, NonAsciiMode,
    build_class_hierarchy,
};
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_lsp_core::document_symbols::DocumentSymbol;
use tcl_lsp_core::folding::FoldingRange;
use tcl_lsp_core::semantic_tokens::{SemanticTokens, VarNameArgRoles};
use tcl_registry::CommandRegistry;
use tcl_registry::model::{DeclaredSurface, KeyedVersions, OverlayMiss};

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
        Self::with_call_site_evidence(db, text, dialect, path, None, None, None, 0)
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
}

fn document_analyser(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Analyser {
    let disabled = config.disabled_diagnostics(db).iter().cloned().collect();
    let extra = config.extra_commands(db).iter().cloned().collect();
    Analyser::with_disabled_diagnostics(disabled)
        .with_non_ascii_mode(config.non_ascii_mode(db))
        .with_pack_overlay(config.spec_pack_key(db))
        .with_extra_commands(extra)
        .with_bigip_version(config.bigip_version(db).clone())
        .with_declared_targets(config.targets(db).clone())
        .with_package_provides(config.package_provides(db).clone())
        .with_file_path(file.path(db).clone())
        .with_workspace_class_factories(file.workspace_class_factories(db).clone())
        .with_workspace_subclass_methods(file.workspace_subclass_methods(db).clone())
}

/// The checked document input shared by analysis and compilation. This query
/// resolves context only; it performs no source walk and depends on no CU query.
///
/// # Errors
/// An unavailable overlay remains a typed miss until its epoch changes.
#[salsa::tracked(returns(clone))]
pub fn document_analysis_input(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Result<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>, OverlayMiss> {
    if config.spec_pack_key(db) != 0 {
        let _epoch = OverlayEpoch::try_get(db).map_or(0, |epoch| epoch.generation(db));
    }
    document_analyser(db, file, config)
        .prepare_analysis_input(file.dialect(db))
        .map(Arc::new)
}

fn unavailable_document_analysis(dialect: &str, miss: OverlayMiss) -> Arc<AnalysisResult> {
    let mut result = AnalysisResult::default();
    result.dialect = dialect.to_owned();
    result.analysis_context_unavailable = Some(miss);
    Arc::new(result)
}

/// Whole-file analysis, behind an `Arc` so reads bump a refcount rather than
/// deep-clone.
///
/// Wraps [`Analyser::analyse`] unchanged, uncancellable and with no per-item
/// memoisation.  Every production feature provider reads
/// [`file_analysis_incremental`] instead: this coarse query has no
/// interior salsa cancellation checkpoint, so a caller holding a read blocks a
/// concurrent edit's `set_text` until the whole walk finishes. `file_analysis`
/// itself stays live as the differential-fuzzer / corpus-gate ground truth
/// [`file_analysis_incremental`] is proven byte-identical against.
///
/// One deliberate asymmetry: this query lets the analyser build its **own**
/// compilation unit, so it never sees [`SourceFile::external_call_sites`],
/// while [`file_analysis_incremental`] feeds it the shared
/// [`compilation_unit`] (which does). The two therefore agree exactly when
/// the file has no cross-file view — which is every gate and fuzzer input,
/// since [`SourceFile::new`] leaves the field `None`. Production reads only
/// the incremental query.
///
/// `config.disabled_diagnostics` is the analyser's production-time skip —
/// rule 2's permitted saving in `docs/design/compiler/diagnostic-policy.md`
/// § What the producers leave to the policy: the codes the document's policy
/// turns off, which the analyser need not compute. It is never a presentation filter. The
/// surface that reads this analysis declares the same set to its report, so a
/// code left uncomputed is explained rather than read as clean; what the
/// document shows is the policy step's decision.
#[salsa::tracked(returns(clone))]
pub fn file_analysis(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let input = match document_analysis_input(db, file, config) {
        Ok(input) => input,
        Err(miss) => return unavailable_document_analysis(file.dialect(db), miss),
    };
    Arc::new(
        document_analyser(db, file, config)
            .with_resolved_input((*input).clone())
            .analyse(file.text(db), file.dialect(db)),
    )
}

/// Explicit standalone item inventory, keyed by stable declaration name/kind.
/// Configured document providers use [`item_tree_for_config`] so selected
/// pack definitions, source grammar and available generations stay attached.
///
/// `ensemble_namespaces` is retained on the analyser, so this structure-only
/// query reads the item/header projection directly from that same source walk.
/// The published workspace factory oracle remains an input to the file.
#[salsa::tracked(returns(clone))]
pub fn item_tree(db: &dyn salsa::Database, file: SourceFile) -> Arc<ItemTree> {
    // `structure_only` skips diagnostic emission (the dominant analyse cost)
    // while building the identical declaration/scope structure — a cheap,
    // non-divergent item extractor (gated by `file_decls_corpus`).
    let mut analyser = Analyser::new()
        .structure_only()
        // The workspace class-factory oracle is part of the *file's* input,
        // not the analyser config, so the item tree stays a
        // function of `SourceFile` alone. It has to be here: a class a
        // cross-file metaclass manufactures is a declaration of this file, and
        // leaving it out of the item tree would leave it out of `file_decls`
        // too — so cross-file W123 would call `IconList` an unknown command
        // while the full analysis, hover, and the outline all describe it.
        .with_workspace_class_factories(file.workspace_class_factories(db).clone());
    let result = analyser.analyse(file.text(db), file.dialect(db));
    Arc::new(ItemTree::from_analysis(
        &result,
        &analyser.ensemble_namespaces,
    ))
}

/// Explicit standalone item signatures — cross-item headers with bodies stripped
/// (`item_sig*` in the design graph). A body-only edit leaves these equal, so
/// [`file_decls`] and the future cross-item passes early-cutoff.
#[salsa::tracked(returns(clone))]
pub fn item_sigs(db: &dyn salsa::Database, file: SourceFile) -> Arc<Vec<ItemSig>> {
    Arc::new(item_tree(db, file).sigs())
}

/// Aggregate declaration sets (`file_decls ← item_sig*`): the set of declared
/// procs / classes / aliases / ensembles + the namespace tree the cross-item
/// passes (W123 / arity) read read-only.
#[salsa::tracked(returns(clone))]
pub fn file_decls(db: &dyn salsa::Database, file: SourceFile) -> Arc<FileDecls> {
    Arc::new(FileDecls::from_sigs(item_sigs(db, file).iter()))
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

/// The project-wide set of declared `proc` qualified names — the cross-file
/// command-resolution domain (e.g. W123 unresolved-command suppression against
/// the workspace's procs).  The salsa-native replacement for `WorkspaceIndex`'s
/// proc-name set: it lifts the project signature table into salsa.
///
/// Depends **only** on each file's [`file_decls`] — the signature firewall — so a
/// **body edit in any file leaves it unchanged** → it backdates → **zero
/// cross-file work**; only a signature/decl change (a proc added / removed /
/// renamed) recomputes it. This is the input discipline extended across
/// files: a keystroke that does not alter a signature wakes nobody project-wide.
/// Proven by `project_proc_names_firewall` (a body edit re-runs zero
/// `project_proc_names`; a decl change re-runs exactly one).
#[salsa::tracked(returns(clone))]
pub fn project_proc_names(db: &dyn salsa::Database, project: Project) -> Arc<BTreeSet<String>> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for &file in project.files(db) {
        names.extend(file_decls(db, file).procs.iter().cloned());
    }
    Arc::new(names)
}

/// The **class factories** one file declares — the user-defined `TclOO`
/// metaclasses (`oo::class create Meta { superclass oo::class; self method
/// create … }`) it publishes to the rest of the workspace.
///
/// Read straight off [`item_tree`], which every project file already
/// computes for [`file_decls`] / [`project_proc_names`], so the factory index
/// costs no extra analysis pass. Structure-level, like [`file_decls`]: a body
/// edit that does not change a metaclass's `create` override leaves this
/// equal, so it backdates and no cross-file query re-runs.
///
/// [`item_tree`] itself reads [`SourceFile::workspace_class_factories`], so
/// this query is a function of the very input the host computes *from* it. A
/// metaclass that is **itself** manufactured by another file's metaclass is
/// therefore not provable until the manufacturing metaclass has already been
/// published: the file holding `MetaA create MetaB` proves `MetaB` only on a
/// round whose index already names `MetaA`. One publish is consequently one
/// link of the chain deep, and no more — so the host must **iterate the
/// publish to a fixpoint**, not publish once: a single pass would leave a
/// cross-file `MetaA` → `MetaB` → `Widget` chain's `Widget` unknown, with a
/// call site on one of its methods resolving to nothing.
///
/// The fixpoint is cheap and terminates: a round adds an entry only when some
/// file *proved* it — never a guess — so the round function is normally
/// monotone over a set bounded by the project's creation calls, and the host
/// stops as soon as a round moves nothing. A workspace with no metaclass at
/// all computes an empty index, which the host stores as `None`, so it settles
/// in one round that writes nothing and invalidates nothing. The host caps the
/// loop regardless rather than assuming convergence — see
/// `sync_workspace_class_factories` in `tcl-lsp-server`.
#[salsa::tracked(returns(clone))]
pub fn file_class_factories(
    db: &dyn salsa::Database,
    file: SourceFile,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    Arc::clone(&item_tree(db, file).class_factories)
}

/// The project-wide merge of every file's [`file_class_factories`] — the
/// oracle the server sets on [`SourceFile::workspace_class_factories`].
///
/// Unordered, exactly as the workspace index's other cross-file facts are: a
/// `source` graph can prove a load order, but nothing requires one here —
/// a metaclass declaration either exists in the project or it does not.
/// Later entries win on a duplicate qualified name, which cannot be told
/// apart from the earlier one by anything the index knows.
#[salsa::tracked(returns(clone))]
pub fn project_class_factories(
    db: &dyn salsa::Database,
    project: Project,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    merge_class_factories(
        project
            .files(db)
            .iter()
            .map(|&file| file_class_factories(db, file)),
    )
}

fn merge_class_factories(
    parts: impl IntoIterator<Item = Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut merged = tcl_compiler::analyser::ClassFactoryIndex::new();
    for part in parts {
        for (qname, factory) in part.iter() {
            merged.insert(qname.clone(), factory.clone());
        }
    }
    Arc::new(merged)
}

/// The coverage-relevant projection of one `source` site.
///
/// Spans are intentionally absent: inserting text before an unchanged source
/// command must not invalidate project topology or cross-file coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSourceTarget {
    /// Verbatim path text, including any substitution markers.
    pub raw_path: String,
    /// Whether the path is a plain literal known at analysis time.
    pub is_literal: bool,
}

/// Every `source` target in the file's current Salsa revision.
///
/// Kept as a separate signature-level query so consumers that must distinguish
/// a literal target from a computed one share the same scan as
/// [`file_link_targets`] instead of consulting the independently-published
/// workspace index.
#[salsa::tracked(returns(clone))]
pub fn file_source_targets(db: &dyn TclDb, file: SourceFile) -> Arc<Vec<FileSourceTarget>> {
    let dialect = file.dialect(db).clone();
    let registry = db.registry(&dialect);
    Arc::new(
        tcl_compiler::signature_scan::extract_signatures(file.text(db), registry)
            .source_targets
            .into_iter()
            .map(|target| FileSourceTarget {
                raw_path: target.raw_path,
                is_literal: target.is_literal,
            })
            .collect(),
    )
}

/// The project files this file pulls into its own interpreter — its resolved
/// literal `source` targets, as document-path strings keyed the same way
/// [`SourceFile::path`] is.
///
/// Only *literal* targets are recorded. A computed path (`source [file join
/// $dir x.tcl]`) is not resolved here: guessing an edge would be worse than
/// missing one, because the edge widens what an unenumerable dispatch is
/// allowed to reach (see [`file_dispatch_reach`]).
///
/// Signature-level, like [`file_decls`]: a body edit that does not add or
/// remove a `source` leaves this equal, so it backdates and no cross-file
/// query re-runs.
#[salsa::tracked(returns(clone))]
pub fn file_link_targets(db: &dyn TclDb, file: SourceFile) -> Arc<BTreeSet<String>> {
    let Some(path) = file.path(db).as_deref() else {
        return Arc::new(BTreeSet::new());
    };
    let parent = std::path::Path::new(path);
    Arc::new(
        file_source_targets(db, file)
            .iter()
            .filter(|target| target.is_literal)
            .map(|target| {
                tcl_lsp_core::source_graph::resolve_source_target(parent, &target.raw_path)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect(),
    )
}

/// Union-find root of `i`, with path halving.
fn component_root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// The `source`-connected components of a project, each carrying the merged
/// declarations of its members — the per-project half of
/// [`file_dispatch_reach`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DispatchComponents {
    /// Component number of each project file, positionally aligned with
    /// [`Project::files`].
    component_of: Vec<usize>,
    /// Per component, every procedure its member files declare.  Shared by
    /// `Arc` with each member's [`file_dispatch_reach`], so a component is
    /// merged once and stored once however many files belong to it.
    procs: Vec<Arc<BTreeSet<String>>>,
}

impl DispatchComponents {
    /// The procedures the file at `index` in [`Project::files`] may reach
    /// through an unenumerable dispatch; `None` when `index` is past the file
    /// set this was computed for.
    #[must_use]
    pub fn reach(&self, index: usize) -> Option<&Arc<BTreeSet<String>>> {
        self.procs.get(*self.component_of.get(index)?)
    }
}

/// Decompose the project into `source`-connected components once per revision.
///
/// `set cmd [gets stdin]; $cmd dev` names a command this analysis cannot
/// determine, so it is a caller of *something*. Which something is bounded by
/// the interpreter the file's script runs in, and `source` is what puts two
/// files in one interpreter — in **both** directions: a file reaches the procs
/// of what it sources, and equally the procs of whatever sources *it*, because
/// its script runs in that caller's interpreter. Hence connected components of
/// the undirected `source` graph, not reachability along its arrows.
///
/// Bounding it this way is what keeps the blast radius proportionate: an
/// unrelated file in the same workspace folder is not in the component, so its
/// `eval $script` cannot withdraw this file's seeds. Without any bound, one
/// such file disables interprocedural folding project-wide; with a
/// *directional* bound (what the file loads, but not what loads it) a sourced
/// library's unreadable dispatch silently fails to retract its sourcing file's
/// seeds — the hole this replaces.
///
/// **Why this is a project query rather than per-file work:** the
/// union-find is over the whole `source` graph and the merge visits every
/// component member's [`file_decls`], so computing it inside
/// [`file_dispatch_reach`] cost `O(N)` *per file* — `O(N²)` `String` clones and
/// path-map inserts across a project, which a signature edit re-paid in full
/// because it invalidates every file's reach at once. Hoisted here it is paid
/// once, and each file's reach becomes an `Arc` clone.
///
/// Depends only on signature-level facts ([`file_link_targets`],
/// [`file_decls`]), so it sits behind the same firewall as the rest of the
/// cross-file layer.
#[salsa::tracked(returns(clone))]
pub fn project_dispatch_components(db: &dyn TclDb, project: Project) -> Arc<DispatchComponents> {
    merge_dispatch_components(project.files(db).iter().map(|&file| {
        (
            file.path(db).clone(),
            file_link_targets(db, file),
            file_decls(db, file),
        )
    }))
}

fn merge_dispatch_components(
    rows: impl IntoIterator<Item = (Option<String>, Arc<BTreeSet<String>>, Arc<FileDecls>)>,
) -> Arc<DispatchComponents> {
    let rows: Vec<_> = rows.into_iter().collect();
    // Index files by path so `source` targets can be matched to project files.
    let mut by_path: HashMap<&str, usize> = HashMap::new();
    for (i, (path, _, _)) in rows.iter().enumerate() {
        if let Some(p) = path.as_deref() {
            by_path.insert(p, i);
        }
    }
    // Union-find over the undirected `source` graph.
    let mut parent: Vec<usize> = (0..rows.len()).collect();
    for (i, (_, links, _)) in rows.iter().enumerate() {
        for target in links.iter() {
            let Some(&j) = by_path.get(target.as_str()) else {
                continue;
            };
            let (a, b) = (
                component_root(&mut parent, i),
                component_root(&mut parent, j),
            );
            if a != b {
                parent[a] = b;
            }
        }
    }
    // Number the roots densely, then merge each component's declarations once.
    let mut number: HashMap<usize, usize> = HashMap::new();
    let mut component_of: Vec<usize> = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let root = component_root(&mut parent, i);
        let next = number.len();
        component_of.push(*number.entry(root).or_insert(next));
    }
    let mut merged: Vec<BTreeSet<String>> = vec![BTreeSet::new(); number.len()];
    for (i, (_, _, declarations)) in rows.iter().enumerate() {
        merged[component_of[i]].extend(declarations.procs.iter().cloned());
    }
    Arc::new(DispatchComponents {
        component_of,
        procs: merged.into_iter().map(Arc::new).collect(),
    })
}

/// The procedures an *unenumerable* dispatch in `file` may reach — every proc
/// declared by a file in the same `source`-connected component.
///
/// An index into [`project_dispatch_components`], which does the work; see
/// there for what the component bound means and why it is not computed here.
/// A file the project does not contain reaches only its own declarations.
///
/// Kept as its own query so the result still early-cutoffs per file: a decl
/// change in one component recomputes the project decomposition, but every
/// file outside that component gets back an equal set and backdates, leaving
/// [`file_call_site_evidence`] untouched.
#[salsa::tracked(returns(clone))]
pub fn file_dispatch_reach(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Arc<BTreeSet<String>> {
    let Some(me) = project.files(db).iter().position(|f| *f == file) else {
        return Arc::new(file_decls(db, file).procs.clone());
    };
    let components = project_dispatch_components(db, project);
    match components.reach(me) {
        Some(reach) => Arc::clone(reach),
        None => Arc::new(file_decls(db, file).procs.clone()),
    }
}

/// Every call site **this** file contributes, resolved against the whole
/// project's procedure names.
///
/// The per-file half of the cross-file interprocedural seed:
/// `lib.tcl`'s own compilation unit can never see `main.tcl`'s `helper dev`,
/// so each file publishes its call sites here and
/// [`project_call_site_evidence`] merges them.
///
/// Depends on the file's text **and** on [`project_proc_names`] — the
/// signature firewall — so a body edit anywhere else in the project leaves
/// this query's inputs unchanged and it is not re-run.  Its own result
/// early-cutoffs too: an edit that does not move a call site's literal
/// arguments produces an equal `CallSiteEvidence` and backdates, so
/// [`project_call_site_evidence`] (and every compilation unit downstream of
/// it) is left alone.
#[salsa::tracked(returns(clone))]
pub fn file_call_site_evidence(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Arc<CallSiteEvidence> {
    let dialect = file.dialect(db).clone();
    let registry = db.registry(&dialect);
    let known: std::collections::HashSet<String> =
        project_proc_names(db, project).iter().cloned().collect();
    let reach: Vec<String> = file_dispatch_reach(db, file, project)
        .iter()
        .cloned()
        .collect();
    // The file's own stub declarations, through the same query the build
    // reads: a call site sitting in a stub-declared body or behind a
    // stub-declared callback counts for this file exactly as a catalogue one
    // does.
    let declared = declared_command_surface(db, file);
    let scanned = tcl_compiler::unit_scope::scan_source_call_sites(
        file.text(db),
        registry,
        Some(&declared),
        tcl_lsp_core::profile_for_dialect(&dialect),
        &known,
        &reach,
    );
    // Drop the callees *this* file declares. A call to a name the file also
    // defines binds to its own definition, so it is in-unit evidence (already
    // collected during the build) — not evidence about a same-named proc in
    // some other file.
    //
    // Without this, any two unrelated workspace files that happen to reuse a
    // common helper name (`helper`, `init`, `run` — pervasive in Tcl) pool
    // their call sites: differing arities make `binds_position` fail and
    // differing literals contradict each other, so *both* files lose folds
    // they are individually entitled to, and editing one changes diagnostics
    // in the other. Excluding self-declared callees keeps the merge to what
    // the query name promises — the call sites a file contributes to *other*
    // files' procedures.
    let declared = file_decls(db, file);
    let external: Vec<&str> = scanned
        .callees()
        .filter(|qname| !declared.procs.contains(*qname))
        .collect();
    Arc::new(scanned.slice_for(external.into_iter()))
}

/// The project-wide merge of every file's [`file_call_site_evidence`].
///
/// One table serves every file: a unit's own call sites are already in it, and
/// merging is monotone, so handing a file the whole project's view is exactly
/// what it would have collected had the project been one source text.
#[salsa::tracked(returns(clone))]
pub fn project_call_site_evidence(db: &dyn TclDb, project: Project) -> Arc<CallSiteEvidence> {
    let mut merged = CallSiteEvidence::default();
    for &file in project.files(db) {
        merged.merge_from(&file_call_site_evidence(db, file, project));
    }
    Arc::new(merged)
}

/// The slice of [`project_call_site_evidence`] that concerns the procedures
/// `file` itself declares — what a host sets on
/// [`SourceFile::external_call_sites`].
///
/// Narrowing to the file's own declarations is what keeps invalidation
/// precise: editing a call site in `main.tcl` changes only the evidence of the
/// file that *defines* the callee, so only that file's compilation unit is
/// rebuilt.
#[salsa::tracked(returns(clone))]
pub fn file_external_call_sites(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
) -> Arc<CallSiteEvidence> {
    let declared = file_decls(db, file);
    let all = project_call_site_evidence(db, project);
    Arc::new(all.slice_for(declared.procs.iter().map(String::as_str)))
}

/// Original unresolved source name retained by the diagnostic emitter.
/// Missing semantic subjects stay unknown, independent of presentation.
fn w123_command(diagnostic: &tcl_compiler::analyser::Diagnostic) -> Option<&str> {
    diagnostic
        .unresolved_command()
        .map(|subject| subject.reporting_name())
}

/// Inclusive count bounds from the formal owner's frozen header metadata.
/// Unknown counts remain fully open. Selected C/Jim grammar and original
/// formal storage are projected before the body-free database boundary.
fn proc_arity(
    count: tcl_compiler::signature_scan::formal_count::SourceFormalCountProjection,
) -> (usize, usize) {
    // naming.database.original-formal-count-header
    // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
    let arity = count.arity();
    let max = if arity.is_unlimited() {
        usize::MAX
    } else {
        usize::from(arity.max)
    };
    (usize::from(arity.min), max)
}

/// Project declaration arities keyed by complete constructed declaration names.
/// Bare-tail entries separately retain legacy unresolved-command assistance.
/// Positioned callbacks use their retained ordered lookup candidates; a qualified
/// callback cannot borrow another namespace's same-tailed declaration.
///
/// A key shared by a proc and a non-proc command carries an empty arity list,
/// because the class, alias or ensemble has no proc signature to validate.
/// The table supplies no reached command, provider or runtime binding proof.
///
/// Depends only on each file's `item_sigs` (the signature firewall), so a body
/// edit anywhere recomputes nothing here.
#[salsa::tracked(returns(clone))]
pub fn project_command_arities(
    db: &dyn TclDb,
    project: Project,
) -> Arc<HashMap<String, Vec<(usize, usize)>>> {
    merge_command_arities(project.files(db).iter().map(|&file| item_sigs(db, file)))
}

fn merge_command_arities(
    signatures: impl IntoIterator<Item = Arc<Vec<ItemSig>>>,
) -> Arc<HashMap<String, Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    // Complete declaration keys and bare-tail assistance remain separate entries.
    let mut acc: HashMap<String, (Vec<(usize, usize)>, bool)> = HashMap::new();
    for signatures in signatures {
        for sig in signatures.iter() {
            // Command-resolvable kinds only — methods are object-dispatched and
            // namespaces aren't commands, so neither suppresses a bare-command W123.
            let resolvable = matches!(
                sig.id.kind,
                ItemKind::Proc | ItemKind::Class | ItemKind::Alias | ItemKind::Ensemble
            );
            if resolvable {
                let tail = tcl_compiler::naming::key_tail(&sig.id.key);
                for name in [sig.id.key.as_str(), tail] {
                    if name.is_empty() {
                        continue;
                    }
                    // A qualified procedure key can draw arity diagnostics only
                    // when its original publication and global lookup select the
                    // same exact retained slot. Bare tails remain assistance.
                    if name == sig.id.key
                        && sig.id.kind == ItemKind::Proc
                        && sig
                            .source_name
                            .as_ref()
                            .and_then(|source| source.source_spelling())
                            .as_deref()
                            != Some(name)
                    {
                        continue;
                    }
                    let entry = acc.entry(name.to_owned()).or_default();
                    if sig.id.kind == ItemKind::Proc {
                        entry.0.push(proc_arity(sig.formal_count));
                    } else {
                        entry.1 = true;
                    }
                }
            }
        }
    }
    // A tail with any non-proc resolver can't be arity-checked (the call may
    // dispatch to the arity-less class/alias/ensemble), so drop its proc arities —
    // it still resolves (suppresses W123) but never draws an arity error.
    let map: HashMap<String, Vec<(usize, usize)>> = acc
        .into_iter()
        .map(|(tail, (arities, has_non_proc))| {
            (tail, if has_non_proc { Vec::new() } else { arities })
        })
        .collect();
    Arc::new(map)
}

/// Body-free original source headers under their exact publication keys.
/// Current source declarations grant no installed command or callback entry.
#[salsa::tracked(returns(clone))]
pub fn project_original_command_signatures(
    db: &dyn TclDb,
    project: Project,
) -> Arc<
    HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
> {
    merge_original_command_signatures(project.files(db).iter().map(|&file| item_sigs(db, file)))
}

fn merge_original_command_signatures(
    signatures: impl IntoIterator<Item = Arc<Vec<ItemSig>>>,
) -> Arc<
    HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
> {
    let mut entries: HashMap<_, Vec<_>> = HashMap::new();
    for signatures in signatures {
        for signature in signatures.iter() {
            let Some(header) = signature.original_declaration.as_ref() else {
                continue;
            };
            let entry = entries.entry(header.name().clone()).or_default();
            if !entry.contains(header) {
                entry.push(header.clone());
            }
        }
    }
    Arc::new(entries)
}

/// Count projection of the shared original source-header query. Non-procedure
/// publications retain lookup barriers without borrowing procedure counts.
#[salsa::tracked(returns(clone))]
pub fn project_original_command_arities(
    db: &dyn TclDb,
    project: Project,
) -> Arc<HashMap<tcl_compiler::signature_scan::scope::SignatureSourceCommand, Vec<(usize, usize)>>>
{
    use tcl_compiler::analyser::ItemKind;
    Arc::new(
        project_original_command_signatures(db, project)
            .iter()
            .map(|(name, headers)| {
                let mut arities = if headers
                    .iter()
                    .any(|header| header.kind() == ItemKind::Class)
                {
                    Vec::new()
                } else {
                    headers
                        .iter()
                        .filter(|header| header.kind() == ItemKind::Proc)
                        .map(|header| proc_arity(header.formal_count_projection()))
                        .collect::<Vec<_>>()
                };
                arities.sort_unstable();
                arities.dedup();
                (name.clone(), arities)
            })
            .collect(),
    )
}

/// Interned exact byte lookup key, without a command-publication grant.
#[salsa::interned]
pub struct OriginalCommandSlot<'db> {
    /// Complete selected name policy, including authored/native authority.
    #[returns(copy)]
    pub policy: tcl_syntax::naming::NamePolicyProtocol,
    /// Exact lookup geometry, without a publication or allocation receipt.
    #[returns(ref)]
    pub slot: tcl_core_types::ByteCommandSlot,
}

/// Per-slot exact source headers retain independent query early cutoff.
/// Display tails and current command occupancy do not enter this key.
#[salsa::tracked(returns(clone))]
pub fn original_command_signatures<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let table = project_original_command_signatures(db, project);
    select_original_command_signatures(db, name, &table)
}

fn select_original_command_signatures(
    db: &dyn TclDb,
    name: OriginalCommandSlot<'_>,
    table: &HashMap<
        tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        Vec<tcl_compiler::analyser::SourceDeclarationSignature>,
    >,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let selected = tcl_compiler::signature_scan::scope::first_matching_byte_publications(
        name.policy(db),
        std::slice::from_ref(name.slot(db)),
        table
            .iter()
            .map(|(publication, headers)| (publication, headers)),
    );
    if selected.is_empty() {
        return None;
    }
    Some(Arc::new(
        selected
            .into_iter()
            .flat_map(|headers| headers.iter().cloned())
            .collect(),
    ))
}

/// Exact current source headers under the original lookup's retained ordered
/// alternatives. This does not select a future implementation or entered frame.
#[must_use]
pub fn original_lookup_command_signatures(
    db: &dyn TclDb,
    project: Project,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    original_lookup_command_signatures_using(db, lookup, |key| {
        original_command_signatures(db, project, key)
    })
}

fn original_lookup_command_signatures_using<'db>(
    db: &'db dyn TclDb,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
    mut query: impl FnMut(
        OriginalCommandSlot<'db>,
    ) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>>,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    let mut slots = Vec::new();
    for candidate in lookup.candidates().iter().flatten() {
        if slots.iter().any(|(slot, _)| slot == candidate) {
            continue;
        }
        let key = OriginalCommandSlot::new(db, lookup.policy(), candidate.clone());
        if let Some(headers) = query(key) {
            slots.push((candidate.clone(), headers));
        }
    }
    let selected = lookup.matching_slot_publications(
        slots
            .iter()
            .map(|(slot, headers)| (slot, lookup.policy(), Arc::clone(headers))),
    )?;
    if selected.is_empty() {
        return None;
    }
    Some(Arc::new(
        selected
            .into_iter()
            .flat_map(|headers| headers.as_ref().clone())
            .collect(),
    ))
}

/// Per-slot original declaration arities. Early cutoff retains independent
/// consumers when an unrelated header changes; UI tails cannot enter the key.
#[salsa::tracked(returns(clone))]
pub fn original_command_arity<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    let table = project_original_command_arities(db, project);
    let matching = tcl_compiler::signature_scan::scope::first_matching_byte_publications(
        name.policy(db),
        std::slice::from_ref(name.slot(db)),
        table
            .iter()
            .map(|(publication, arities)| (publication, arities)),
    );
    if matching.is_empty() {
        return None;
    }
    if matching.iter().any(|arities| arities.is_empty()) {
        return Some(Arc::new(Vec::new()));
    }
    let mut arities = matching
        .into_iter()
        .flat_map(|arities| arities.iter().copied())
        .collect::<Vec<_>>();
    arities.sort_unstable();
    arities.dedup();
    Some(Arc::new(arities))
}

/// Header arity assistance at an authentic original call's retained lookup.
/// Ordered paths and policy come from the shared owner; unknown alternatives
/// cannot be replaced by a bare-tail search. Each demanded slot retains the
/// signature query's early cutoff independently of unrelated declarations.
#[must_use]
pub fn original_lookup_command_arities(
    db: &dyn TclDb,
    project: Project,
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<(usize, usize)>>> {
    let mut slots = Vec::new();
    for candidate in lookup.candidates().iter().flatten() {
        if slots.iter().any(|(slot, _)| slot == candidate) {
            continue;
        }
        let key = OriginalCommandSlot::new(db, lookup.policy(), candidate.clone());
        if let Some(arities) = original_command_arity(db, project, key) {
            slots.push((candidate.clone(), arities));
        }
    }
    let selected = lookup.matching_slot_publications(
        slots
            .iter()
            .map(|(slot, arities)| (slot, lookup.policy(), Arc::clone(arities))),
    )?;
    if selected.is_empty() {
        return None;
    }
    if selected.iter().any(|arities| arities.is_empty()) {
        return Some(Arc::new(Vec::new()));
    }
    let mut arities = selected
        .iter()
        .flat_map(|arities| arities.iter().copied())
        .collect::<Vec<_>>();
    arities.sort_unstable();
    arities.dedup();
    Some(Arc::new(arities))
}

/// Interned declaration key or legacy bare-tail assistance key for the
/// per-symbol cross-file accessor [`command_arity`].
#[salsa::interned]
pub struct CommandTail<'db> {
    #[returns(ref)]
    pub name: String,
}

/// Per-symbol cross-file resolution accessor — the **early-cutoff** point that
/// gives cross-file diagnostics *per-symbol* (not whole-project) invalidation
/// precision.
///
/// Reads the firewalled whole-project [`project_command_arities`] table and
/// projects out one exact declaration or bare-tail assistance key:
/// `Some(arities)` when the workspace contains that key. An empty list is
/// non-proc assistance (class / alias / ensemble), without an arity error. `None`
/// when nothing in the project claims it.
///
/// Why this is its own query: [`project_diagnostics`] for a file demands
/// `command_arity` only for the tails that file actually references, so it depends
/// on **those symbols' resolutions, not the whole table**.  A signature edit to an
/// *unrelated* proc recomputes the aggregate table and re-runs this accessor for
/// each demanded tail — but a tail this file does not call keeps the same
/// projected output, so salsa **early-cutoff backdates** it and the file's
/// `project_diagnostics` does not re-run.  Changing a widely-called utility's
/// signature still re-checks exactly its callers (correct fan-out); changing a
/// proc nobody in file B calls wakes nobody in B.  Proven by
/// `project_diagnostics_per_symbol_cutoff`.
#[salsa::tracked(returns(clone))]
pub fn command_arity<'db>(
    db: &'db dyn TclDb,
    project: Project,
    tail: CommandTail<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    project_command_arities(db, project)
        .get(tail.name(db).as_str())
        .map(|arities| Arc::new(arities.clone()))
}

/// Build the cross-file wrong-argument-count diagnostic for a call to a workspace
/// proc whose arg count fits none of the proc's arities.  Reuses the analyser's
/// **own** arity codes — `E002` (too few) / `E003` (too many), `Severity::Error`,
/// same message shape — so a cross-file arity problem is classified, linked, and
/// disabled exactly like the local one.  (The code `W124` is the *unrelated*
/// invalid-IP-literal warning and must not be reused here.)
///
/// Returns `None` when `argc` sits inside the candidates' `(min, max)` envelope —
/// i.e. it fits some arity, or falls in a rare gap between disjoint same-tail
/// arities, which is too ambiguous to flag.
fn cross_file_arity_diagnostic(
    name: &str,
    span: tcl_lexer::Span,
    argc: (usize, Option<usize>),
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    use tcl_compiler::analyser::types::{Diagnostic, Severity};
    // `argc` is the caller's supplied arg-count RANGE `(lo, hi)`: an ordinary
    // call is exact (`(k, Some(k))`); a command-prefix callback with an
    // `AtLeast(n)` arity is open-ended (`(baked+n, None)`).  Flag too-few only
    // when even the MOST args the caller can supply (`hi`) is below the proc's
    // `min`, and too-many only when even the FEWEST args (`lo`) exceeds `max` —
    // so an open-ended callback never false-fires "too few".
    let (lo, hi) = argc;
    let min = candidates.iter().map(|&(lo, _)| lo).min()?;
    let max = candidates.iter().map(|&(_, hi)| hi).max()?;
    let (code, message) = if hi.is_some_and(|h| h < min) {
        (
            DiagCode::E002,
            format!(
                "Too few arguments for '{name}': expected at least {min}, got {}",
                hi.unwrap_or(lo)
            ),
        )
    } else if max != usize::MAX && lo > max {
        (
            DiagCode::E003,
            format!("Too many arguments for '{name}': expected at most {max}, got {lo}"),
        )
    } else {
        return None;
    };
    Some(Diagnostic {
        subject: None,
        code,
        span,
        message,
        severity: Severity::Error,
        fixes: Vec::new(),
    })
}

/// Diagnose one exact callback argument count when no visible proc candidate
/// accepts it. Unlike the direct-call compatibility helper above, this also
/// handles a genuine gap between disjoint same-tail proc arities: every
/// possible target rejects the count, so E005 is the precise shape error.
fn callback_exact_arity_diagnostic(
    name: &str,
    span: tcl_lexer::Span,
    argc: usize,
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    use tcl_compiler::analyser::types::{Diagnostic, Severity};

    if candidates
        .iter()
        .any(|&(min, max)| min <= argc && argc <= max)
    {
        return None;
    }
    if let Some(diag) = cross_file_arity_diagnostic(name, span, (argc, Some(argc)), candidates) {
        return Some(diag);
    }
    Some(Diagnostic {
        subject: None,
        code: DiagCode::E005,
        span,
        message: format!(
            "Wrong argument count for callback '{name}': no visible definition accepts {argc} arguments"
        ),
        severity: Severity::Error,
        fixes: Vec::new(),
    })
}

/// Legacy standalone project resolution: a W123 (unknown
/// command) whose command tail is a workspace proc is **suppressed** (resolved
/// cross-file); if that call's arg count is known and fits **none** of the
/// proc's arities, a cross-file arity error (`E002`/`E003`, the analyser's own
/// codes) replaces it. The server does not use this direct-call resolver: it
/// settles direct calls once through its workspace-index oracle. `arities`
/// empty ⇒ no project context ⇒ status-quo diagnostics.
///
/// This compatibility projection requires independently retained Logical input.
/// Native, hosted and missing inputs preserve diagnostics without tail advice.
/// `unresolved_sites` are the call sites of unknown commands
/// ([`AnalysisResult::unresolved_command_sites`]), recorded by the analyser
/// **regardless of whether W123 is disabled** — so cross-file arity is independent
/// of the W123 toggle (matching local arity), since the arity check keys off these
/// rather than the (possibly filtered) W123 diagnostic.
///
/// `is_disabled` reads the analyser's `disabled_diagnostics` for the
/// synthesised arity code: the synthesised arity codes honour the same
/// production skip as the analyser's own; whether they show is the policy
/// step's decision.
#[must_use]
pub fn apply_cross_file_resolution<S: std::hash::BuildHasher>(
    analysis: &AnalysisResult,
    diags: &[tcl_compiler::analyser::types::Diagnostic],
    unresolved_sites: &[(tcl_lexer::Span, String)],
    invocations: &[tcl_compiler::signature_scan::types::SignatureCommandInvocation],
    arities: &HashMap<String, Vec<(usize, usize)>, S>,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
    if !analysis.allows_retained_logical_declaration_advice() || arities.is_empty() {
        return diags.to_vec();
    }
    // Suppress every W123 that resolves cross-file (its tail is a workspace
    // command); a genuinely-unknown command's W123 is kept.
    let mut out: Vec<tcl_compiler::analyser::types::Diagnostic> = diags
        .iter()
        .filter(|d| {
            d.code != DiagCode::W123
                || !w123_command(d).is_some_and(|name| arities.contains_key(name))
        })
        .cloned()
        .collect();
    // Cross-file arity: for each unresolved call site that resolves to a workspace
    // **proc** (non-empty arity list) with a known arg count fitting no arity, emit
    // E002/E003 — unless that code is disabled.  Driven off the toggle-independent
    // `unresolved_sites`, so disabling W123 does not also silence arity.
    let argc_by_span: HashMap<(u32, u32), Option<usize>> = invocations
        .iter()
        .map(|inv| ((inv.range.start(), inv.range.end()), inv.argc))
        .collect();
    for (span, name) in unresolved_sites {
        if let Some(candidates) = arities.get(name)
            && !candidates.is_empty()
            && let Some(Some(argc)) = argc_by_span.get(&(span.start(), span.end()))
            && let Some(diag) =
                cross_file_arity_diagnostic(name, *span, (*argc, Some(*argc)), candidates)
            && !is_disabled(diag.code.as_str())
        {
            out.push(diag);
        }
    }
    apply_callback_arity(&mut out, invocations, arities, &is_disabled);
    out
}

/// Legacy callback count advice for independently retained Logical input.
/// Native, hosted and missing inputs preserve the supplied diagnostics.
/// Authentic source callbacks use [`project_callback_diagnostics_for_analysis`].
///
/// Direct cross-file command existence and direct-call arity are settled by
/// the server's workspace-index oracle. Keeping this helper limited to
/// callback metadata lets the opt-in project pass retain that independent
/// check without competing for direct-call verdicts.
///
/// `is_disabled` reads the analyser's `disabled_diagnostics`: the synthesised
/// arity codes honour the same production skip as the analyser's own; whether
/// they show is the policy step's decision.
#[must_use]
pub fn apply_project_callback_arity<S: std::hash::BuildHasher>(
    analysis: &AnalysisResult,
    diags: &[tcl_compiler::analyser::types::Diagnostic],
    invocations: &[tcl_compiler::signature_scan::types::SignatureCommandInvocation],
    arities: &HashMap<String, Vec<(usize, usize)>, S>,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
    let mut out = diags.to_vec();
    if analysis.allows_retained_logical_declaration_advice() {
        apply_callback_arity(&mut out, invocations, arities, is_disabled);
    }
    out
}

/// Validate command-prefix **callback** arity: for each recorded callback head
/// (`lsort -command myCompare` → `myCompare` with `callback_arity =
/// Exactly(2)`), check that the referenced project proc accepts the arguments
/// the calling command appends.
///
/// The effective arg count is `baked` args already in the prefix (`{myCmp
/// extra}` bakes one; a bare word bakes zero) plus the command's appended
/// arity. `Exactly(n)` checks one count; `OneOf(set)` requires the proc to
/// accept every exact alternative; `AtLeast(n)` retains the conservative open
/// range. Skipped for `Unknown`/absent arities and for tails the project
/// resolves with a non-proc (empty candidate list).
/// Same-file callbacks are covered too: `project_command_arities` aggregates
/// every file, so a same-file proc's arity is in `arities`.
fn apply_callback_arity<S: std::hash::BuildHasher>(
    out: &mut Vec<tcl_compiler::analyser::types::Diagnostic>,
    invocations: &[tcl_compiler::signature_scan::types::SignatureCommandInvocation],
    arities: &HashMap<String, Vec<(usize, usize)>, S>,
    is_disabled: impl Fn(&str) -> bool,
) {
    for inv in invocations {
        let Some(appended) = inv.callback_arity else {
            continue;
        };
        if !appended.is_checkable() {
            continue;
        }
        let Some(candidates) = callback_command_keys(inv)
            .iter()
            .find_map(|name| arities.get(name))
        else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }
        if let Some(diag) = callback_arity_diagnostic(inv, candidates)
            && !is_disabled(diag.code.as_str())
        {
            out.push(diag);
        }
    }
}

fn callback_arity_diagnostic(
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    let appended = invocation.callback_arity?;
    if !appended.is_checkable() || candidates.is_empty() {
        return None;
    }
    let baked = invocation.callback_baked_args;
    if let Some(exact_counts) = appended.exact_counts() {
        exact_counts
            .map(|count| baked + usize::from(count))
            .find_map(|count| {
                callback_exact_arity_diagnostic(
                    &invocation.name,
                    invocation.range,
                    count,
                    candidates,
                )
            })
    } else {
        let lo = baked + appended.min() as usize;
        let hi = appended.max().map(|max| baked + max as usize);
        cross_file_arity_diagnostic(&invocation.name, invocation.range, (lo, hi), candidates)
    }
}

#[derive(Clone, Copy)]
enum ProjectSourceInputMode {
    Standalone,
    Supplied,
}

/// Local canonical signatures do not read the project table. Authenticated
/// external source names and unknown local targets use exact byte geometry;
/// explicit source refusals never borrow a surviving project declaration.
fn callback_source_target_signatures(
    db: &dyn TclDb,
    project: Project,
    selection: &tcl_compiler::analyser::SourceCallbackSignatureLookup,
    mode: ProjectSourceInputMode,
) -> Option<Arc<Vec<tcl_compiler::analyser::SourceDeclarationSignature>>> {
    use tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind as Kind;
    let original = selection.original();
    if let Some(target) = original.target() {
        match target.kind() {
            Kind::LocalProcedure => Some(Arc::new(Vec::new())),
            Kind::ExternalSourceName => {
                let key = OriginalCommandSlot::new(
                    db,
                    target.target_input().native_input()?.policy(),
                    target.source_slot().clone(),
                );
                match mode {
                    ProjectSourceInputMode::Standalone => {
                        original_command_signatures(db, project, key)
                    }
                    ProjectSourceInputMode::Supplied => {
                        original_command_signatures_for_inputs(db, project, key)
                    }
                }
            }
        }
    } else if original.permits_external_signature_lookup() {
        match mode {
            ProjectSourceInputMode::Standalone => {
                original_lookup_command_signatures(db, project, selection.prefix().lookup()?)
            }
            ProjectSourceInputMode::Supplied => original_lookup_command_signatures_for_inputs(
                db,
                project,
                selection.prefix().lookup()?,
            ),
        }
    } else {
        None
    }
}

fn apply_original_callback_arity<'a>(
    db: &dyn TclDb,
    project: Project,
    out: &mut Vec<tcl_compiler::analyser::types::Diagnostic>,
    invocations: impl IntoIterator<
        Item = &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    >,
    is_disabled: impl Fn(&str) -> bool,
) {
    apply_original_callback_arity_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        out,
        invocations,
        is_disabled,
    );
}

fn apply_original_callback_arity_with_mode<'a>(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    out: &mut Vec<tcl_compiler::analyser::types::Diagnostic>,
    invocations: impl IntoIterator<
        Item = &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation,
    >,
    is_disabled: impl Fn(&str) -> bool,
) {
    let (db, project, mode) = source;
    use tcl_compiler::analyser::{
        Diagnostic, DiagnosticSubject, Severity, SourceCallbackArityIssue as Issue,
        SourceCallbackAritySubject,
    };
    for invocation in invocations {
        let Some(selection) = invocation.original_callback_signature_lookup.as_ref() else {
            // A prefix's scope geometry alone does not own source occupancy.
            continue;
        };
        let Some(headers) = callback_source_target_signatures(db, project, selection, mode) else {
            continue;
        };
        let Some(subject) =
            SourceCallbackAritySubject::from_source_lookup(Arc::clone(selection), &headers)
        else {
            continue;
        };
        let issue = subject.issue();
        let code = issue.code();
        if is_disabled(code.as_str()) {
            continue;
        }
        // Presentation is independent of exact retained name bytes and policy.
        let name = String::from_utf8_lossy(subject.prefix().name_input().bytes());
        let message = match issue {
            Issue::TooFew {
                supplied,
                expected_minimum,
            } => format!(
                "Too few arguments for '{name}': expected at least {expected_minimum}, got {supplied}"
            ),
            Issue::TooMany {
                supplied,
                expected_maximum,
            } => format!(
                "Too many arguments for '{name}': expected at most {expected_maximum}, got {supplied}"
            ),
            Issue::NoCompatibleSignature { supplied } => format!(
                "Wrong argument count for callback '{name}': no visible definition accepts {supplied} arguments"
            ),
        };
        out.push(
            Diagnostic::new(code, subject.span(), message, Severity::Error)
                .with_subject(DiagnosticSubject::CallbackSourceArity(Arc::new(subject))),
        );
    }
}

/// Qualified callback lookup keeps its complete retained candidate keys. Bare
/// tails remain assistance only for legacy callback records without a lookup.
fn callback_command_keys(
    invocation: &tcl_compiler::signature_scan::types::SignatureCommandInvocation,
) -> Vec<String> {
    // An original callback producer requires its independently selected future
    // lookup purpose. Reporting candidates cannot replace a missing receipt.
    if invocation.original_callback_signature_lookup.is_some()
        || invocation.original_callback_prefix.is_some()
        || invocation.original_name_input.is_some()
    {
        return Vec::new();
    }

    if let Some(reference) = &invocation.resolved_command_reference {
        return reference.slot().map(str::to_owned).into_iter().collect();
    }
    if !invocation.resolution_candidates.is_empty() {
        return invocation.resolution_candidates.clone();
    }
    if invocation.name.contains("::") {
        return invocation
            .name
            .starts_with("::")
            .then(|| tcl_compiler::naming::canonical_written_command(&invocation.name))
            .into_iter()
            .collect();
    }
    vec![invocation.name.clone()]
}

/// Legacy standalone analyser diagnostics for `file` resolved against the
/// project:
/// cross-file-resolvable W123 (unknown command) suppressed, plus a
/// cross-file arity error (`E002`/`E003`) for calls to workspace procs with a bad
/// arg count. Server diagnostics use the workspace index for direct calls;
/// this query remains for the database corpus and incremental-resolution tests.
///
/// Deliberately a **separate query off the paramount [`file_analysis`] path**:
/// `file_analysis` stays project-independent (so a signature change in another
/// file cannot regress this file's time-to-first-tokens), and this
/// debounced/non-paramount query layers cross-file resolution on top.  It depends
/// only on `file_analysis(file, config)` + the firewalled [`project_command_arities`],
/// so a **body** edit in any file recomputes nothing here; only a proc-decl /
/// signature change (or this file's own edit) does — precise cross-file
/// reverse-dependency invalidation, for free, from salsa.
///
/// Cross-file arity is **independent of the W123 toggle**: it keys off
/// [`AnalysisResult::unresolved_command_sites`], which the analyser records even
/// when W123 is disabled, so a wrong-arg cross-file call still reports `E002`/`E003`
/// (matching local arity) regardless of the user's W123 setting.
#[salsa::tracked(returns(clone))]
pub fn project_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    // The analyser's production skip: the synthesised arity codes honour the
    // same production skip as the analyser's own; whether they show is the
    // policy step's decision.
    let disabled = config.disabled_diagnostics(db);
    // `file_analysis_incremental` (not the coarse `file_analysis`) so this reuses
    // the per-item firewall result the diagnostics worker already computed for
    // `file` — a cache hit, not a second whole-file analysis.
    let analysis = file_analysis_incremental(db, file, config);

    // Per-symbol demand (the precision lever): resolve only the command tails this
    // file actually references — every unknown-command (W123) tail and every
    // recorded unresolved call site — through the early-cutoff `command_arity`
    // accessor, building the same `tail -> arities` map shape
    // `apply_cross_file_resolution` reads.  Depending on *those symbols'*
    // resolutions rather than the whole `project_command_arities` table is what
    // stops an unrelated proc's signature edit from re-running this file's
    // cross-file diagnostics (see `command_arity`).
    let unresolved = analysis
        .unresolved_command_sites
        .iter()
        .map(|(span, _)| (span.start(), span.end()))
        .collect::<std::collections::HashSet<_>>();
    let original_sites = analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            invocation.original_name_input.is_some()
                && unresolved.contains(&(invocation.range.start(), invocation.range.end()))
        })
        .map(|invocation| {
            (
                (invocation.range.start(), invocation.range.end()),
                invocation
                    .original_lookup
                    .as_ref()
                    .and_then(|lookup| original_lookup_command_arities(db, project, lookup)),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut tails: BTreeSet<String> = BTreeSet::new();
    for diag in &analysis.diagnostics {
        if analysis.allows_retained_logical_declaration_advice()
            && diag.code == DiagCode::W123
            && !original_sites.contains_key(&(diag.span.start(), diag.span.end()))
            && let Some(name) = w123_command(diag)
        {
            tails.insert(name.to_owned());
        }
    }
    for (span, name) in &analysis.unresolved_command_sites {
        if analysis.allows_retained_logical_declaration_advice()
            && !original_sites.contains_key(&(span.start(), span.end()))
        {
            tails.insert(name.clone());
        }
    }
    // Command-prefix callback heads (`lsort -command myCompare`) resolve (they
    // are not W123/unresolved), so their target proc's arity would not be
    // loaded — pull each callback tail in so `apply_callback_arity` can check it.
    for inv in &analysis.command_invocations {
        if analysis.allows_retained_logical_declaration_advice() && inv.callback_arity.is_some() {
            tails.extend(callback_command_keys(inv));
        }
    }
    let mut arities: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    for tail in tails {
        if let Some(resolved) = command_arity(db, project, CommandTail::new(db, tail.clone())) {
            arities.insert(tail, (*resolved).clone());
        }
    }

    let legacy_diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code != DiagCode::W123
                || !original_sites.contains_key(&(diagnostic.span.start(), diagnostic.span.end()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let legacy_sites = analysis
        .unresolved_command_sites
        .iter()
        .filter(|(span, _)| !original_sites.contains_key(&(span.start(), span.end())))
        .cloned()
        .collect::<Vec<_>>();
    let mut diagnostics = apply_cross_file_resolution(
        &analysis,
        &legacy_diagnostics,
        &legacy_sites,
        &analysis.command_invocations,
        &arities,
        |code| disabled.iter().any(|c| c == code),
    );
    diagnostics.extend(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == DiagCode::W123
                    && original_sites
                        .get(&(diagnostic.span.start(), diagnostic.span.end()))
                        .is_some_and(Option::is_none)
            })
            .cloned(),
    );
    for invocation in &analysis.command_invocations {
        let Some(Some(candidates)) =
            original_sites.get(&(invocation.range.start(), invocation.range.end()))
        else {
            continue;
        };
        let Some(argc) = invocation.argc else {
            continue;
        };
        if !candidates.is_empty()
            && let Some(diagnostic) = cross_file_arity_diagnostic(
                &invocation.name,
                invocation.range,
                (argc, Some(argc)),
                candidates,
            )
            && !disabled.iter().any(|code| code == diagnostic.code.as_str())
        {
            diagnostics.push(diagnostic);
        }
    }
    apply_original_callback_arity(
        db,
        project,
        &mut diagnostics,
        retained_callback_invocations(&analysis),
        |code| disabled.iter().any(|disabled| disabled == code),
    );
    Arc::new(diagnostics)
}

/// Original callback rows that still belong to this analysis's complete input.
/// Reissuing the existing lower-owner join also verifies its local canonical
/// declaration, independently of mutable invocation labels and count fields.
fn retained_callback_invocations(
    analysis: &AnalysisResult,
) -> impl Iterator<Item = &tcl_compiler::signature_scan::types::SignatureCommandInvocation> {
    analysis.command_invocations.iter().filter(|invocation| {
        let Some(selection) = invocation.original_callback_signature_lookup.as_ref() else {
            return false;
        };
        tcl_compiler::analyser::SourceCallbackSignatureLookup::from_original_lookup(
            analysis,
            Arc::new(selection.prefix().clone()),
            Arc::new(selection.original().clone()),
        )
        .as_ref()
            == Some(selection.as_ref())
    })
}

/// Add project callback signature advice to an already analysed complete source.
/// The source channel, lexer configuration and retained input must agree;
/// foreign or missing ownership preserves the supplied analyser diagnostics.
/// Known source barriers and missing registration horizons never borrow tails.
/// Local canonical headers and exact held external names share the original
/// callback owner. Legacy scalar records require positive retained Logical input.
/// This supplies no installed callback, future entry or execution verdict.
/// Project headers in this compatibility entry use the explicit standalone
/// provider. Document callers use [`project_callback_diagnostics_for_analysis_with_inputs`].
#[must_use]
pub fn project_callback_diagnostics_for_analysis(
    db: &dyn TclDb,
    project: Project,
    source: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    project_callback_diagnostics_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        source,
        analysis,
        is_disabled,
    )
}

fn project_callback_diagnostics_with_mode(
    project_input: (&dyn TclDb, Project, ProjectSourceInputMode),
    source: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let (db, project, mode) = project_input;
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    let Some(input) = analysis.resolved_input.as_ref() else {
        return analysis.diagnostics.clone();
    };
    if !analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        input.lexer_config(),
    ) {
        return analysis.diagnostics.clone();
    }
    let mut arities = HashMap::new();
    if analysis.allows_retained_logical_declaration_advice() {
        let tails = analysis
            .command_invocations
            .iter()
            .filter(|invocation| invocation.callback_arity.is_some())
            .flat_map(callback_command_keys)
            .collect::<BTreeSet<_>>();
        for tail in tails {
            let key = CommandTail::new(db, tail.clone());
            let resolved = match mode {
                ProjectSourceInputMode::Standalone => command_arity(db, project, key),
                ProjectSourceInputMode::Supplied => command_arity_for_inputs(db, project, key),
            };
            if let Some(resolved) = resolved {
                arities.insert(tail, (*resolved).clone());
            }
        }
    }
    let mut diagnostics = apply_project_callback_arity(
        analysis,
        &analysis.diagnostics,
        &analysis.command_invocations,
        &arities,
        &is_disabled,
    );
    apply_original_callback_arity_with_mode(
        (db, project, mode),
        &mut diagnostics,
        retained_callback_invocations(analysis),
        is_disabled,
    );
    diagnostics
}

/// Opt-in callback signature diagnostics from the same supplied-analysis owner.
/// Direct calls retain the server's separate workspace-index settlement.
#[salsa::tracked(returns(clone))]
pub fn project_callback_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    let disabled = config.disabled_diagnostics(db);
    let analysis = file_analysis_incremental(db, file, config);
    Arc::new(project_callback_diagnostics_for_analysis(
        db,
        project,
        file.text(db),
        &analysis,
        |code| disabled.iter().any(|disabled| disabled == code),
    ))
}

/// Interned identity of a single `proc` body's isolated analysis — the per-item
/// firewall's memoisation key.  **Offset-invariant**: it holds only what the
/// offset-0 analysis consumes (body text + enclosing namespace / name / params +
/// config), *not* the body's position — so a shifted-but-unedited proc has the
/// same key and reuses the cached [`item_body_analysis`] (the aggregator rebases
/// the offset-0 facts by the body's real span).
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `body_text` changes on every keystroke inside the body, so each edit mints a
/// fresh id here.  What keeps that bounded is the collector reusing the LRU
/// tail's stale slots (`clear_memos` releasing the retained analysis `Arc`s),
/// which happens only for `Durability::LOW` slots interned inside a tracked
/// query.  See the crate docs' "The interned garbage collector is
/// load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct ItemBodyKey<'db> {
    /// The proc / method body source, shared with the
    /// [`tcl_compiler::analyser::per_item::DeferredBody`] the aggregator built
    /// it from rather than copied out of it.  Interning is content-addressed, so
    /// the text must be *in* the key; what it must not be is a fresh `String`
    /// per proc per edit — on a 114-proc file that is 114 full body copies
    /// before a single memo is consulted, with the copies discarded the
    /// moment the intern hits.  `Arc<str>` also makes
    /// [`item_body_analysis`]'s rebuild of the `DeferredBody` a refcount bump.
    #[returns(ref)]
    pub body_text: Arc<str>,
    #[returns(ref)]
    pub namespace: String,
    #[returns(ref)]
    pub scope_name: String,
    #[returns(ref)]
    pub params: Vec<ParamDef>,
    /// `true` for a `TclOO` method body (isolated in a `Method` scope with
    /// instance variables pre-bound); `false` for a `proc`.
    #[returns(copy)]
    pub is_method: bool,
    /// Mirrors `Scope::oo_global_resolution`: `true` for a TclOO method
    /// body (bare commands resolve globally — object-namespace semantics),
    /// `false` for procs and snit / itcl members.
    #[returns(copy)]
    pub oo_global_resolution: bool,
    /// Variables pre-bound in the body: a method body's class instance
    /// variables, or a procedure's static variables.
    #[returns(ref)]
    pub seeded_variables: Vec<String>,
    /// The constant command-substitution fold context: the
    /// whole-file command-mutation trust snapshot the shell attached for a
    /// body with a fold candidate, paired with the instance-side `TclOO`
    /// defining class (`[self class]`'s answer) when the body is such a
    /// method — mirrors
    /// [`tcl_compiler::analyser::per_item::DeferredBody::command_trust`] /
    /// `oo_defining_class`. One field because the class fact is only ever
    /// consumed under a trust snapshot (no snapshot ⇒ the fold distrusts
    /// everything and never reads the class). Part of the key so a `rename`
    /// appearing (or disappearing) anywhere in the file re-analyses exactly
    /// the bodies whose folds it could affect; `None` (no candidate) keeps
    /// the key untouched by unrelated renames.
    #[returns(ref)]
    pub fold_ctx: Option<(
        Arc<tcl_compiler::command_binding::CommandTrustSnapshot>,
        Option<String>,
    )>,
    /// The body's enclosing-environment snapshot, several halves in one field
    /// (salsa interns the field tuple, whose `Hash` impl caps at 12
    /// elements — and this struct is already at that cap, which is why these
    /// travel packed rather than as fields of their own):
    ///
    /// - `.0` — the ensemble environment, itself a pair that must stay a
    ///   pair:
    ///   - `.0.0` — the ensemble `subcommand → target` maps visible to this
    ///     body, mirroring
    ///     [`tcl_compiler::analyser::per_item::DeferredBody::ensemble_targets`]
    ///     (already canonically sorted and filtered to ensembles the body
    ///     mentions, so an unrelated ensemble edit re-keys no body). Part of
    ///     the key so an `<ensemble> <sub> …` call inside the body
    ///     re-analyses when the mapping it resolves through changes.
    ///   - `.0.1` — the `-prefixes 0` opt-out for those same ensembles,
    ///     mirroring
    ///     [`tcl_compiler::analyser::per_item::DeferredBody::prefixless_ensembles`].
    ///     **Bound to `.0.0` by type on purpose.**
    ///     `AnalysisResult::resolve_ensemble_subcommand` reads the two
    ///     together — the map says which subcommands exist, this says
    ///     whether an abbreviation of one may match — so seeding the map
    ///     alone lets an isolated body resolve `g fo` against a
    ///     `-prefixes 0` ensemble and record a dispatch the whole-file walk
    ///     (and real Tcl) refuse. This query *resolves* the body rather than
    ///     replaying a recorded answer, so the salsa altitude needs the
    ///     opt-out exactly as much as the whole-file walk does; the nested
    ///     pair is what stops a future edit from cloning one without the
    ///     other.
    /// - `.1` — the complete original conditional child-visibility receipt,
    ///   mirroring
    ///   [`tcl_compiler::analyser::per_item::DeferredBody::safe_interp_ctx`].
    ///   Its hash retains full source/configuration, immutable editing input,
    ///   scoped visible slots and independent hidden-token relationships.
    ///   It never supplies selected callability or native hidden-table identity.
    /// - `.2` — the workspace **class factory** oracle
    ///   ([`SourceFile::workspace_class_factories`]). A
    ///   `Meta create …` inside a proc body is classified by the whole-file
    ///   walk from this index, so the isolated body pass must see the same
    ///   one or the two strategies disagree about whether a class exists.
    ///   Part of the key for the usual reason: a metaclass appearing or
    ///   changing shape elsewhere in the workspace must re-analyse the
    ///   bodies whose creation calls it decides.
    #[returns(ref)]
    pub body_env: (
        (
            Vec<(
                String,
                Vec<(
                    String,
                    tcl_compiler::analyser::types::EnsembleSubcommandTarget,
                )>,
            )>,
            Vec<String>,
        ),
        Option<tcl_compiler::analyser::SourceInterpreterVisibilitySnapshot>,
        Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
        Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
    ),
    #[returns(ref)]
    pub dialect: String,
    #[returns(ref)]
    pub disabled: Vec<String>,
    #[returns(copy)]
    pub non_ascii: NonAsciiMode,
}

/// Memoised offset-0 isolated analysis of one `proc` body.  A body-only edit
/// changes only that body's [`ItemBodyKey`], so salsa reuses every other body's
/// result; an edit that merely *shifts* a body leaves its key unchanged.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn item_body_analysis<'db>(db: &'db dyn TclDb, key: ItemBodyKey<'db>) -> Arc<BodyFragment> {
    // The isolated analysis works at offset 0 and ignores `body_tok` / scope
    // path (the aggregator supplies the real position when grafting), so a
    // placeholder token is fine.
    let (command_trust, oo_defining_class) = match key.fold_ctx(db) {
        Some((trust, class)) => (Some(Arc::clone(trust)), class.clone()),
        None => (None, None),
    };
    let body = DeferredBody {
        resolved_input: key.body_env(db).3.clone(),
        body_text: Arc::clone(key.body_text(db)),
        body_tok: tcl_lexer::Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, 0)),
        scope_path: Vec::new(),
        is_method: key.is_method(db),
        oo_global_resolution: key.oo_global_resolution(db),
        namespace: key.namespace(db).clone(),
        scope_name: key.scope_name(db).clone(),
        params: key.params(db).clone(),
        seeded_variables: key.seeded_variables(db).clone(),
        command_trust,
        oo_defining_class,
        // The two ensemble halves are seeded together, never separately —
        // see `ItemBodyKey::body_env`.
        ensemble_targets: key.body_env(db).0.0.clone(),
        prefixless_ensembles: key.body_env(db).0.1.clone(),
        safe_interp_ctx: key.body_env(db).1.clone(),
    };
    let disabled: HashSet<String> = key.disabled(db).iter().cloned().collect();
    let no_declarations = tcl_registry::model::DeclaredSurface::new();
    Arc::new(analyse_proc_body_isolated(
        &body,
        key.dialect(db),
        &disabled,
        key.non_ascii(db),
        Some(no_declarations),
        key.body_env(db).2.clone(),
    ))
}

/// Interned module-wide context (`upvar_procs` + `proc_params` +
/// `global_write_procs` from `prepare_cfg_context`) that a procedure body's
/// CFG is built under. Interned once per build and shared by every
/// [`FnLatticeKey`] so a procedure's key stays small and the per-build
/// interning cost is `O(procs)`, not `O(procs²)`. The entry vecs are sorted
/// by name before interning so an equal context (regardless of hash-map
/// iteration order) yields the same id.
#[salsa::interned]
pub struct CfgContext<'db> {
    #[returns(ref)]
    pub upvar_ctx: Vec<(String, UpvarInfo)>,
    #[returns(ref)]
    pub proc_params: Vec<(String, Vec<String>)>,
    #[returns(ref)]
    pub global_write_ctx: Vec<(String, GlobalWriteInfo)>,
    #[returns(ref)]
    pub command_bindings: ModuleCommandBindings,
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
    /// Trace-visible mode preserves ordinary dynamically replaceable dispatch.
    pub plain_command_dispatch: bool,
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
    pub body: Script,
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
    let upvar: HashMap<String, UpvarInfo> = context.upvar_ctx(db).iter().cloned().collect();
    let proc_params: HashMap<String, Vec<String>> =
        context.proc_params(db).iter().cloned().collect();
    let global_write_procs: HashMap<String, GlobalWriteInfo> =
        context.global_write_ctx(db).iter().cloned().collect();
    let registry = key.snapshot(db).registry.registry();
    // The request's exact grammar, not one reconstructed from the registry or
    // environment name: grammar overrides are part of the memo identity.
    let config = key.lexer_config(db);
    let prepared = PreparedCfgContext::from_source_input(
        (
            upvar,
            proc_params,
            global_write_procs,
            context.command_bindings(db).clone(),
        ),
        registry,
        key.entry(db).source_metadata_input.as_ref(),
    );
    let cfg = build_cfg_function_with_prepared_context(
        key.qname(db),
        key.body(db),
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
    Arc::new(
        FunctionUnit::build_for_lattice(
            key.qname(db),
            cfg,
            key.params(db),
            tcl_compiler::compilation_unit::FunctionLatticeInputs {
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
                    .map(|event| (event, key.body(db))),
            },
        )
        .with_retained_semantic_analysis(
            registry,
            Some(key.body(db)),
            // A procedure body runs only after arbitrary interposed history,
            // so its dispatch proofs start from an unknown world.
            tcl_compiler::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        ),
    )
}

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
    let registry = key.snapshot(db).registry.registry();
    let profile = key.snapshot(db).profile.map(|snapshot| snapshot.profile());
    let config = key.lexer_config(db);
    Arc::new(tcl_compiler::lowering::lower_proc_body_isolated(
        key.body_text(db),
        key.namespace(db),
        &registry,
        config,
        profile,
    ))
}

/// Build a `CompilationUnit` (with interprocedural summary applied) whose
/// per-procedure baseline lattices are memoised by the salsa-native
/// [`function_lattice`] query.
///
/// Shared by the analyser's CFG/SSA diagnostic tail
/// ([`file_analysis_incremental`]) and the optimiser's compiler-checks pass
/// ([`compiler_check_diagnostics`]) so an unchanged procedure's lattice is built
/// once and reused (rebased to its new offset) across edits *and* across both
/// consumers' passes — and garbage-collected by salsa, not a process-wide
/// content cache.  Byte-identical to
/// [`CompilationUnit::build_for_with_config`] `+ with_interprocedural`.
///
/// The two consumers lower with different [`tcl_lexer::LexerConfig`]s (which can
/// change a `{*}`/`}{` body's IR), so the same procedure can intern to two
/// different bodies; because the **post-lowering body is part of the key**, the
/// two never cross-pollute — no explicit namespace is needed.
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
    source: &str,
    options: UnitBuildOptions<'_>,
) -> CompilationUnit {
    build_unit_with_keys(db, source, options).0
}

/// `qname`'s offset-0 lattice key, when the unit reads its lattice from the
/// memo: a lattice that read another procedure of the module was built
/// afresh for the unit (`SccpResult::reads_module`), and every reader of the
/// memo — the checks, the rewrites, the taint cascade — takes the unit's.
fn memo_key<'db>(
    db: &'db dyn TclDb,
    keys: &HashMap<String, FnLatticeKey<'db>>,
    qname: &str,
) -> Option<FnLatticeKey<'db>> {
    keys.get(qname)
        .copied()
        .filter(|&key| !function_lattice(db, key).sccp.reads_module)
}

/// `memoised_compilation_unit` that also returns the per-procedure
/// [`FnLatticeKey`] map built during lowering (qname → offset-0 baseline key).
///
/// [`proc_taint_solve`] needs those keys to memoise the interprocedural summary
/// fixpoint per procedure ([`proc_summary_cascade`]), but they **cannot be
/// threaded out of the shared [`compilation_unit`] query**: a salsa tracked
/// return must be `'static`, and `FnLatticeKey<'db>` is `'db`-interned (the
/// finished `CompilationUnit` keeps only the rebased `FunctionUnit`s, not the
/// offset-0 bodies the keys are built from).  So the checks path re-derives them
/// with this second build — whose per-procedure lattice/cascade demands hit the
/// **same** [`function_lattice`] / [`taint_cascade`] memos the shared build
/// already populated, making the duplicate build mostly cache hits (~29 ms warm
/// vs ~57 ms cold, measured on `linalg.tcl`).
fn build_unit_with_keys<'db>(
    db: &'db dyn TclDb,
    source: &str,
    options: UnitBuildOptions<'_>,
) -> (CompilationUnit, HashMap<String, FnLatticeKey<'db>>) {
    build_unit_with_keys_and_input(db, source, options, None)
}

fn build_unit_with_keys_and_input<'db>(
    db: &'db dyn TclDb,
    source: &str,
    options: UnitBuildOptions<'_>,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> (CompilationUnit, HashMap<String, FnLatticeKey<'db>>) {
    let UnitBuildOptions { registry, .. } = options;
    let dialect = options.dialect;
    // Values, rather than an environment-name round trip, own memo identity.
    let dialect_key = dialect.map_or("", |profile| profile.name);
    let dialect_opt = dialect;
    let profile_key = dialect.map(tcl_dialect::DialectProfile::cache_key);
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
    let mut lattice_keys: HashMap<String, FnLatticeKey<'db>> = HashMap::new();
    let mut lattice_memo = |req: &LatticeRequest<'_>| -> FunctionUnit {
        let context = *context.get_or_insert_with(|| {
            let mut upvar: Vec<(String, UpvarInfo)> = req
                .upvar_procs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            upvar.sort_by(|a, b| a.0.cmp(&b.0));
            let mut proc_params: Vec<(String, Vec<String>)> = req
                .proc_params
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            proc_params.sort_by(|a, b| a.0.cmp(&b.0));
            let mut global_write: Vec<(String, GlobalWriteInfo)> = req
                .global_write_procs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            global_write.sort_by(|a, b| a.0.cmp(&b.0));
            CfgContext::new(
                db,
                upvar,
                proc_params,
                global_write,
                req.command_bindings.clone(),
            )
        });
        let template = req.body_source.and_then(|body_source| {
            req.command_bindings.prepare_native_body_template(
                req.body,
                tcl_compiler::command_binding::BodyProofScope {
                    source: req.source,
                    body_source,
                    original_body_offset: req.original_body_offset,
                    executable_body_offset: req.executable_body_offset,
                },
                registry,
            )
        });
        if let Some(template) = template {
            let template_context = CfgContext::new(
                db,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                template.command_bindings,
            );
            let key = lattice_request_key(
                db,
                req,
                template.body,
                template_context,
                &registry_snapshot,
                epoch,
            );
            let proofs = tcl_compiler::command_binding::BodySourceProofs::from_body(req.body);
            let unit = template
                .variable_relocation
                .inverse()
                .and_then(|inverse| function_lattice(db, key).relocated_variable_proofs(&inverse))
                .and_then(|unit| unit.restored_source_proofs(&proofs));
            if let Some(unit) = unit {
                lattice_keys.insert(req.qname.to_owned(), key);
                return unit;
            }
        }
        // Exact full-world identity is the fallback when template admission or
        // restoration cannot establish equivalence. No partially relocated
        // artifact crosses this boundary.
        let key = lattice_request_key(
            db,
            req,
            req.body.clone(),
            context,
            &registry_snapshot,
            epoch,
        );
        lattice_keys.insert(req.qname.to_owned(), key);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_FUNCTION_LATTICE").is_some() {
            eprintln!(
                "ORIGINAL_FUNCTION_LATTICE qname={} event={} untouched={}",
                req.qname,
                req.irules_event_body.is_some(),
                req.command_trust.agrees_with_untouched_bindings()
            );
        }
        (*function_lattice(db, key)).clone()
    };
    // The binding owner proves isolated-body equivalence against each actual
    // entry state. Cache hits restore positioned carriers after rebasing, so
    // namespace, activation and nested-dispatch proofs remain current.
    let cu = {
        // Same offset-0-plus-rebase contract as `lattice_memo` above: the caller
        // shifts the returned `Script` to the body's real position, so it needs
        // an owned copy.
        let body_memo = |body_text: &str, namespace: &str| -> Script {
            let key = ProcBodyKey::new(
                db,
                body_text.to_owned(),
                namespace.to_owned(),
                dialect_key.to_owned(),
                CompilerMemoSnapshot {
                    profile: profile_key,
                    registry: registry_snapshot.clone(),
                },
                body_config,
            );
            (*lower_proc_body(db, key)).clone()
        };
        match input {
            Some(input) => CompilationUnit::build_memoized_with_analysis_input(
                source,
                options,
                &mut lattice_memo,
                &body_memo,
                None,
                input,
            ),
            None => CompilationUnit::build_for_memoized_with_body_cache(
                source,
                options,
                &mut lattice_memo,
                &body_memo,
            ),
        }
    };
    // Memoise the per-procedure interprocedural taint re-run via `taint_cascade`.
    // The whole-module summary is still rebuilt here (it is the memo's input);
    // only unchanged procedures' `propagate_taints` is skipped.
    let unit = cu.with_interprocedural_memoized(
        registry,
        dialect_opt,
        &mut |qname: &str, ia: &InterproceduralAnalysis| {
            let key = memo_key(db, &lattice_keys, qname)?;
            let summary_key = taint_summary_key(db, ia, qname, dialect_key);
            // A hit returns the memoised map by refcount: `FunctionUnit::taints`
            // is span-free (the offset rebase never touches it), so the unit can
            // share the cached lattice rather than deep-copying it per procedure
            // per build.
            Some(taint_cascade(db, key, summary_key))
        },
    );
    (unit, lattice_keys)
}

fn lattice_request_key<'db>(
    db: &'db dyn TclDb,
    req: &LatticeRequest<'_>,
    body: Script,
    context: CfgContext<'db>,
    registry: &tcl_registry::RegistrySnapshot,
    epoch: u64,
) -> FnLatticeKey<'db> {
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
            plain_command_dispatch: req.plain_command_dispatch,
            source_metadata_input: req.source_metadata_input.cloned(),
            irules_event_body: req.irules_event_body.cloned(),
            command_trust: req.command_trust.snapshot(),
        },
        ValueTransferContext::of(db, req.analysis_context.clone(), epoch),
    )
}

/// The taint-relevant projection of one procedure's [`ProcSummary`], in the
/// deterministic, hashable form interned into [`TaintSummaryKey`].  Holds only
/// the fields `propagate_taints` reads from a summary — `writes_global`
/// (reachable-global seeding), `return_passthrough_param` + `params` (passthrough
/// taint transfer), and `calls` (only the cascade root's transitive callee list,
/// used by the reachable-global check).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ProcTaintSummary {
    /// Fully-qualified procedure name.
    pub qname: String,
    /// Declared parameter names, in order.
    pub params: Vec<String>,
    /// Transitive callee qnames (populated only for the cascade root; sorted).
    pub calls: Vec<String>,
    /// Whether the procedure (or a callee) writes a global/namespace variable.
    pub writes_global: bool,
    /// The parameter the return value passes through, when any.
    pub return_passthrough_param: Option<String>,
}

/// Interned identity of a procedure's interprocedural taint dependencies — the
/// `taint_cascade` key alongside its [`FnLatticeKey`] baseline.  Holds exactly
/// what `propagate_taints` reads from the interprocedural summary: the full set
/// of procedure **names** (so call resolution picks the same target as the whole
/// summary) and the taint-relevant projection of the cascade root + its
/// transitive callees.  A body edit that leaves these unchanged is a cache hit;
/// an edit that flips a reachable callee's `writes_global` / passthrough
/// re-interns this key for exactly the callers that reach it.
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `reachable` moves as the edited procedure's projections move, so a typing
/// session mints fresh ids steadily.  Only the collector's reuse of stale LRU
/// slots keeps the table flat, and it reclaims `Durability::LOW` slots interned
/// inside a tracked query only.  See the crate docs' "The interned garbage
/// collector is load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct TaintSummaryKey<'db> {
    /// All procedure names in the module (sorted) — the call-resolution domain.
    #[returns(ref)]
    pub known_procs: Vec<String>,
    /// The cascade root + its transitive callees' taint projections (sorted by
    /// qname).
    #[returns(ref)]
    pub reachable: Vec<ProcTaintSummary>,
    #[returns(ref)]
    pub dialect: String,
}

/// Build the [`TaintSummaryKey`] for procedure `qname` from the whole-module
/// summary `ia`.  Includes every procedure **name** (resolution domain) plus the
/// taint projection of `qname` and each of its transitive callees.
fn taint_summary_key<'db>(
    db: &'db dyn TclDb,
    ia: &InterproceduralAnalysis,
    qname: &str,
    dialect: &str,
) -> TaintSummaryKey<'db> {
    let mut known: Vec<String> = ia.procedures.keys().cloned().collect();
    known.sort();
    let mut reachable: Vec<ProcTaintSummary> = Vec::new();
    if let Some(root) = ia.procedures.get(qname) {
        let mut calls = root.calls.clone();
        calls.sort();
        reachable.push(ProcTaintSummary {
            qname: qname.to_owned(),
            params: root.params.clone(),
            calls,
            writes_global: root.writes_global,
            return_passthrough_param: root.return_passthrough_param.clone(),
        });
        for callee in &root.calls {
            if callee == qname {
                continue;
            }
            if let Some(s) = ia.procedures.get(callee) {
                reachable.push(ProcTaintSummary {
                    qname: callee.clone(),
                    params: s.params.clone(),
                    calls: Vec::new(),
                    writes_global: s.writes_global,
                    return_passthrough_param: s.return_passthrough_param.clone(),
                });
            }
        }
    }
    reachable.sort_by(|a, b| a.qname.cmp(&b.qname));
    TaintSummaryKey::new(db, known, reachable, dialect.to_owned())
}

/// Memoised interprocedural taint for one procedure (backlog #1 — the
/// `taint_cascade` query layered on [`function_lattice`]'s offset-0 baseline).
///
/// Reconstructs the minimal [`InterproceduralAnalysis`] the key encodes —
/// every procedure name (so call resolution is identical to the whole summary)
/// with the taint projection overlaid for the cascade root + its transitive
/// callees — and re-runs `propagate_taints` over the offset-0 baseline.  Because
/// the taint lattice is `ValueKey`-keyed (span-free), the offset-0 result is
/// installed directly into the rebased unit (no rebase needed).  Byte-identical
/// to [`CompilationUnit::with_interprocedural`]'s per-procedure re-run, guarded
/// by the `compiler_check` corpus differential + the taint-cascade edit tests.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn taint_cascade<'db>(
    db: &'db dyn TclDb,
    lattice_key: FnLatticeKey<'db>,
    summary_key: TaintSummaryKey<'db>,
) -> Arc<HashMap<ValueKey, TaintLattice>> {
    let baseline = function_lattice(db, lattice_key);
    let dialect_opt = lattice_key
        .snapshot(db)
        .profile
        .map(|snapshot| snapshot.profile());
    let registry = lattice_key.snapshot(db).registry.registry();

    // Reconstruct the minimal summary: a stub per known name (resolution
    // domain), with the real taint-relevant fields overlaid for the reachable
    // set.  `propagate_taints` reads only those fields, so this is byte-identical
    // to running against the whole summary.
    let mut ia = InterproceduralAnalysis::default();
    for name in summary_key.known_procs(db) {
        ia.procedures
            .insert(name.clone(), ProcSummary::unknown(name));
    }
    for r in summary_key.reachable(db) {
        let mut s = ProcSummary::unknown(&r.qname);
        s.params.clone_from(&r.params);
        s.calls.clone_from(&r.calls);
        s.writes_global = r.writes_global;
        s.return_passthrough_param
            .clone_from(&r.return_passthrough_param);
        ia.procedures.insert(r.qname.clone(), s);
    }

    Arc::new(baseline.interproc_taints(registry, &ia, dialect_opt))
}

/// Interned identity of one procedure's *interprocedural summary-fixpoint*
/// dependencies — the [`proc_summary_cascade`] key alongside its [`FnLatticeKey`]
/// baseline.  `infer_proc_summary(P)` is a pure function of
/// `P`'s offset-0 body (the `FnLatticeKey`) and, from the *current* summaries it
/// reads: the resolution domain (`known_procs`), the interprocedural
/// [`ProcSummary`] projection of `P`'s reachable set (`interproc_reachable` —
/// what `propagate_taints` reads for call resolution + reachable-global seeding,
/// identical to [`TaintSummaryKey`]), and the [`ReturnTaintSummary`] of every
/// procedure in `P`'s transitive call closure (`callee_summaries` — the return
/// transfer functions `propagate_taints` applies at `P`'s call sites).  A body
/// edit that leaves all of these unchanged re-interns to the same key, so `P`'s
/// inference is a cache hit; an edit that flips a reachable callee's summary
/// re-keys exactly the callers that reach it.
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `interproc_reachable` / `callee_summaries` move with the edited procedure's
/// projections, exactly as [`TaintSummaryKey`]'s `reachable` does, so the same
/// contract applies: collection needs `Durability::LOW` slots interned inside a
/// tracked query.  See the crate docs' "The interned garbage collector is
/// load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct SummaryDepsKey<'db> {
    /// All procedure names in the module (sorted) — the call-resolution domain.
    #[returns(ref)]
    pub known_procs: Vec<String>,
    /// The interproc-analysis projection of the root + its transitive callees
    /// (sorted by qname) — mirrors [`TaintSummaryKey::reachable`].
    #[returns(ref)]
    pub interproc_reachable: Vec<ProcTaintSummary>,
    /// The colour-aware return-taint summaries the root reads from `summaries`:
    /// every procedure in its transitive call closure (sorted by qname, deduped).
    #[returns(ref)]
    pub callee_summaries: Vec<ReturnTaintSummary>,
    #[returns(ref)]
    pub dialect: String,
}

/// Build the [`SummaryDepsKey`] for procedure `qname` from the in-progress
/// summary-fixpoint state.  Reachable set = the root + its transitive callees
/// (`ProcSummary::calls`), exactly as [`taint_summary_key`] computes it, so the
/// interproc projection is identical; `callee_summaries` overlays the
/// return-taint summary of each reachable procedure (including `qname` itself
/// when it is in its own closure — i.e. recursive).  Over-approximating the
/// summaries read (transitive, not just direct callees) is sound: a wrong/missed
/// dependency is caught by the debug fixpoint guard in `converge_summaries_with`,
/// which re-runs the real `infer_proc_summary`.
// A cache-key builder that must observe every input the summary fixpoint reads;
// bundling them into a struct would just move the argument list off-site.
#[allow(clippy::too_many_arguments)]
fn summary_deps_key<'db>(
    db: &'db dyn TclDb,
    qname: &str,
    fu: &FunctionUnit,
    body_source: Option<&str>,
    interproc: Option<&InterproceduralAnalysis>,
    summaries: &HashMap<String, ReturnTaintSummary>,
    known: &HashSet<String>,
    dialect: &str,
) -> SummaryDepsKey<'db> {
    let mut known_procs: Vec<String> = known.iter().cloned().collect();
    known_procs.sort();

    let mut interproc_reachable: Vec<ProcTaintSummary> = Vec::new();
    let mut callee_summaries: Vec<ReturnTaintSummary> = Vec::new();
    if let Some(ia) = interproc
        && let Some(root) = ia.procedures.get(qname)
    {
        let mut calls = root.calls.clone();
        calls.sort();
        calls.dedup();
        // Root: full interproc projection (with its transitive `calls`), plus
        // its own return summary when recursive (qname appears in `calls`).
        interproc_reachable.push(ProcTaintSummary {
            qname: qname.to_owned(),
            params: root.params.clone(),
            calls: calls.clone(),
            writes_global: root.writes_global,
            return_passthrough_param: root.return_passthrough_param.clone(),
        });
        // Complete the callee set: `root.calls` comes from `direct_calls`, which
        // misses a callee buried in a nested command substitution under a dynamic
        // command (e.g. `symbolNodeOf` in `[$t get [symbolNodeOf …] …]`). The real
        // `infer_proc_summary` reads that callee's summary anyway (it scans the
        // FunctionUnit), so without it here the cascade would seed the callee clean
        // and under-taint — diverging from the whole-module solve (and tripping its
        // debug fixpoint guard). `resolved_callees` scans `fu` exactly as the
        // inference does, so we overlay the same callee projections + summaries.
        // The root's own `.calls` field above is left as the real `root.calls` so
        // the reconstructed `ia` still matches the whole-module projection.
        let mut callee_set = calls.clone();
        callee_set.extend(tcl_compiler::taint_interproc::resolved_callees(
            fu,
            known,
            fu.source_lexer_config(),
        ));
        if let Some(src) = body_source {
            callee_set.extend(
                tcl_compiler::taint_interproc::command_subst_callees_with_config(
                    src,
                    qname,
                    known,
                    fu.source_lexer_config(),
                ),
            );
        }
        callee_set.sort();
        callee_set.dedup();
        for callee in &callee_set {
            if callee != qname
                && let Some(s) = ia.procedures.get(callee)
            {
                interproc_reachable.push(ProcTaintSummary {
                    qname: callee.clone(),
                    params: s.params.clone(),
                    calls: Vec::new(),
                    writes_global: s.writes_global,
                    return_passthrough_param: s.return_passthrough_param.clone(),
                });
            }
            if let Some(ts) = summaries.get(callee) {
                callee_summaries.push(ts.clone());
            }
        }
    }
    interproc_reachable.sort_by(|a, b| a.qname.cmp(&b.qname));
    callee_summaries.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
    callee_summaries.dedup();

    SummaryDepsKey::new(
        db,
        known_procs,
        interproc_reachable,
        callee_summaries,
        dialect.to_owned(),
    )
}

/// Memoised per-procedure interprocedural summary inference — the
/// `infer_proc_summary` half of the summary fixpoint, layered on
/// [`function_lattice`]'s offset-0 baseline the way [`taint_cascade`] layers the
/// per-proc taint re-run.
///
/// Reconstructs the minimal context [`SummaryDepsKey`] encodes — every procedure
/// name (so call resolution matches the whole summary), the interproc projection
/// of the reachable set, and the reachable return-taint summaries — and re-runs
/// the real [`tcl_compiler::taint_interproc::infer_proc_summary`] over the
/// offset-0 baseline.  Span-free (the summary is a transfer function over
/// parameter/return taint, not positions), so the offset-0 result is the same
/// the whole-module build computes.  A body edit re-keys only the edited
/// procedure and the callers that reach it; everything else is a cache hit.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn proc_summary_cascade<'db>(
    db: &'db dyn TclDb,
    lattice_key: FnLatticeKey<'db>,
    deps_key: SummaryDepsKey<'db>,
) -> Arc<ReturnTaintSummary> {
    let fu = function_lattice(db, lattice_key);
    let qname = lattice_key.qname(db);
    let params = lattice_key.params(db);
    let dialect = deps_key.dialect(db);
    let dialect_opt = lattice_key
        .snapshot(db)
        .profile
        .map(|profile| profile.profile());
    let registry = lattice_registry(db, lattice_key);
    let registry: &CommandRegistry = &registry;

    // Reconstruct the minimal interproc summary (stub per known name + real
    // fields for the reachable set) — identical to `taint_cascade`'s rebuild.
    let mut ia = InterproceduralAnalysis::default();
    for name in deps_key.known_procs(db) {
        ia.procedures
            .insert(name.clone(), ProcSummary::unknown(name));
    }
    for r in deps_key.interproc_reachable(db) {
        let mut s = ProcSummary::unknown(&r.qname);
        s.params.clone_from(&r.params);
        s.calls.clone_from(&r.calls);
        s.writes_global = r.writes_global;
        s.return_passthrough_param
            .clone_from(&r.return_passthrough_param);
        ia.procedures.insert(r.qname.clone(), s);
    }
    let known: HashSet<String> = deps_key.known_procs(db).iter().cloned().collect();
    // Seed the whole resolution domain with clean summaries — the worklist passes
    // `infer_proc_summary` a map with an entry for *every* procedure, and a
    // resolved callee that is *absent* (vs. present-but-clean) makes
    // `propagate_taints` fall through to its conservative bare-argument join and
    // over-taint (`taint.rs`'s `summaries.get(&target)?`).  So the seed is
    // load-bearing; the reachable overlay then installs the real (possibly
    // tainted) summaries the edited proc actually depends on.
    let mut summaries: HashMap<String, ReturnTaintSummary> = deps_key
        .known_procs(db)
        .iter()
        .map(|name| (name.clone(), ReturnTaintSummary::untainted(name, &[])))
        .collect();
    for s in deps_key.callee_summaries(db) {
        summaries.insert(s.qualified_name.clone(), s.clone());
    }

    Arc::new(tcl_compiler::taint_interproc::infer_proc_summary(
        qname,
        params,
        &fu,
        registry,
        Some(&ia),
        dialect_opt,
        &known,
        &summaries,
    ))
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
    let dialect = key.dialect(db);
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

/// The checks-path memoised solve for one document: the interprocedural taint
/// result **and** the rebased per-procedure non-taint checks, both
/// gathered from a single re-derived [`build_unit_with_keys`] so the duplicate
/// build is paid once for both halves.  `PartialEq` for salsa early-cutoff.
#[derive(Clone, PartialEq)]
pub struct CheckSolve {
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
    let declared = declared_command_surface(db, file);
    let (cu, lattice_keys) = build_unit_with_keys(
        db,
        file.text(db),
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
    lattice_keys: &HashMap<String, FnLatticeKey<'db>>,
    dialect_opt: Option<&'static tcl_dialect::DialectProfile>,
) -> Arc<CheckSolve> {
    let interproc = cu.interproc.as_ref();

    let taints = tcl_compiler::taint_interproc::solve_interprocedural_taints_with(
        &cu,
        registry,
        dialect_opt,
        &mut |qname, params, fu, known, summaries| match memo_key(db, lattice_keys, qname) {
            // Memoised path: the proc has an offset-0 baseline key.
            Some(lattice_key) => {
                let body_source = cu
                    .ir_module
                    .procedures
                    .get(qname)
                    .and_then(|p| p.body_source.as_deref());
                let deps_key = summary_deps_key(
                    db,
                    qname,
                    fu,
                    body_source,
                    interproc,
                    summaries,
                    known,
                    dialect,
                );
                (*proc_summary_cascade(db, lattice_key, deps_key)).clone()
            }
            // Fallback (a proc without a memoised lattice — e.g. an unanalysable
            // body — or whose lattice read another procedure): run the real
            // inference directly, exactly as the bare solve.
            None => tcl_compiler::taint_interproc::infer_proc_summary(
                qname,
                params,
                fu,
                registry,
                interproc,
                dialect_opt,
                known,
                summaries,
            ),
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
    for fu in cu.analysable_functions() {
        match memo_key(db, lattice_keys, &fu.name) {
            Some(key) => {
                let body_offset = cu
                    .ir_module
                    .procedures
                    .get(&fu.name)
                    .map_or(0, |p| p.span.start());
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
                    None::<&std::collections::HashSet<String>>,
                ) {
                    fn_checks.push(d);
                }
            }
        }
    }

    // The per-function non-taint checks over `TclOO` method bodies and
    // synthetic body units (`apply` lambdas, `namespace eval` bodies). The
    // main per-function loop above iterates the proc-only
    // `analysable_functions`, unlike `compiler_checks::run_all_checks_with_solved_and_patterns`'s
    // direct path (which iterates the wider `analysable_body_function_units`
    // and so needs no separate top-up — see `analysable_methods_and_body_units`'s
    // own doc comment for why adding it there too would double-count), so
    // this memoised path needs its own top-up loop to reach the same
    // methods/body units.
    //
    // This must run the **whole** `function_nontaint_checks` family, not just
    // its `shimmer_family_checks` half: the direct path folds the SCCP
    // constant-branch (O100) and GVN full / partial / loop-invariant
    // (O105/O106) halves in here too, and running only the shimmer half would
    // make the memoised path silently drop every O1xx finding inside a method
    // or a `namespace eval` body — 20 of them on one large TclOO corpus file
    // — a diff users on the LSP (memoised) path would see as missing hints
    // the CLI reported.
    //
    // These units never get an offset-0 `FnLatticeKey`, so they carry absolute
    // spans already and need no rebase, same as the `None` arm above (the
    // direct path's `shift` is likewise the identity here — a whole-module
    // build leaves every `FunctionUnit::base_offset` at 0).
    for fu in cu.analysable_methods_and_body_units() {
        for d in tcl_compiler::compiler_checks::function_nontaint_checks(
            fu,
            registry,
            dialect_opt,
            cu.method_instance_vars(&fu.name),
        ) {
            fn_checks.push(d);
        }
    }

    let optimisations = solve_optimisations(db, &cu, lattice_keys, registry, dialect_opt);
    Arc::new(CheckSolve {
        taints,
        fn_checks,
        optimisations,
    })
}

/// Hashable normalisation of a callee's `ConstantReturn` (`f64` isn't `Hash`/`Eq`)
/// for the interned [`OptDepsKey`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ConstReturnKey {
    Int(i64),
    FloatBits(u64),
    Bool(bool),
    Str(String),
}

/// The opt-relevant projection of a direct-callee `ProcSummary` — the fields the
/// optimiser passes read from `cu.interproc` (O103 static-call folding + the
/// purity / effect gates). Hashable, so it can live in the interned [`OptDepsKey`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[allow(clippy::struct_excessive_bools)] // a faithful projection of ProcSummary's fold/purity flags
pub struct OptCalleeSummary {
    pub qname: String,
    pub params: Vec<String>,
    pub can_fold_static_calls: bool,
    pub returns_constant: bool,
    pub constant_return: Option<ConstReturnKey>,
    pub pure: bool,
    pub writes_global: bool,
    pub has_barrier: bool,
    pub has_unknown_calls: bool,
    pub return_passthrough_param: Option<String>,
    pub return_depends_on_params: Vec<String>,
    /// Per-parameter traits (`upvar` / call-by-name / passthrough / …), sorted —
    /// the `call_by_name` O109/O126 suppression reads a callee's by-name params.
    pub param_traits: Vec<(String, Vec<tcl_compiler::interprocedural::ProcArgTrait>)>,
    /// Each parameter's default, sorted by parameter — the same suppression
    /// reads the place an omitted by-name argument's default names.
    pub param_defaults: Vec<(String, String)>,
    /// Whether a call completes normally whatever its arguments hold — the
    /// O109/O126/O108 raise proof takes a pure callee's unused store as dead
    /// where it does.
    pub completes: bool,
    /// The callees surely defined before the load may first run this
    /// procedure, which the same proof takes as callable from its body. The
    /// positions they are proved from stay out of the key, so an edit that
    /// only moves text leaves it alone.
    pub defined_callees: Vec<String>,
}

fn opt_callee_from_summary(s: &ProcSummary) -> OptCalleeSummary {
    use tcl_compiler::interprocedural::ConstantReturn;
    let mut param_traits: Vec<(String, Vec<tcl_compiler::interprocedural::ProcArgTrait>)> = s
        .param_traits
        .iter()
        .map(|(p, traits)| {
            let mut ts: Vec<_> = traits.iter().copied().collect();
            ts.sort_unstable();
            (p.clone(), ts)
        })
        .collect();
    param_traits.sort_by(|a, b| a.0.cmp(&b.0));
    let mut param_defaults: Vec<(String, String)> = s
        .param_defaults
        .iter()
        .map(|(param, default)| (param.clone(), default.clone()))
        .collect();
    param_defaults.sort();
    OptCalleeSummary {
        qname: s.qualified_name.clone(),
        params: s.params.clone(),
        can_fold_static_calls: s.can_fold_static_calls,
        returns_constant: s.returns_constant,
        constant_return: s.constant_return.as_ref().map(|cr| match cr {
            ConstantReturn::Int(i) => ConstReturnKey::Int(*i),
            ConstantReturn::Float(f) => ConstReturnKey::FloatBits(f.to_bits()),
            ConstantReturn::Bool(b) => ConstReturnKey::Bool(*b),
            ConstantReturn::Str(t) => ConstReturnKey::Str(t.clone()),
        }),
        pure: s.pure,
        writes_global: s.writes_global,
        has_barrier: s.has_barrier,
        has_unknown_calls: s.has_unknown_calls,
        return_passthrough_param: s.return_passthrough_param.clone(),
        return_depends_on_params: s.return_depends_on_params.clone(),
        param_traits,
        param_defaults,
        completes: s.completes,
        defined_callees: s.defined_callees.clone(),
    }
}

fn opt_callee_to_summary(o: &OptCalleeSummary) -> ProcSummary {
    use tcl_compiler::interprocedural::ConstantReturn;
    let mut s = ProcSummary::unknown(&o.qname);
    s.params.clone_from(&o.params);
    s.can_fold_static_calls = o.can_fold_static_calls;
    s.returns_constant = o.returns_constant;
    s.constant_return = o.constant_return.as_ref().map(|cr| match cr {
        ConstReturnKey::Int(i) => ConstantReturn::Int(*i),
        ConstReturnKey::FloatBits(b) => ConstantReturn::Float(f64::from_bits(*b)),
        ConstReturnKey::Bool(b) => ConstantReturn::Bool(*b),
        ConstReturnKey::Str(t) => ConstantReturn::Str(t.clone()),
    });
    s.pure = o.pure;
    s.writes_global = o.writes_global;
    s.has_barrier = o.has_barrier;
    s.has_unknown_calls = o.has_unknown_calls;
    s.return_passthrough_param
        .clone_from(&o.return_passthrough_param);
    s.return_depends_on_params
        .clone_from(&o.return_depends_on_params);
    s.param_traits = o
        .param_traits
        .iter()
        .map(|(p, ts)| (p.clone(), ts.iter().copied().collect()))
        .collect();
    s.param_defaults = o.param_defaults.iter().cloned().collect();
    s.completes = o.completes;
    s.defined_callees.clone_from(&o.defined_callees);
    s
}

/// Interned per-procedure optimiser dependency key: the offset-0 body source (for
/// the optimiser's `source[span]` reads), the cross-proc resolution domain (every
/// module proc qname), the opt-relevant summaries of this proc's resolved direct
/// callees (O103 fold / purity inputs), and the module `redefined_procedures` set
/// (the O103 don't-fold-a-redefined-callee gate).  A body edit to an unrelated proc
/// whose summary this proc doesn't read leaves this key unchanged → cache hit.
///
/// **Per-revision key — reclaimed by salsa's interned garbage collector.**
/// `body_source` and `proc_body_source` change on every keystroke inside the
/// procedure, and the memo behind a stale id holds that procedure's whole
/// optimisation set.  Slot reuse releases it, for `Durability::LOW` slots
/// interned inside a tracked query only.  See the crate docs' "The interned
/// garbage collector is load-bearing"; pinned by `tests/interned_gc.rs`.
#[salsa::interned]
pub struct OptDepsKey<'db> {
    #[returns(ref)]
    pub body_source: String,
    #[returns(ref)]
    pub proc_names: Vec<String>,
    #[returns(ref)]
    pub callees: Vec<OptCalleeSummary>,
    #[returns(ref)]
    pub redefined: Vec<String>,
    /// The procedure's `name` field exactly as the whole-module lowering records
    /// it — the *written* name (a fully-qualified `proc ::ns::p` keeps its `::`
    /// prefix; a short `proc p` inside `namespace eval` stays short). Name-bearing
    /// optimisation messages (e.g. O121 "tailcall for self-recursion in proc
    /// '<name>'") echo this verbatim, so the single-proc memo must reconstruct the
    /// same `proc.name` rather than deriving a short name from the qualified key.
    #[returns(ref)]
    pub proc_name: String,
    /// The procedure's `body_source` exactly as the whole-module lowering records
    /// it — the **body text only** (`args[2]`), *not* the whole-command slice used
    /// for `Module.source` span alignment. O122's loop-conversion rewrite
    /// (`emit_loop_conversion`) locates `body_source` inside the proc text and
    /// wraps it in `proc … { while {1} { <body> } }`, so it must be the body — a
    /// whole-`proc …` slice would nest the entire declaration into the replacement.
    #[returns(ref)]
    pub proc_body_source: String,
    /// The procedure's `params_raw` (`args[1]`) as written, so O122's replacement
    /// reproduces the original parameter-list text verbatim (spacing, defaults)
    /// rather than a `params.join(" ")` reconstruction.
    #[returns(ref)]
    pub proc_params_raw: String,
    #[returns(ref)]
    pub source_entry: tcl_compiler::command_binding::SourceAnalysisEntry,
}

/// Build the [`OptDepsKey`] for `qname` from the whole-module interproc summary.
///
/// Captures **every** module proc's opt-relevant summary (the resolution domain +
/// fold/purity inputs).  A proc's call to a callee can come from a bare statement
/// *or* a `[…]` command substitution (which `direct_calls` does not record), so a
/// resolved-direct-callee-only key would miss an O103 fold inside a substitution.
/// Keying on every proc's *opt-projection* is the correct superset: it only changes
/// when some proc's fold/purity facts (`can_fold_static_calls` / `constant_return` /
/// `pure` / …) change — **not** on every body edit, since most edits leave those
/// summary fields untouched (a `set y 1` → `set y 2` edit re-keys only the edited
/// proc's own `FnLatticeKey`, not every caller's `OptDepsKey`).
fn opt_deps_key<'db>(
    db: &'db dyn TclDb,
    ia: &InterproceduralAnalysis,
    redefined: &HashSet<String>,
    body_source: &str,
    proc_name: &str,
    proc_body_source: &str,
    proc_params_raw: &str,
    source_entry: &tcl_compiler::command_binding::SourceAnalysisEntry,
) -> OptDepsKey<'db> {
    let mut proc_names: Vec<String> = ia.procedures.keys().cloned().collect();
    proc_names.sort();
    let mut callees: Vec<OptCalleeSummary> = ia
        .procedures
        .values()
        .map(opt_callee_from_summary)
        .collect();
    callees.sort_by(|a, b| a.qname.cmp(&b.qname));
    let mut redef: Vec<String> = redefined.iter().cloned().collect();
    redef.sort();
    OptDepsKey::new(
        db,
        body_source.to_owned(),
        proc_names,
        callees,
        redef,
        proc_name.to_owned(),
        proc_body_source.to_owned(),
        proc_params_raw.to_owned(),
        source_entry.clone(),
    )
}

/// Memoised offset-0 raw optimisations for one procedure.
/// Builds a single-procedure offset-0 [`CompilationUnit`] — the proc's offset-0
/// `function_lattice` unit, its offset-0 IR body, and the reconstructed interproc
/// (domain stubs overlaid with the resolved direct callees' real opt summaries) —
/// and runs [`optimise_unit_raw`] on it.  Returns the **raw** (pre-overlap-select,
/// pre-renumber) optimisations at **offset 0**; the caller rebases by the proc's
/// `body_offset` and runs the whole-module [`finalise_optimisations`] over the
/// assembled set.  A body edit re-runs only the edited proc; an unrelated proc's
/// edit is a cache hit unless this proc reads its summary (a resolved direct call).
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn function_optimisations<'db>(
    db: &'db dyn TclDb,
    key: FnLatticeKey<'db>,
    deps: OptDepsKey<'db>,
) -> Arc<Vec<Optimisation>> {
    let fu = function_lattice(db, key);
    let qname = key.qname(db).clone();
    let params = key.params(db).clone();
    let body = key.body(db).clone();
    let dialect_opt = key.snapshot(db).profile.map(|snapshot| snapshot.profile());
    let registry = key.snapshot(db).registry.registry();
    let body_source = deps.body_source(db).clone();

    let mut ia = InterproceduralAnalysis::default();
    for name in deps.proc_names(db) {
        ia.procedures
            .insert(name.clone(), ProcSummary::unknown(name));
    }
    for c in deps.callees(db) {
        ia.procedures
            .insert(c.qname.clone(), opt_callee_to_summary(c));
    }
    let redefined: HashSet<String> = deps.redefined(db).iter().cloned().collect();

    let body_len = u32::try_from(body_source.len()).unwrap_or(u32::MAX);
    let proc = tcl_compiler::ir::Procedure {
        name: deps.proc_name(db).clone(),
        qualified_name: qname.clone(),
        params: params.clone(),
        span: tcl_lexer::Span::new(0, body_len),
        body,
        // `params_raw` / `body_source` are the *written* param-list and body text
        // (matching the whole-module `Procedure`), NOT the whole-command slice held
        // in `body_source` for `Module.source` span alignment — O122's rewrite wraps
        // the body verbatim, so a slice here would nest the whole `proc …`.
        params_raw: deps.proc_params_raw(db).clone(),
        body_source: Some(deps.proc_body_source(db).clone()),
        // The unit source here *is* the body text, so it opens at offset 0.
        body_offset: 0,
        namespace_scoped: false,
        base_priority: 0,
    };
    let mut ir_procs = HashMap::new();
    ir_procs.insert(qname.clone(), proc);
    let ir_module = tcl_compiler::ir::Module {
        irules_event_bodies: HashMap::new(),
        retained_source_bindings: None,
        // A body-only synthetic image lacks the complete original input owner.
        source_metadata_input: None,
        lexer_config: key.lexer_config(db),
        dialect_profile: dialect_opt,
        registry_snapshot: Some(key.snapshot(db).registry.clone()),
        top_level_kind: tcl_compiler::ir::TopLevelKind::Script,
        source_entry: deps.source_entry(db).clone(),
        future_call_sites: Vec::new(),
        installed_procedure_body_units: Default::default(),
        original_declaration_body_units: Default::default(),
        // This synthetic unit has no retained source allocation attestation;
        // its procedure name cannot invent a callee implementation identity.
        procedure_implementation_bodies: Default::default(),
        source: body_source.clone().into(),
        native_namespace: None,
        // This synthetic module has no executable top-level script.
        // Its procedure body retains its selected namespace independently.
        top_level_namespace: "::".to_owned(),
        top_level_namespace_context: None,
        // The document's dialect, not `None`. The unit is synthesised, but the
        // release it is analysed under is real and known right here
        // (`dialect_opt`, which is also what `optimise_unit_raw` below is
        // given), so labelling the module "no release" is simply false.
        //
        // Nothing on *this* path reads it today — `Module::dialect`'s numeral
        // consumers (`native_integer_proof`, `common_aot_plan`) hang off the
        // wasm codegen pipeline, and the optimisations this path does produce
        // take their grammar from `dialect_opt` directly, so the folds are
        // correct either way (`0755 + 1` → 494 under 8.6, 756 under 9.0).
        // It is set correctly because a synthesised module carrying the wrong
        // release is a trap for the next consumer: `numbers_for_dialect(None)`
        // silently means Tcl 9.0, so a future reader would mis-fold rather than
        // fail. Cheap to keep honest, expensive to debug later.
        dialect: dialect_opt.map(|profile| profile.name.to_owned()),
        // This synthetic optimisation unit models the ordinary compiler path;
        // it is never a trace/mutation recovery artefact.
        plain_command_dispatch: false,
        top_level: tcl_compiler::ir::Script::new(),
        procedures: ir_procs,
        methods: HashMap::new(),
        body_units: HashMap::new(),
        lambda_body_units: std::collections::BTreeSet::new(),
        redefined_procedures: redefined,
        redefined_methods: HashMap::new(),
        oo_unanalysed_classes: HashSet::new(),
        oo_evidence: tcl_compiler::ir::OoDefinitionEvidence::default(),
        class_relations: Vec::new(),
        namespace_imports: Vec::new(),
        namespace_exports: Vec::new(),
        // Always empty/false here — the caller (`memoised_module_optimisations`)
        // falls back to the whole-module `optimise_unit` whenever the real
        // module carries any trace fact, so this per-proc offset-0 unit is
        // only ever built for a module with none. See that fallback's
        // comment for why threading these through the salsa `OptDepsKey`
        // instead was not the chosen fix.
        traced_commands: BTreeSet::new(),
        has_dynamic_trace: false,
        traced_variables: BTreeSet::new(),
        has_dynamic_variable_trace: false,
        deferred_writes: tcl_compiler::ir::DeferredWrites::default(),
        reference_bodies: tcl_compiler::ir::ReferenceBodies::default(),
        declared_frame_effects: std::collections::BTreeMap::new(),
    };
    let empty_cfg = tcl_compiler::cfg::Function::new("::", "entry");
    let top_fu = FunctionUnit::build("::", empty_cfg.clone(), &[], registry, key.lexer_config(db));
    let mut cfg_procs = HashMap::new();
    cfg_procs.insert(qname.clone(), fu.cfg.clone());
    let mut fu_procs = HashMap::new();
    fu_procs.insert(qname.clone(), (*fu).clone());
    let cu = CompilationUnit {
        source: body_source,
        ir_module,
        cfg_module: tcl_compiler::cfg::CfgModule {
            top_level: empty_cfg,
            procedures: cfg_procs,
        },
        // This per-procedure memo path is entered only after the whole-unit
        // solver proves there are no command mutations.
        command_mutations: tcl_compiler::command_binding::ModuleCommandMutations::default(),
        top_level: top_fu,
        procedures: fu_procs,
        methods: HashMap::new(),
        body_units: HashMap::new(),
        interproc: Some(ia),
        connection_scope: None,
        // A synthetic single-procedure unit: no source text of its own to
        // scan for boundaries, no cross-file view to inherit, and no
        // document of its own to carry stub declarations.
        caller_scope: tcl_compiler::compilation_unit::UnitCallerScope::default(),
        declared_commands: deps
            .source_entry(db)
            .declared_commands
            .clone()
            .unwrap_or_default(),
        transfers: tcl_compiler::interprocedural::TransferSummaries::default(),
    };
    Arc::new(tcl_compiler::optimiser::optimise_unit_raw(
        &cu,
        registry,
        dialect_opt,
    ))
}

/// Whether `module` carries any whole-module trace fact (execution *or*
/// variable — `Module::traced_commands` / `has_dynamic_trace` /
/// `traced_variables` / `has_dynamic_variable_trace`) or any callback script
/// that writes a variable (`Module::deferred_writes`).
///
/// The single-proc offset-0 `Module` [`function_optimisations`] builds has
/// no way to reconstruct these — its `OptDepsKey` threads `proc_names` /
/// `callees` / `redefined`, but not trace state, so a memoised per-proc
/// unit would silently see "nothing is traced" regardless of the real
/// module content. [`solve_optimisations`] falls back to the whole-module
/// build whenever this is `true`, exactly like its `mutations` /
/// `has_arg_sensitive_target` fallbacks — traces are rare enough in
/// practice that this costs little, and it is far lower-risk than
/// widening the salsa dependency key to thread four more whole-module
/// facts through the per-proc cache.
fn module_has_trace_facts(module: &tcl_compiler::ir::Module) -> bool {
    !module.traced_commands.is_empty()
        || module.has_dynamic_trace
        || !module.traced_variables.is_empty()
        || module.has_dynamic_variable_trace
        || !module.deferred_writes.is_clear()
}

/// Assemble a document's optimisations from the per-procedure memo.
///
/// For a non-iRules module with no command mutations and a lattice key for every
/// analysable procedure, each proc's raw optimisations come from the memoised
/// [`function_optimisations`] (offset 0), rebased by the proc's `body_offset`; the
/// top-level body's raw optimisations are computed on a top-level-only unit (small,
/// not memoised), and the whole-module [`finalise_optimisations`] runs once over the
/// assembled set.  Otherwise (iRules / command mutations / a complexity-guarded
/// proc without a key / a memoised lattice that read another procedure) it falls
/// back to the whole-module [`optimise_unit`] — always byte-identical, guarded by
/// the `compiler_check` random-edit + corpus fuzzers.
fn solve_optimisations<'db>(
    db: &'db dyn TclDb,
    cu: &CompilationUnit,
    lattice_keys: &HashMap<String, FnLatticeKey<'db>>,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<Optimisation> {
    let mutations = cu.command_mutations.clone();
    let every_proc_keyed = cu
        .procedures
        .keys()
        .all(|qname| lattice_keys.contains_key(qname));
    let is_irules = dialect.is_some_and(tcl_dialect::DialectProfile::is_irules);
    let dialect_opt = dialect;
    // The **argument-sensitive** O103 fold re-runs a *pure* callee's body with the
    // call's constant arguments (`evaluate_proc_with_constants`), so it reads the
    // callee's whole `FunctionUnit` (`cu.procedures.get(callee)`), not just its
    // summary — a genuine cross-function *body* dependency the single-proc unit
    // cannot serve. Fall back to the whole-module optimise when any proc could be
    // such a fold target: `pure` but not an argument-independent constant return
    // (the latter is summary-level and the memo handles it).
    let has_arg_sensitive_target = cu.interproc.as_ref().is_some_and(|ia| {
        ia.procedures
            .values()
            .any(|s| s.pure && !(s.can_fold_static_calls && s.constant_return.is_some()))
    });
    // A memoised lattice that read another procedure is not the unit's: the
    // unit built that procedure afresh (`SccpResult::reads_module`).
    let reads_module = lattice_keys
        .values()
        .any(|&key| function_lattice(db, key).sccp.reads_module);
    if is_irules
        || !cu.methods.is_empty()
        || mutations != tcl_compiler::command_binding::ModuleCommandMutations::default()
        || has_arg_sensitive_target
        || !every_proc_keyed
        || reads_module
        || module_has_trace_facts(&cu.ir_module)
    {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    }

    let ia = cu.interproc.clone().unwrap_or_default();
    let redefined = &cu.ir_module.redefined_procedures;
    let mut raw: Vec<Optimisation> = Vec::new();
    // Each per-proc `optimise_unit_raw` allocates group ids from 0, so the procs'
    // raw sets carry **colliding** group ids; offset each proc's groups into a
    // disjoint range so the assembled set has unique ids (the whole-module build's
    // invariant). The final group numbers are then identical: `renumber_groups`
    // reassigns by sorted first-appearance, which only depends on the (unchanged)
    // span order and the preserved per-proc grouping — not the offset values.
    let mut group_base: u32 = 0;

    // Per-procedure memoised raw optimisations, rebased to absolute spans.
    for qname in cu.procedures.keys() {
        let Some(&key) = lattice_keys.get(qname) else {
            continue;
        };
        let Some(proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        // The offset-0 lattice normalises every span by `-proc.span.start()`
        // (the body offset), so the single-proc unit's `source` must be the file
        // sliced at `[body_offset, body_end)` — `proc.body_source` is the body text
        // but not necessarily at that exact base. Reading the slice keeps
        // `source[offset0_span]` byte-aligned with the whole-module read.
        let body_offset = proc.span.start();
        let body_end = proc.span.end();
        let body_source = cu
            .source
            .get(body_offset as usize..body_end as usize)
            .unwrap_or("")
            .to_owned();
        // `body_source` (the whole-command slice) is `Module.source` for span
        // alignment; the proc's real `body_source` (`args[2]`) + `params_raw`
        // (`args[1]`) are threaded separately so O122's rewrite matches the
        // whole-module `Procedure` (see `OptDepsKey`).
        let deps = opt_deps_key(
            db,
            &ia,
            redefined,
            &body_source,
            &proc.name,
            proc.body_source.as_deref().unwrap_or(""),
            &proc.params_raw,
            &cu.ir_module.source_entry,
        );
        let mut max_group: Option<u32> = None;
        for opt in function_optimisations(db, key, deps).iter() {
            let mut opt = opt.clone();
            opt.span =
                tcl_lexer::Span::new(opt.span.start() + body_offset, opt.span.end() + body_offset);
            if let Some(g) = opt.group {
                opt.group = Some(group_base + g);
                max_group = Some(max_group.map_or(g, |m| m.max(g)));
            }
            raw.push(opt);
        }
        if let Some(m) = max_group {
            group_base += m + 1;
        }
    }

    // Top-level body: not per-proc memoised (it changes on top-level edits), but it
    // is usually tiny.  Run the passes on a top-level-only unit — `procedures`
    // empty so only the top-level is optimised, `interproc` retained so the
    // top-level's O103 calls resolve — producing absolute-span raw optimisations.
    let top_unit = top_level_only_unit(cu, redefined);
    for mut opt in tcl_compiler::optimiser::optimise_unit_raw(&top_unit, registry, dialect_opt) {
        if let Some(g) = opt.group {
            opt.group = Some(group_base + g);
        }
        raw.push(opt);
    }

    tcl_compiler::optimiser::finalise_optimisations(&raw, cu, registry, dialect_opt)
}

/// The top-level-only [`CompilationUnit`] [`solve_optimisations`] runs the
/// passes over: `procedures` empty so only the top-level is optimised,
/// `interproc` / `caller_scope` retained so the top-level's O103 calls
/// resolve against the same boundary / cross-file facts the whole-module
/// build resolved. Trace facts are copied from the real module (the
/// `has_trace_facts` fallback means this path only runs when there are
/// none, but copy the real values rather than asserting that by omission).
fn top_level_only_unit(
    cu: &CompilationUnit,
    redefined: &std::collections::HashSet<String>,
) -> CompilationUnit {
    CompilationUnit {
        source: cu.source.clone(),
        ir_module: tcl_compiler::ir::Module {
            irules_event_bodies: HashMap::new(),
            retained_source_bindings: cu.ir_module.retained_source_bindings.clone(),
            source_metadata_input: cu.ir_module.source_metadata_input.clone(),
            lexer_config: cu.ir_module.lexer_config,
            dialect_profile: cu.ir_module.dialect_profile,
            registry_snapshot: cu.ir_module.registry_snapshot.clone(),
            top_level_kind: tcl_compiler::ir::TopLevelKind::Script,
            source_entry: cu.ir_module.source_entry.clone(),
            future_call_sites: cu.ir_module.future_call_sites.clone(),
            installed_procedure_body_units: cu.ir_module.installed_procedure_body_units.clone(),
            original_declaration_body_units: cu.ir_module.original_declaration_body_units.clone(),
            procedure_implementation_bodies: cu.ir_module.procedure_implementation_bodies.clone(),
            source: cu.source.clone().into(),
            native_namespace: cu.ir_module.native_namespace.clone(),
            top_level_namespace: cu.ir_module.top_level_namespace.clone(),
            top_level_namespace_context: cu.ir_module.top_level_namespace_context.clone(),
            dialect: cu.ir_module.dialect.clone(),
            plain_command_dispatch: cu.ir_module.plain_command_dispatch,
            top_level: cu.ir_module.top_level.clone(),
            procedures: HashMap::new(),
            methods: HashMap::new(),
            body_units: HashMap::new(),
            lambda_body_units: std::collections::BTreeSet::new(),
            redefined_procedures: redefined.clone(),
            redefined_methods: HashMap::new(),
            oo_unanalysed_classes: HashSet::new(),
            oo_evidence: tcl_compiler::ir::OoDefinitionEvidence::default(),
            class_relations: Vec::new(),
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            traced_commands: cu.ir_module.traced_commands.clone(),
            has_dynamic_trace: cu.ir_module.has_dynamic_trace,
            traced_variables: cu.ir_module.traced_variables.clone(),
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
            deferred_writes: cu.ir_module.deferred_writes.clone(),
            reference_bodies: cu.ir_module.reference_bodies.clone(),
            declared_frame_effects: cu.ir_module.declared_frame_effects.clone(),
        },
        cfg_module: tcl_compiler::cfg::CfgModule {
            top_level: cu.cfg_module.top_level.clone(),
            procedures: HashMap::new(),
        },
        command_mutations: cu.command_mutations.clone(),
        top_level: cu.top_level.clone(),
        procedures: HashMap::new(),
        methods: HashMap::new(),
        body_units: HashMap::new(),
        interproc: cu.interproc.clone(),
        connection_scope: None,
        caller_scope: cu.caller_scope.clone(),
        declared_commands: cu.declared_commands.clone(),
        transfers: cu.transfers.clone(),
    }
}

/// Interned identity of the lexer grammar a [`compilation_unit`] build lexes
/// under: **the resolved environment id**, not an expanded
/// [`tcl_lexer::LexerConfig`].  The call-site knobs (`strict_quoting = false`,
/// zero base offsets) are genuinely the default on every path.
///
/// Interning only three of the six dialect-derived `LexerConfig` fields
/// (`expand_syntax`, `irules_brace_separator`, `brace_line_continuation`) and
/// letting `to_config` restore the rest from
/// [`tcl_lexer::LexerConfig::default`] would pin the memoised path's
/// `braced_var` to `Tcl9Nesting` and `escapes` to `Tcl90` regardless of the
/// document's dialect, so a `tcl8.6` document would lex `${a{b}c}` under the
/// 9.0 close rule.  Keying on the environment id
/// instead of the expanded fields makes `to_config` name
/// [`tcl_lexer::LexerConfig::for_dialect`] itself, so every field is the
/// document's.
///
/// **And it keeps the sharing**, which widening the tuple would have cost:
/// with all four hosts of the CFG/SSA tail's unit (this crate's
/// [`analyse_per_item_with`], `Analyser::emit_cfg_ssa_diagnostics`'s own
/// build, `tcl diag`'s `collect_rows`, and `xtask fp_sweep`) lexing under the
/// document's environment, both diagnostics consumers intern the **same** id
/// for a document and demand one [`compilation_unit`] build per edit — for
/// every environment, where the truncated tuple shared for 15 of the 20 and
/// built twice for the other five.
#[salsa::interned]
pub struct LexerCfgKey<'db> {
    /// The resolved environment id (`tcl8.6`, `f5-irules`, …) whose grammar
    /// this build lexes under.
    #[returns(ref)]
    pub environment: String,
}

impl LexerCfgKey<'_> {
    /// The full [`tcl_lexer::LexerConfig`] this key represents: the
    /// environment's own grammar plus the invariant call-site knobs.
    fn to_config(self, db: &dyn TclDb) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::from_grammar(
            tcl_lsp_core::environment_for_dialect(self.environment(db)).grammar(),
        )
    }
}

/// Intern a [`LexerCfgKey`] for the environment `dialect` names.
///
/// The name is resolved through the one dialect-name ingress seam first, so
/// an alias (`irules`) and its canonical id (`f5-irules`) share one key and
/// one build rather than interning two.
fn lexer_cfg_key<'db>(db: &'db dyn TclDb, dialect: &str) -> LexerCfgKey<'db> {
    LexerCfgKey::new(
        db,
        tcl_registry::model::ingress::resolve_environment(dialect)
            .id()
            .to_owned(),
    )
}

/// The registry a unit resolves against: the dialect's shared one, or its
/// generation carrying a workspace pack overlay.
enum UnitRegistry {
    /// The dialect's process-wide registry ([`TclDb::registry`]).
    Shared(&'static CommandRegistry),
    /// The dialect's registry with the workspace's packs
    /// ([`TclDb::registry_with_overlay`]).
    Overlaid(Arc<CommandRegistry>),
}

impl std::ops::Deref for UnitRegistry {
    type Target = CommandRegistry;

    fn deref(&self) -> &CommandRegistry {
        match self {
            Self::Shared(registry) => registry,
            Self::Overlaid(registry) => registry,
        }
    }
}

/// The registry a unit for `dialect` under pack overlay `overlay` resolves
/// against: the shared one when there is no overlay (`0`), so a workspace
/// with no packs resolves exactly as before, and the overlaid generation
/// otherwise (`docs/design/compiler/value-transfers.md` § *One invocation,
/// one context*: a workspace pack's declarations reach the memoised
/// lattice).
///
/// # Errors
///
/// [`OverlayMiss`] when the overlay is non-zero and its packs are not
/// installed: the caller decides what it may do without them.
fn unit_registry(db: &dyn TclDb, dialect: &str, overlay: u64) -> Result<UnitRegistry, OverlayMiss> {
    if overlay == 0 {
        return Ok(UnitRegistry::Shared(db.registry(dialect)));
    }
    // A tracked read: a moved overlay epoch asks the query again.
    let _epoch = overlay_epoch(db);
    db.registry_with_overlay(dialect, overlay)
        .map(UnitRegistry::Overlaid)
}

/// The registry a memoised per-procedure query resolves against: the one the
/// unit query that keyed it resolved.
///
/// A per-procedure query runs inside, or is re-verified for, a unit query
/// that has already resolved this overlay through
/// [`TclDb::registry_with_overlay`], and the database holds what it resolved,
/// so this cannot miss for a key a unit query resolved. Being asked for one
/// none did is a caller error, and it stops here: a lattice computed against
/// a registry the packs are missing from would be memoised under the packs'
/// key.
fn nested_registry(db: &dyn TclDb, dialect: &str, overlay: u64) -> UnitRegistry {
    unit_registry(db, dialect, overlay).unwrap_or_else(|miss| {
        panic!("{miss}: a per-procedure query ran for an overlay no unit query resolved")
    })
}

/// [`nested_registry`] for a lattice key, which carries its overlay in the
/// analysis context.
fn lattice_registry(db: &dyn TclDb, key: FnLatticeKey<'_>) -> Arc<CommandRegistry> {
    key.snapshot(db).registry.shared_registry()
}

/// A query that cannot be answered without packs nobody has installed
/// answers nothing. The answer depends on the overlay epoch, which
/// [`unit_registry`] read, so it is asked again when the host moves the epoch
/// after installing the packs.
fn abstain<T>() -> Option<T> {
    None
}

/// [`abstain`] for the checks-and-optimisations pass: no findings, and no
/// rewrites.
fn abstain_diagnostics() -> Arc<CompilerDiagnostics> {
    Arc::new(CompilerDiagnostics {
        checks: Vec::new(),
        optimisations: Vec::new(),
    })
}

/// The registry the token queries classify commands against: the overlaid
/// generation once the packs are installed, and the dialect's plain one until
/// then.
///
/// Highlighting advises and is asked for again once the packs arrive (a pack
/// reload has the client re-pull its tokens), so a window of pack commands
/// coloured as unknown ones costs nothing that outlives it; nothing here
/// builds a unit or offers a rewrite, which is why this is the one query
/// family that reads the plain registry for a miss. The miss is not
/// memoised past the overlay epoch that [`unit_registry`] read.
fn token_registry(db: &dyn TclDb, dialect: &str, overlay: u64) -> UnitRegistry {
    unit_registry(db, dialect, overlay)
        .unwrap_or_else(|_| UnitRegistry::Shared(db.registry(dialect)))
}

/// The [`UnitBuildOptions`] every [`CompilationUnit`] built for `file` under
/// `cfg` shares — one place so the taint solve and the shared unit cannot
/// drift on the dialect, the cross-file view, or the document's own
/// declarations.
fn unit_build_options<'a>(
    db: &dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'_>,
    registry: &'a CommandRegistry,
    external: Option<&'a CallSiteEvidence>,
    declared: &'a DeclaredSurface,
) -> UnitBuildOptions<'a> {
    UnitBuildOptions {
        registry,
        defer_top_level: false,
        config: cfg.to_config(db),
        dialect: tcl_lsp_core::optional_profile_for_dialect(file.dialect(db)),
        external_call_sites: external,
        declared_commands: Some(declared),
    }
}

/// The document's own command declarations — its inline `# tcl-lsp: stub`
/// block and the nearest `<dialect>.tcl.stubs` sidecar, ingested through the
/// analyser's one stub-ingestion path
/// ([`tcl_compiler::analyser::utils::document_declared_surface`]).
///
/// Every unit built for this file declares the same thing, so a stubbed
/// command's `body` / `var` argument roles reach lowering and the
/// interprocedural scan exactly as a shipped `CommandSpec`'s do. Cache
/// invalidation rides the ordinary inputs — the document's text and path for
/// an inline block, [`SourceFile::sidecar_stubs_epoch`] for a sidecar.
#[salsa::tracked(returns(clone))]
pub fn declared_command_surface(db: &dyn TclDb, file: SourceFile) -> Arc<DeclaredSurface> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    Arc::new(tcl_compiler::analyser::utils::document_declared_surface(
        file.text(db),
        file.path(db).as_deref(),
        file.dialect(db),
    ))
}

/// The shared, memoised [`CompilationUnit`] for a document under a given lexer
/// config and workspace pack overlay (`0` for none) — built via
/// `memoised_compilation_unit` (per-procedure lattices on the salsa-native
/// [`function_lattice`] graph) against the overlay's registry, so a pack's
/// declarations reach every lattice of the file and a pack edit, which
/// changes the overlay, invalidates them.  Tracked + keyed on
/// `(file, cfg, overlay)` so the analyser tail ([`file_analysis_incremental`]) and the
/// optimiser/compiler-checks pass ([`compiler_check_diagnostics`]) **share one
/// build per edit** whenever their configs coincide — every dialect bar
/// `tcl8.4` and the three `f5-tcl`-grammar dialects (`f5-irules`, `f5-tmsh`,
/// `f5-iapps`, all of which select `GRAMMAR_F5_TCL`);
/// for those four the configs differ, so each consumer builds its own.  Byte-identical to a direct
/// `memoised_compilation_unit` call.
///
/// **`None` is an abstention, not an error page.** A non-zero `overlay` names a
/// pack generation only the pack loader can build, so when nothing has
/// installed it for the document's dialect ([`TclDb::registry_with_overlay`]
/// answers an [`OverlayMiss`]) there is no unit: a unit built against the
/// plain registry would resolve every pack command as an unknown one and
/// would be memoised under the pack's key, and the rewrites the optimiser
/// and the checks derive from it could be wrong for the workspace. The miss
/// is recorded once for the host ([`take_overlay_misses`]), nothing is
/// published in its place, and the abstention reads the overlay epoch
/// ([`set_overlay_epoch`]), so it runs again — and so does everything that
/// read it — once the host installs the packs and moves the epoch.
// LRU-capped: per-item key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 512, returns(clone))]
pub fn compilation_unit<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    overlay: u64,
) -> Option<Arc<CompilationUnit>> {
    let dialect = file.dialect(db).clone();
    let Ok(registry) = unit_registry(db, &dialect, overlay) else {
        return abstain();
    };
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file);
    Some(Arc::new(memoised_compilation_unit(
        db,
        file.text(db),
        unit_build_options(db, file, cfg, &registry, external.as_deref(), &declared),
    )))
}

/// Memoised source compilation under the document's checked complete input.
/// Missing context or a stale requested grammar withholds the unit.
#[salsa::tracked(lru = 512, returns(clone))]
pub fn compilation_unit_for_config<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    config: AnalyserConfig,
) -> Option<Arc<CompilationUnit>> {
    let input = document_analysis_input(db, file, config).ok()?;
    let registry = input.borrowed_context_registry().commands();
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file);
    let options = supplied_unit_build_options(
        db,
        file,
        cfg,
        registry,
        external.as_deref(),
        &declared,
        &input,
    )?;
    Some(Arc::new(
        build_unit_with_keys_and_input(db, file.text(db), options, Some(&input)).0,
    ))
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn proc_taint_solve_for_config<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
    config: AnalyserConfig,
) -> Option<Arc<CheckSolve>> {
    let input = document_analysis_input(db, file, config).ok()?;
    let registry = input.borrowed_context_registry().commands();
    let external = file.external_call_sites(db).clone();
    let declared = declared_command_surface(db, file);
    let options = supplied_unit_build_options(
        db,
        file,
        cfg,
        registry,
        external.as_deref(),
        &declared,
        &input,
    )?;
    let (cu, keys) = build_unit_with_keys_and_input(db, file.text(db), options, Some(&input));
    Some(solve_checks_for_unit(
        db,
        file.dialect(db),
        registry,
        cu,
        &keys,
        Some(input.unit_profile()),
    ))
}

fn supplied_unit_build_options<'a>(
    db: &dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'_>,
    registry: &'a CommandRegistry,
    external: Option<&'a CallSiteEvidence>,
    declared: &'a DeclaredSurface,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Option<UnitBuildOptions<'a>> {
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        cfg.to_config(db),
        Some(input.unit_profile()),
    )?;
    Some(UnitBuildOptions {
        dialect: Some(input.unit_profile()),
        config: input.lexer_config(),
        ..unit_build_options(db, file, cfg, registry, external, declared)
    })
}

/// Incremental whole-file analysis: the per-item path with each `proc` body's
/// isolated analysis memoised via [`item_body_analysis`], so a body edit
/// recomputes one body + the cheap shell instead of the whole walk; the
/// CFG/SSA diagnostic tail's per-procedure lattices are likewise memoised via
/// the salsa-native [`function_lattice`] query (through
/// `memoised_compilation_unit`), so an unchanged procedure's lattice is reused
/// (and rebased) instead of rebuilt.  Byte-identical to [`file_analysis`] (and
/// `analyse`) — proven by the `per_item_corpus` gate over the shared
/// `analyse_per_item_with` orchestration.
///
/// `config.disabled_diagnostics` is the analyser's production-time skip —
/// rule 2's permitted saving in `docs/design/compiler/diagnostic-policy.md`
/// § What the producers leave to the policy: the codes the document's policy
/// turns off, which the analyser need not compute. It is never a presentation filter. The
/// surface that reads this analysis declares the same set to its report, so a
/// code left uncomputed is explained rather than read as clean; what the
/// document shows is the policy step's decision.
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn file_analysis_incremental(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    let disabled_vec = config.disabled_diagnostics(db).clone();
    let non_ascii = config.non_ascii_mode(db);
    let dialect = file.dialect(db).clone();
    let text = file.text(db).clone();
    let workspace_class_factories = file.workspace_class_factories(db).clone();
    let input = match document_analysis_input(db, file, config) {
        Ok(input) => input,
        Err(miss) => return unavailable_document_analysis(&dialect, miss),
    };
    let mut analyser = document_analyser(db, file, config).with_resolved_input((*input).clone());

    // Build the CFG/SSA tail's compilation unit with per-procedure lattices
    // memoised by `function_lattice`, and feed it through the analyser's
    // `cu_override` seam, via the shared [`compilation_unit`] query.  The
    // document's own environment grammar mirrors what
    // `emit_cfg_ssa_diagnostics` builds for itself, so the supplied unit is
    // the one it would otherwise build; routing through the tracked query lets
    // `compiler_check_diagnostics` reuse this exact build in the same edit,
    // for *every* environment, because both consumers intern the same
    // environment id.
    let cfg_key = lexer_cfg_key(db, &dialect);
    // The input was checked before either producer walked source. A missing
    // generation returns above; it cannot reopen standalone compilation.
    if let Some(unit) = compilation_unit_for_config(db, file, cfg_key, config) {
        analyser.set_cu_override(unit);
    }

    let mut body_fn = |body: &DeferredBody| -> BodyFragment {
        let key = ItemBodyKey::new(
            db,
            Arc::clone(&body.body_text),
            body.namespace.clone(),
            body.scope_name.clone(),
            body.params.clone(),
            body.is_method,
            body.oo_global_resolution,
            body.seeded_variables.clone(),
            body.command_trust
                .clone()
                .map(|trust| (trust, body.oo_defining_class.clone())),
            (
                (
                    body.ensemble_targets.clone(),
                    body.prefixless_ensembles.clone(),
                ),
                body.safe_interp_ctx.clone(),
                workspace_class_factories.clone(),
                body.resolved_input.clone(),
            ),
            dialect.clone(),
            disabled_vec.clone(),
            non_ascii,
        );
        // Owned for the same reason as the lattice memo: `graft_proc_body`
        // rebases the offset-0 fragment to the body's real span and then moves
        // its fields into the shell, so the fragment cannot be shared.
        (*item_body_analysis(db, key)).clone()
    };
    Arc::new(analyser.analyse_per_item_with(&text, &dialect, &mut body_fn))
}

/// The compiler-checks + optimiser diagnostics for one document, unfiltered.
///
/// Returned by [`compiler_check_diagnostics`]. Unfiltered: every surface
/// converts these to findings, and the policy step decides what shows.
/// Kept independent of the runtime gate so the query caches across config
/// toggles.  `Clone + PartialEq` for salsa early-cutoff.
#[derive(Clone, PartialEq)]
pub struct CompilerDiagnostics {
    /// `run_all_checks` output (GVN / shimmer / thunking / taint / iRules-flow /
    /// SCCP), severities preserved.
    pub checks: Vec<CompilerCheck>,
    /// `optimise_unit` rewrites (`O1xx`), surfaced as HINT-severity suggestions.
    pub optimisations: Vec<Optimisation>,
}

/// Run the compiler-checks + optimiser passes over a built unit.  Shared by the
/// memoised [`compiler_check_diagnostics`] query and the no-salsa-input
/// fallback so both produce byte-identical diagnostics.
fn compiler_diagnostics_from_unit(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    generic_patterns: Option<&[String]>,
) -> CompilerDiagnostics {
    let dialect_opt = dialect;
    CompilerDiagnostics {
        checks: tcl_compiler::compiler_checks::run_all_checks_with_generic_patterns(
            cu,
            registry,
            dialect_opt,
            generic_patterns,
        ),
        optimisations: tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt),
    }
}

/// Compiler-checks + optimiser diagnostics for one document, with the unit's
/// per-procedure lattices memoised by the salsa-native [`function_lattice`]
/// query (so an unchanged procedure is built once and shared with the analyser
/// tail).  The optimiser lowers with the dialect lexer config — distinct from
/// the analyser tail's default config, so the two intern different bodies and
/// never cross-pollute.  Byte-identical to
/// [`compiler_check_diagnostics_uncached`].
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn compiler_check_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<CompilerDiagnostics> {
    let dialect = file.dialect(db).clone();
    let Ok(input) = document_analysis_input(db, file, config) else {
        return abstain_diagnostics();
    };
    let registry: &CommandRegistry = input.borrowed_context_registry().commands();
    let dialect_opt = Some(input.unit_profile());
    // Share the analyser tail's build via the [`compilation_unit`] query when the
    // dialect's lexer config matches the default (every dialect but `tcl8.4` /
    // `f5-irules`): the optimiser lowers with the dialect config, so a matching
    // config interns the same `LexerCfgKey` and reuses the same per-edit build.
    let cfg_key = lexer_cfg_key(db, &dialect);
    let Some(cu) = compilation_unit_for_config(db, file, cfg_key, config) else {
        return abstain_diagnostics();
    };
    // Both halves of `run_all_checks` come from the memoised [`proc_taint_solve`]:
    // the interprocedural taint solve (`solve.taints`)
    // and the per-procedure non-taint checks (`solve.fn_checks`, already rebased),
    // so an unchanged procedure contributes neither a re-solve nor a re-check.
    // The remaining taint-family + iRules module checks (which read the solved
    // taints) are appended over the shared build, then the combined set is sorted
    // into the same deterministic order `run_all_checks` produces.  Byte-identical
    // to the in-line build; guarded by the corpus differential.  Optimiser
    // unchanged.
    let Some(solve) = proc_taint_solve_for_config(db, file, cfg_key, config) else {
        return abstain_diagnostics();
    };
    let mut checks = solve.fn_checks.clone();
    let generic_patterns = config.generic_variable_patterns(db).as_deref();
    tcl_compiler::compiler_checks::push_taint_and_module_checks(
        &cu,
        registry,
        dialect_opt,
        &solve.taints,
        generic_patterns,
        &mut checks,
    );
    tcl_compiler::compiler_checks::retain_diagnostic_source_context(&cu, registry, &mut checks);
    tcl_compiler::compiler_checks::sort_diagnostics(&mut checks);
    Arc::new(CompilerDiagnostics {
        checks,
        optimisations: solve.optimisations.clone(),
    })
}

/// No-salsa-input fallback for [`compiler_check_diagnostics`]: build the unit
/// directly (no per-procedure memoisation) and run the same passes.  Used when
/// a document has no [`SourceFile`] input yet (mirrors the analyser fallback).
#[must_use]
pub fn compiler_check_diagnostics_uncached(
    text: &str,
    registry: &CommandRegistry,
    dialect: &str,
    generic_patterns: Option<&[String]>,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilerDiagnostics {
    let dialect_opt = tcl_lsp_core::stated_profile_for_dialect(dialect);
    // No `SourceFile` here means no path, so only the document's own inline
    // block is reachable; a command only a sidecar declares answers as the
    // catalogue has it.
    let declared = tcl_compiler::analyser::utils::document_declared_surface(text, None, dialect);
    let cu = CompilationUnit::build_with_options(
        text,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(
                tcl_lsp_core::environment_for_dialect(dialect).grammar(),
            ),
            dialect: tcl_lsp_core::optional_profile_for_dialect(dialect),
            external_call_sites,
            declared_commands: Some(&declared),
        },
    )
    .with_interprocedural(registry, dialect_opt);
    compiler_diagnostics_from_unit(
        &cu,
        registry,
        tcl_lsp_core::stated_profile_for_dialect(dialect),
        generic_patterns,
    )
}

/// Build a source-advice unit from the document's complete retained analysis.
/// Missing, changed-source, foreign-store or changed-grammar ownership refuses
/// the build. This does not supply a native execution entry or erasure licence.
#[must_use]
pub fn compilation_unit_uncached_from_analysis(
    text: &str,
    registry: &CommandRegistry,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    external_call_sites: Option<&CallSiteEvidence>,
) -> Option<CompilationUnit> {
    if analysis.analysis_context_unavailable.is_some() {
        return None;
    }
    let (_image, config) = tcl_compiler::source_graph::current_analysis(text, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    )?;
    let declared = tcl_compiler::analyser::utils::document_declared_surface(
        text,
        None,
        input.unit_profile().name,
    );
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    );
    Some(CompilationUnit::build_with_analysis_input(
        text,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(input.unit_profile()),
            external_call_sites,
            declared_commands: Some(&declared),
        },
        Some(&entry),
        input,
    ))
}

/// Uncached compiler findings for an already analysed document. The actual
/// input owns availability, unit profile and lexer configuration. A refused
/// supplied build returns no compiler projections; it never enters the scalar
/// standalone API. Diagnostic subjects and source contexts stay intact.
#[must_use]
pub fn compiler_check_diagnostics_uncached_from_analysis(
    text: &str,
    registry: &CommandRegistry,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    generic_patterns: Option<&[String]>,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilerDiagnostics {
    let Some(cu) =
        compilation_unit_uncached_from_analysis(text, registry, analysis, external_call_sites)
    else {
        return CompilerDiagnostics {
            checks: Vec::new(),
            optimisations: Vec::new(),
        };
    };
    let profile = analysis
        .resolved_input
        .as_ref()
        .map(|input| input.unit_profile());
    let cu = cu.with_interprocedural(registry, profile);
    compiler_diagnostics_from_unit(&cu, registry, profile, generic_patterns)
}

/// Document outline — wraps `document_symbols_from_analysis`, reusing the
/// tracked [`file_analysis_incremental`] so the outline shares the per-item
/// memoised analysis with the push-diagnostics path in the same edit.
// `returns(clone)`, not `returns(ref)`: the caller must be able to move the
// result out of `Cancelled::catch` and project it into `lsp-types` *outside*
// the read, because that projection has no cancellation checkpoint and a read
// held across it blocks a concurrent `set_text`. See the crate docs' "Return
// modes".
#[salsa::tracked(returns(clone))]
pub fn document_symbols(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<DocumentSymbol> {
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::document_symbols::document_symbols_from_analysis(file.text(db), &analysis)
}

/// The document's [`CompilationUnit`] under the default lexer config and no
/// pack overlay — a thin wrapper over [`compilation_unit`] that interns the
/// `LexerCfgKey` from the file's dialect, for the server-side accessors that
/// only have `(db, file)`. It shares the diagnostics path's memoised build
/// when the workspace has no packs; a caller holding an [`AnalyserConfig`]
/// uses [`document_compilation_unit_for`] to share it with packs too.
// LRU-capped: per-file key, see the crate docs' "Deep-memo eviction".
#[salsa::tracked(lru = 64, returns(clone))]
pub fn document_compilation_unit(db: &dyn TclDb, file: SourceFile) -> Arc<CompilationUnit> {
    let cfg_key = lexer_cfg_key(db, file.dialect(db));
    compilation_unit(db, file, cfg_key, 0).expect("a unit with no pack overlay always builds")
}

/// The document's [`CompilationUnit`] under the default lexer config and
/// `config`'s pack overlay: the diagnostics path's memoised build, which a
/// consumer resolving against the overlaid registry reads so the unit and
/// the registry agree on which commands the workspace's packs declare.
///
/// `None` when the packs are not installed ([`compilation_unit`]).
#[must_use]
pub fn document_compilation_unit_for(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<CompilationUnit>> {
    let cfg_key = lexer_cfg_key(db, file.dialect(db));
    compilation_unit_for_config(db, file, cfg_key, config)
}

/// Semantic tokens — wraps `semantic_tokens::full_with_cu`; reads the durable
/// registry.
///
/// This query demands the document's [`CompilationUnit`] so a `regexp` /
/// `regsub` pattern supplied through a provably-constant string variable
/// highlights its originating `set` literal as a regex (see
/// [`tcl_compiler::regex_source`]), and the whole-file analysis so a
/// `$obj method …` / `[dict get $objs $k] method …` dispatch resolves against
/// user classes and their `oo::configurable` properties, not only
/// registry ones. Using the coarse, non-incremental [`file_analysis`] for
/// this instead of [`file_analysis_incremental`] would cost every token
/// request a *third* independent whole-file analyser walk (on top of
/// the two the diagnostics path already shares via [`compilation_unit`]), and
/// that walk has no interior salsa cancellation checkpoint, so a concurrent
/// edit's `set_text` would block until it finishes.  Using
/// [`file_analysis_incremental`] here instead gets the same correctness
/// (identical `AnalysisResult` shape, proven byte-identical to `file_analysis`
/// by the `per_item_corpus` gate) while sharing the diagnostics path's
/// per-item memoisation and cancellation checkpoints: a token request that
/// lands after diagnostics have already analysed this revision is a cache
/// hit, and a cold request is preemptible by the next edit instead of running
/// an uninterruptible pass to completion.
// `returns(clone)`: same liveness rule as the other bare-value queries, and the
// payload is a flat `Vec<u32>` the handler hands to the JSON-RPC layer by value
// anyway, so a borrow would buy nothing even if it were safe.
#[salsa::tracked(returns(clone))]
pub fn semantic_tokens(db: &dyn TclDb, file: SourceFile, config: AnalyserConfig) -> SemanticTokens {
    let registry = token_registry(db, file.dialect(db), config.spec_pack_key(db));
    let cu = document_compilation_unit_for(db, file, config);
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
        file.text(db),
        tcl_lsp_core::profile_for_dialect(file.dialect(db)),
        &registry,
        cu.as_deref(),
        Some(&analysis),
    )
}

mod project_call_inputs;
pub use project_call_inputs::{
    file_external_call_sites_for_inputs, file_source_targets_for_inputs,
};
mod project_source_inputs;
use project_source_inputs::{
    command_arity_for_inputs, original_command_signatures_for_inputs,
    original_lookup_command_signatures_for_inputs,
};
pub use project_source_inputs::{
    item_sigs_for_config, item_tree_for_config,
    project_callback_diagnostics_for_analysis_with_inputs, project_callback_diagnostics_for_inputs,
    project_class_factories_for_inputs, project_original_command_signatures_for_inputs,
};

mod project_token_inputs;
pub use project_token_inputs::{
    file_token_facts_for_config, project_class_index_for_inputs,
    project_named_instance_index_for_inputs, project_proc_var_index_for_inputs,
    semantic_tokens_project_for_inputs,
};

/// The cross-file facts the project-level token aggregates read from **one**
/// file: its class definitions and its inferred variable-name argument roles.
///
/// Deliberately small and `PartialEq`: it is the per-file firewall in front of
/// [`project_class_index`] / [`project_proc_var_index`], so an edit that leaves
/// a file's classes and parameter roles alone — which is nearly every keystroke
/// — backdates here and the project aggregates do not re-execute at all.
#[derive(Clone, Default, PartialEq)]
pub struct FileTokenFacts {
    /// Classes declared in this file, keyed by qualified name.
    pub classes: HashMap<String, ClassDef>,
    /// The file's user-proc parameter roles.
    pub proc_roles: VarNameArgRoles,
    /// Bareword instance-command names this file binds via `CLASS create
    /// NAME`, mapped to the (locally-resolved) qualified class name — the
    /// named-object dispatch form, gated on
    /// `created_instance_commands` exactly like the LSP's
    /// `receiver_instance_class`.
    pub named_instances: HashMap<String, String>,
}

/// The light (structure-only) per-file tier feeding [`project_class_index`] and
/// [`project_proc_var_index`].
///
/// These aggregates read *every* file in the project, so whatever per-file
/// query they call decides what an interactive `semanticTokens` request costs
/// on a large workspace.  Reading the deep tier
/// ([`file_analysis_incremental`]) instead would mean one whole-workspace
/// *deep* analysis — CFG/SSA units, per-body lattices, diagnostic emitters —
/// behind a token request for a single file: measured at ~19 s of CPU over an
/// 883-file tcllib checkout, which no request survives.  Every attempt would
/// be cancelled by the next `set_text`, so it would never memoise and every
/// subsequent request would pay it again, while the in-flight read would
/// block the writer that cancelled it (`didOpen`'s `set_text` measured at
/// 270–660 ms) — producing open-to-tokens latency spikes.
///
/// The structural facts these aggregates want do not need the deep tier: an
/// unopened workspace file only ever needs the lightweight state the scan
/// leaves it at.  `structure_only` builds the identical declaration structure
/// while skipping diagnostic emission and cross-feature recording — the bulk of
/// the cost — for the same 883 files in ~2.9 s, and the result is a projection
/// small enough to backdate.
///
/// This explicit standalone query uses the file's stated dialect. Actual
/// document consumers use [`file_token_facts_for_config`], whose checked input
/// retains pack roles, availability and source grammar on the same light tier.
#[salsa::tracked(returns(clone))]
pub fn file_token_facts(db: &dyn TclDb, file: SourceFile) -> Arc<FileTokenFacts> {
    let mut analyser = Analyser::new()
        .structure_only()
        .with_file_path(file.path(db).clone());
    let result = analyser.analyse(file.text(db), file.dialect(db));
    token_facts_from_analysis(result)
}

fn token_facts_from_analysis(result: AnalysisResult) -> Arc<FileTokenFacts> {
    let named_instances = result
        .instance_classes
        .iter()
        .filter(|(name, _)| result.created_instance_commands.contains(name.as_str()))
        .map(|(name, class)| (name.clone(), class.clone()))
        .collect();
    Arc::new(FileTokenFacts {
        proc_roles: VarNameArgRoles::from_analysis(&result),
        classes: result.all_classes,
        named_instances,
    })
}

/// The project's workspace-merged class hierarchy: every file's `ClassDef`s
/// unioned into one cross-file MRO index, so a `$obj method` dispatch resolves
/// against a class defined in *another* file.
///
/// Aggregates [`file_token_facts`]`(f).classes` across `project`.  A body-only
/// edit backdates at the per-file firewall, so this does not even re-aggregate.
/// On a class-signature change the merged index changes and the affected tokens
/// recompute — the correct cross-file invalidation.
#[salsa::tracked(returns(clone))]
pub fn project_class_index(db: &dyn TclDb, project: Project) -> Arc<ClassHierarchy> {
    merge_project_classes(
        project
            .files(db)
            .iter()
            .map(|&file| file_token_facts(db, file)),
    )
}

fn merge_project_classes(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<ClassHierarchy> {
    // `project.files(db)` is an unordered `Vec` with no stable identity, so a
    // "first definition wins" merge would make the winner for a duplicate
    // qualified class name depend on file-enumeration order — non-deterministic
    // cross-file resolution (and token output).  Instead, abstain: a qualified
    // name defined in two or more files is genuinely ambiguous (which `::Foo`
    // does `$obj method` mean?), so drop it from the merged index and fall back
    // to no cross-file resolution — order-independent and sound, matching this
    // feature's highlight-only / sound-by-abstention posture.
    let mut merged: HashMap<String, ClassDef> = HashMap::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    for facts in facts {
        for (name, class) in &facts.classes {
            if ambiguous.contains(name) {
                continue;
            }
            match merged.entry(name.clone()) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    // A second file defines the same qualified name — ambiguous.
                    e.remove();
                    ambiguous.insert(name.clone());
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(class.clone());
                }
            }
        }
    }
    Arc::new(build_class_hierarchy(merged))
}

/// The project's workspace-merged inferred variable-name argument roles: every
/// file's user-proc parameter roles — a parameter the analyser inferred to
/// alias a caller variable (`upvar $param` + write) — unioned into one
/// cross-file index, so a `myproc arr(key) …` call highlights its array-element
/// target even when `myproc` is defined in another file.
///
/// A proc name defined with *conflicting* roles across files is dropped as
/// ambiguous by [`VarNameArgRoles::merge`], so the merged index is
/// order-independent — matching the abstention posture of
/// [`project_class_index`].
///
/// Reads the same light [`file_token_facts`] firewall as [`project_class_index`].
#[salsa::tracked(returns(clone))]
pub fn project_proc_var_index(db: &dyn TclDb, project: Project) -> Arc<VarNameArgRoles> {
    merge_project_proc_roles(
        project
            .files(db)
            .iter()
            .map(|&file| file_token_facts(db, file)),
    )
}

fn merge_project_proc_roles(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<VarNameArgRoles> {
    let per_file = facts.into_iter().collect::<Vec<_>>();
    Arc::new(VarNameArgRoles::merge(
        per_file.iter().map(|facts| &facts.proc_roles),
    ))
}

/// The project's workspace-merged `CLASS create NAME` bareword
/// instance-command index: every file's `named_instances`
/// unioned into one cross-file map, so `$obj method` on a named object
/// bound in *another* project file's file resolves too.
///
/// A name bound to a *different* class in two or more files is genuinely
/// ambiguous — dropped from the merged index, matching
/// [`project_class_index`]'s order-independent abstention posture, rather
/// than depending on `project.files`' unordered enumeration.
///
/// Reads the same light [`file_token_facts`] firewall as [`project_class_index`].
#[salsa::tracked(returns(clone))]
pub fn project_named_instance_index(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    merge_project_named_instances(
        project
            .files(db)
            .iter()
            .map(|&file| file_token_facts(db, file)),
    )
}

fn merge_project_named_instances(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    let mut merged: HashMap<String, String> = HashMap::new();
    let mut ambiguous: HashSet<String> = HashSet::new();
    for facts in facts {
        for (name, class) in &facts.named_instances {
            if ambiguous.contains(name) {
                continue;
            }
            match merged.entry(name.clone()) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    if e.get() != class {
                        e.remove();
                        ambiguous.insert(name.clone());
                    }
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(class.clone());
                }
            }
        }
    }
    Arc::new(merged)
}

/// [`semantic_tokens`] resolved against the **workspace-merged** class index, so
/// a `$obj method …` dispatch on a class defined in another project file
/// resolves too.  The server calls this when a [`Project`] is available; the
/// bare [`semantic_tokens`] (local file only) is the fallback.
///
/// This compatibility query keeps standalone cross-file indexes. Actual
/// workspace document callers use [`semantic_tokens_project_for_inputs`] and
/// the driver's per-file configuration map.
// `returns(clone)` for the same reason as [`semantic_tokens`].
#[salsa::tracked(returns(clone))]
pub fn semantic_tokens_project(
    db: &dyn TclDb,
    file: SourceFile,
    project: Project,
    config: AnalyserConfig,
) -> SemanticTokens {
    let registry = token_registry(db, file.dialect(db), config.spec_pack_key(db));
    let cu = document_compilation_unit_for(db, file, config);
    let classes = project_class_index(db, project);
    let proc_roles = project_proc_var_index(db, project);
    let named_instances = project_named_instance_index(db, project);
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_facts(
        file.text(db),
        tcl_lsp_core::profile_for_dialect(file.dialect(db)),
        &registry,
        cu.as_deref(),
        tcl_lsp_core::semantic_tokens::WorkspaceTokenFacts {
            classes: Some(&classes),
            proc_roles: Some(&proc_roles),
            named_instances: Some(&named_instances),
            analysis: Some(&analysis),
        },
    )
}

/// Folding ranges — wraps `folding::folding_ranges`; reads the durable registry.
// `returns(clone)` for the same liveness reason as [`document_symbols`]: this
// query has no interior cancellation checkpoint at all (it is a single
// straight-line segmenter walk), so the read region must stay as short as
// possible and the projection must happen outside it.
#[salsa::tracked(returns(clone))]
pub fn folding_ranges(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<FoldingRange> {
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::folding::folding_ranges_with_analysis(file.text(db), &analysis)
}

#[cfg(test)]
mod value_transfer_parity;

#[cfg(test)]
mod original_uncached_metadata_tests;

#[cfg(test)]
mod original_tracked_metadata_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A key that misses, installs, retires and misses again is reported each
    /// time it misses: the log keeps a miss once while it stands, and forgets
    /// it when the key installs.
    #[test]
    fn a_key_that_installs_is_reported_again_if_it_misses_later() {
        let generations = OverlayGenerations::default();
        let key: OverlayKey = ("tcl9.0".to_owned(), 0xAB);
        let miss = OverlayMiss {
            environment: key.0.clone(),
            overlay: key.1,
        };
        let take = |generations: &OverlayGenerations| {
            std::mem::take(&mut generations.misses.lock().expect("miss log").unreported)
        };
        generations.record(&miss);
        generations.record(&miss);
        assert_eq!(
            take(&generations),
            vec![miss.clone()],
            "one record while it stands"
        );

        let registry = Arc::new(CommandRegistry::build_default());
        generations.hold(key.clone(), Arc::clone(&registry));
        // Retired: later keys push it out of the held generations.
        let later = u64::try_from(HELD_OVERLAYS).expect("a small count");
        for overlay in 0..=later {
            generations.hold(
                ("tcl9.0".to_owned(), 0x1000 + overlay),
                Arc::clone(&registry),
            );
        }
        assert!(generations.held(&key).is_none(), "the key was retired");
        generations.record(&miss);
        assert_eq!(
            take(&generations),
            vec![miss],
            "a miss after the key installed is a new one"
        );
    }

    fn cfg(db: &TclDatabase) -> AnalyserConfig {
        AnalyserConfig::new(
            db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        )
    }

    const SRC: &str = "proc greet {name} {\n    puts \"hi $name\"\n}\n# c\nset x 1\n";

    #[test]
    fn original_callback_missing_future_lookup_declines_reporting_candidates() {
        // Implementation contract naming.database.original-callback-future-lookup:
        // docs/design/analysis/name-resolution-proofs/database-original-callback-future-lookup.md
        let db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            "proc cb {} {}\ncb\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let analysis = file_analysis_incremental(&db, file, cfg(&db));
        let mut invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| invocation.name == "cb" && invocation.original_name_input.is_some())
            .expect("an actual original command producer")
            .clone();
        // This checks missing-purpose refusal, not callback context inference:
        // the independently retained name cannot manufacture a future lookup.
        invocation.original_lookup = None;
        invocation.resolved_command_reference = None;
        invocation.callback_arity = Some(tcl_registry::AppendedArity::Exactly(2));
        let arities = HashMap::from([("cb".to_owned(), vec![(0, 0)])]);
        assert!(callback_command_keys(&invocation).is_empty());
        assert!(
            apply_project_callback_arity(&analysis, &[], &[invocation], &arities, |_| false)
                .is_empty()
        );
    }

    #[test]
    fn formal_count_project_headers_keep_the_selected_c_and_jim_contract() {
        // naming.database.original-formal-count-header
        // docs/design/analysis/name-resolution-proofs/database-original-formal-count-header.md
        use salsa::Setter as _;
        use tcl_compiler::signature_scan::formal_count::SourceFormalCountOrigin;
        for (dialect, minimum, grammar) in [
            ("tcl8.4", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl8.5", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl8.6", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl9.0", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("tcl9.1", 3, tcl_dialect::ParameterGrammar::Tcl),
            ("jim", 2, tcl_dialect::ParameterGrammar::Jim),
        ] {
            let mut db = TclDatabase::default();
            let file = SourceFile::new(
                &db,
                "proc mixed {a {b B} c} {return FIRST}".to_owned(),
                dialect.to_owned(),
                None,
            );
            let project = Project::new(&db, vec![file]);
            let headers = item_sigs(&db, file);
            let signature = &headers[0];
            let header = signature.original_declaration.as_ref().unwrap();
            let count = header.formal_count_projection();
            assert_eq!(
                count.origin(),
                SourceFormalCountOrigin::OriginalSource,
                "{dialect}"
            );
            assert_eq!(count.parameter_grammar(), Some(grammar), "{dialect}");
            let key = header.name().clone();
            let original = project_original_command_arities(&db, project);
            assert_eq!(original.get(&key), Some(&vec![(minimum, 3)]), "{dialect}");
            let reporting = project_command_arities(&db, project);
            assert_eq!(
                reporting.get("mixed"),
                Some(&vec![(minimum, 3)]),
                "{dialect}"
            );
            file.set_text(&mut db)
                .to("proc mixed {a {b B} c} {return A_LONGER_BODY}".to_owned());
            assert_eq!(
                project_original_command_arities(&db, project),
                original,
                "{dialect}"
            );
            assert_eq!(
                project_command_arities(&db, project),
                reporting,
                "{dialect}"
            );
            assert_eq!(item_sigs(&db, file), headers, "{dialect}");
        }
    }

    #[test]
    fn original_project_headers_keep_opaque_slots_and_body_free_arity() {
        // Proof naming.database.original-header-arity:
        // docs/design/analysis/name-resolution-proofs/database-original-header-arity.md
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            r"proc p\uD800 {a} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![file]);
        let declarations = file_decls(&db, file);
        let first = declarations
            .original_declarations
            .iter()
            .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x80")
            .expect("first original byte declaration")
            .name()
            .clone();
        let second = declarations
            .original_declarations
            .iter()
            .find(|header| header.name().slot().simple.as_bytes() == b"p\xed\xa0\x81")
            .expect("second original byte declaration")
            .name()
            .clone();
        let first_key = OriginalCommandSlot::new(&db, first.policy(), first.slot().clone());
        let second_key = OriginalCommandSlot::new(&db, second.policy(), second.slot().clone());
        assert_eq!(
            original_command_arity(&db, project, first_key)
                .unwrap()
                .as_ref(),
            &[(1, 1)]
        );
        assert_eq!(
            original_command_arity(&db, project, second_key)
                .unwrap()
                .as_ref(),
            &[(2, 2)]
        );
        let wrong_provider = OriginalCommandSlot::new(
            &db,
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_5),
            first.slot().clone(),
        );
        assert!(original_command_arity(&db, project, wrong_provider).is_none());
        let before = project_original_command_arities(&db, project);
        file.set_text(&mut db)
            .to(r"proc p\uD800 {a} {return A_LONGER_BODY}
proc p\uD801 {a b} {return ANOTHER_BODY}"
                .to_owned());
        assert_eq!(project_original_command_arities(&db, project), before);
        assert_eq!(
            original_command_arity(
                &db,
                project,
                OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
            )
            .unwrap()
            .as_ref(),
            &[(1, 1)]
        );
        file.set_text(&mut db)
            .to(r"proc p\uD800 {a b c} {return FIRST}
proc p\uD801 {a b} {return SECOND}"
                .to_owned());
        assert_eq!(
            original_command_arity(
                &db,
                project,
                OriginalCommandSlot::new(&db, first.policy(), first.slot().clone())
            )
            .unwrap()
            .as_ref(),
            &[(3, 3)]
        );
        assert_eq!(
            original_command_arity(
                &db,
                project,
                OriginalCommandSlot::new(&db, second.policy(), second.slot().clone())
            )
            .unwrap()
            .as_ref(),
            &[(2, 2)]
        );
    }

    #[test]
    fn original_project_diagnostics_select_opaque_call_slots_and_namespace_priority() {
        // Implementation contract: naming.database.original-call-arity-diagnostics
        // docs/design/analysis/name-resolution-proofs/database-original-call-arity-diagnostics.md
        let db = TclDatabase::default();
        let config = cfg(&db);
        let provider = SourceFile::new(
            &db,
            r"proc p\uD800 {a} {}; proc p\uD801 {a b} {};
namespace eval N {proc only_here {a} {}}"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let caller = SourceFile::new(
            &db,
            r"p\uD800 A B
p\uD801 A B"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![provider, caller]);
        let analysis = file_analysis_incremental(&db, caller, config);
        let calls = analysis
            .command_invocations
            .iter()
            .filter(|invocation| invocation.original_name_input.is_some())
            .collect::<Vec<_>>();
        let first = calls
            .iter()
            .find(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
            })
            .expect("original first call");
        let second = calls
            .iter()
            .find(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x81")
            })
            .expect("original second call");
        let diagnostics = project_diagnostics(&db, caller, config, project);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagCode::E003
                    && diagnostic.span == first.range),
            "first byte name accepts exactly one argument"
        );
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.span == second.range),
            "distinct second byte name accepts two arguments"
        );
        // Namespace advice is an independent source point. The external
        // calls above do not close the world for a following command.
        let unrelated = SourceFile::new(&db, "only_here A".to_owned(), "tcl8.6".to_owned(), None);
        let unrelated_project = Project::new(&db, vec![provider, unrelated]);
        let unrelated_diagnostics = project_diagnostics(&db, unrelated, config, unrelated_project);
        let unresolved = unrelated_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W123)
            .collect::<Vec<_>>();
        assert_eq!(
            unresolved.len(),
            1,
            "unrelated namespace tails cannot suppress this call"
        );
        assert_eq!(
            unresolved[0]
                .unresolved_command()
                .unwrap()
                .name_input()
                .bytes(),
            b"only_here"
        );
    }

    #[test]
    fn original_db_token_roles_keep_opaque_declarations_and_full_caller_currency() {
        // Implementation contract: naming.database.original-token-role-consumer
        // docs/design/analysis/name-resolution-proofs/database-original-token-role-consumer.md
        use tcl_lsp_core::semantic_tokens::{
            WorkspaceTokenFacts, full_with_cu_and_facts, legend_token_types,
        };
        let source = r"proc p\uD800 {target} {upvar 1 $target cell; set cell 1}
proc p\uD801 {target} {return $target}
p\uD800 written
p\uD801 ordinary";
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, source.to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![file]);
        let facts = file_token_facts(&db, file);
        let roles = project_proc_var_index(&db, project);
        assert!(
            !facts.proc_roles.is_empty(),
            "the retained writer declaration has a role"
        );
        assert_eq!(facts.proc_roles, *roles);
        let mut analysis = (*file_analysis_incremental(&db, file, cfg(&db))).clone();
        analysis.all_procs.clear();
        analysis.superseded_procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "REPORT-COLLISION".to_owned();
            invocation.resolution_candidates = vec!["REPORT-COLLISION".to_owned()];
        }
        let registry = db.registry(file.dialect(&db));
        let profile = tcl_lsp_core::profile_for_dialect(file.dialect(&db));
        let unit = document_compilation_unit(&db, file);
        let render = |image: &str, current: &AnalysisResult| {
            full_with_cu_and_facts(
                image,
                profile,
                registry,
                Some(&unit),
                WorkspaceTokenFacts {
                    proc_roles: Some(&roles),
                    analysis: Some(current),
                    ..Default::default()
                },
            )
        };
        let variable = u32::try_from(
            legend_token_types()
                .iter()
                .position(|kind| *kind == "variable")
                .unwrap(),
        )
        .unwrap();
        let is_variable = |tokens: &SemanticTokens, image: &str, needle: &str| {
            let offset = u32::try_from(image.rfind(needle).unwrap()).unwrap();
            let target = tcl_lexer::LineIndex::new(image).position_at_utf16(offset, image);
            let mut line = 0;
            let mut column = 0;
            tokens.data.chunks_exact(5).any(|entry| {
                if entry[0] == 0 {
                    column += entry[1];
                } else {
                    line += entry[0];
                    column = entry[1];
                }
                line == target.line
                    && column <= target.character.get()
                    && target.character.get() < column + entry[2]
                    && entry[3] == variable
            })
        };
        let tokens = render(source, &analysis);
        assert!(
            is_variable(&tokens, source, "written"),
            "the exact opaque writer retains its caller argument role after reporting clear"
        );
        assert!(
            !is_variable(&tokens, source, "ordinary"),
            "the distinct opaque ordinary procedure does not borrow the writer role"
        );
        let changed = format!("{source} ");
        assert!(!is_variable(
            &render(&changed, &analysis),
            &changed,
            "written"
        ));
        let mut changed_config = analysis.clone();
        changed_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting ^= true;
        assert!(!is_variable(
            &render(source, &changed_config),
            source,
            "written"
        ));
        let mut missing_input = analysis;
        let call = u32::try_from(source.rfind(r"p\uD800 written").unwrap()).unwrap();
        missing_input
            .command_invocations
            .iter_mut()
            .find(|invocation| invocation.range.start() == call)
            .unwrap()
            .original_name_input = None;
        assert!(!is_variable(
            &render(source, &missing_input),
            source,
            "written"
        ));
    }

    #[test]
    fn body_cache_gate_is_per_body_and_whitespace_aware() {
        use tcl_compiler::lowering::body_cache_eligible;
        // A plain, context-free body is eligible.
        assert!(body_cache_eligible(" set x 1 "));
        assert!(body_cache_eligible(" puts hi "));
        // A body carrying a cross-item construct is not — including the tab-
        // separated forms: the isolated lowerer drops the effect.
        assert!(!body_cache_eligible(" interp\talias {} x {} y "));
        assert!(!body_cache_eligible(" namespace\timport ::ns::* "));
        assert!(!body_cache_eligible(" rename set myset "));
        assert!(!body_cache_eligible(" oo::class create C "));
        // A nested `proc` disqualifies the enclosing body.
        assert!(!body_cache_eligible(" proc inner {} {} "));
        // The gate is per-body: a context-carrying sibling does not disable the
        // clean body — the clean body stays eligible on its own.
        assert!(body_cache_eligible(" set y 2 "));
    }

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
    fn file_analysis_matches_direct_analyse() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = file_analysis(&db, file, cfg(&db));

        let mut direct = Analyser::new();
        let expected = direct.analyse(SRC, "tcl");
        assert_eq!(*got, expected);
        assert!(got.all_procs.contains_key("::greet"));
    }

    /// A memo *hit* must hand its per-procedure lattices over by
    /// refcount, not deep-copy them.
    ///
    /// The span-carrying halves of a `FunctionUnit` (`cfg` / `ssa` /
    /// `sccp.constant_branches`) genuinely have to be copied — the memo stores
    /// the unit at offset 0 and every consumer is rebased to the procedure's
    /// real position — but `def_use` / `types` / `taints` / `rendered_props` are
    /// span-free, so a rebuild that hits the memo must share them.
    ///
    /// Two *distinct* `SourceFile`s carrying the same text give two distinct
    /// `compilation_unit` keys, so both really build — while their per-procedure
    /// lattice demands land on the same `function_lattice` memos.  Driving it
    /// through the tracked query rather than calling `memoised_compilation_unit`
    /// directly is deliberate: the per-body keys are only garbage-collectable
    /// when interned inside a tracked query (see the crate docs' "The interned
    /// garbage collector is load-bearing").
    #[test]
    fn memoised_lattices_are_shared_not_deep_copied_across_builds() {
        const SRC: &str = "proc alpha {a b} {\n    set s [expr {$a + $b}]\n    return $s\n}\n\
                           proc beta {x} {\n    return [alpha $x 1]\n}\n";
        let db = TclDatabase::default();
        let cfg_key = lexer_cfg_key(&db, "tcl8.6");
        let file_a = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
        let file_b = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
        let first = compilation_unit(&db, file_a, cfg_key, 0).expect("no overlay always builds");
        let second = compilation_unit(&db, file_b, cfg_key, 0).expect("no overlay always builds");
        for qname in ["::alpha", "::beta"] {
            let a = first
                .procedures
                .get(qname)
                .expect("procedure in first build");
            let b = second
                .procedures
                .get(qname)
                .expect("procedure in second build");
            assert!(
                Arc::ptr_eq(&a.def_use, &b.def_use),
                "{qname}: def-use chains must be shared across a memo hit",
            );
            assert!(
                Arc::ptr_eq(&a.types, &b.types),
                "{qname}: the type lattice must be shared across a memo hit",
            );
            assert!(
                Arc::ptr_eq(&a.rendered_props, &b.rendered_props),
                "{qname}: rendered properties must be shared across a memo hit",
            );
            assert!(
                Arc::ptr_eq(&a.taints, &b.taints),
                "{qname}: the taint cascade result must be shared across a memo hit",
            );
        }
    }

    /// A per-procedure lattice is rebuilt through the public single-function
    /// CFG entry point, so that entry point must receive the document's exact
    /// registry surface. Tcl 8.4 has no builtin `throw`: a user procedure with
    /// that name returns normally and the following statement stays reachable.
    #[test]
    fn tcl84_memoised_cfg_keeps_user_throw_call_fallthrough_reachable() {
        const SRC: &str = "proc throw {} {return ok}\n\
                           proc subject {} {throw; set after_throw 1}\n";
        let db = TclDatabase::default();
        let cfg_key = lexer_cfg_key(&db, "tcl8.4");
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.4".to_owned(), None);
        let unit = compilation_unit(&db, file, cfg_key, 0).expect("no overlay always builds");
        let subject = unit
            .procedures
            .get("::subject")
            .expect("subject procedure lattice");

        assert!(
            subject.cfg.blocks.values().any(|block| {
                block.statements.iter().any(|stmt| {
                    matches!(
                        stmt,
                        tcl_compiler::ir::Statement::AssignConst { name, .. }
                            if name == "after_throw"
                    )
                })
            }),
            "Tcl 8.4 user proc `throw` must fall through to `after_throw`"
        );
    }

    #[test]
    // Implementation contract: naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    // Implementation contract: naming.compiler.original-function-lattice-mutation-identity
    // docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
    fn original_event_lattice_cache_retains_conditional_entry_and_reuses_same_source() {
        let source = "when HTTP_REQUEST {set local 1}";
        let db = TclDatabase::default();
        let config = lexer_cfg_key(&db, "f5-irules");
        let first_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
        let second_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
        let first = compilation_unit(&db, first_file, config);
        let second = compilation_unit(&db, second_file, config);
        let a = first.procedures.get("::when::HTTP_REQUEST").unwrap();
        let b = second.procedures.get("::when::HTTP_REQUEST").unwrap();
        assert!(
            Arc::ptr_eq(&a.def_use, &b.def_use),
            "the sealed event participates in the lattice memo"
        );
        for unit in [a, b] {
            let event = unit
                .irules_event_body
                .as_ref()
                .expect("actual event producer");
            assert_eq!(event.event(), "HTTP_REQUEST");
            assert!(event.matches_source(
                &tcl_lexer::SourceImage::document(source),
                first.ir_module.lexer_config
            ));
            let (&block, _) = unit
                .cfg
                .blocks
                .iter()
                .find(|(_, block)| !block.statements.is_empty())
                .unwrap();
            let point = unit
                .ssa
                .point_contexts
                .as_ref()
                .unwrap()
                .context_before(block, 0)
                .unwrap();
            assert_eq!(
                point.frame_kind,
                tcl_compiler::var_resolve::VariableFrameKind::Local
            );
            assert_eq!(
                point.hosted_execution_context,
                Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
            );
            assert_eq!(
                point.execution, None,
                "memoisation cannot manufacture an executing worker"
            );
        }
    }

    #[test]
    // Implementation contract: naming.variable.registry-event-frame-identity
    // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
    fn original_event_lattice_key_rejects_foreign_source_body_config_and_registry() {
        let source = "when HTTP_REQUEST {set local 1}";
        let db = TclDatabase::default();
        let config_key = lexer_cfg_key(&db, "f5-irules");
        let file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
        let unit = compilation_unit(&db, file, config_key);
        let qname = "::when::HTTP_REQUEST";
        let event = unit.ir_module.irules_event_bodies.get(qname).unwrap();
        let procedure = unit.ir_module.procedures.get(qname).unwrap();
        assert_eq!(
            procedure.span.start(),
            0,
            "this actual declaration already owns the normalization origin"
        );
        let config = unit.ir_module.lexer_config.nested().normalized();
        let context = CfgContext::new(
            &db,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ModuleCommandBindings::default(),
        );
        let snapshot = CompilerMemoSnapshot {
            profile: Some(tcl_dialect::DialectProfile::irules().cache_key()),
            registry: db.registry("f5-irules").snapshot(),
        };
        let make_key = |body: Script, config, snapshot, event| {
            FnLatticeKey::new(
                &db,
                body,
                qname.to_owned(),
                Vec::new(),
                context,
                config,
                "f5-irules".to_owned(),
                snapshot,
                Vec::new(),
                FnLatticeModuleFacts {
                    known_classes: Vec::new(),
                    traced_variables: Vec::new(),
                    has_dynamic_variable_trace: false,
                },
                FnLatticeEntry {
                    plain_command_dispatch: false,
                    source_metadata_input: unit.ir_module.source_metadata_input.clone(),
                    irules_event_body: event,
                    command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                        .snapshot(),
                },
                ValueTransferContext::of(
                    &db,
                    tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                        &unit.command_mutations,
                        unit.ir_module.resolved_registry(),
                    ),
                    0,
                ),
            )
        };
        let selected = make_key(
            procedure.body.clone(),
            config,
            snapshot.clone(),
            Some(Arc::clone(event)),
        );
        let plain = make_key(procedure.body.clone(), config, snapshot.clone(), None);
        assert!(
            selected != plain,
            "the same name and body cannot donate event entry"
        );
        // Implementation contract: naming.compiler.original-function-lattice-mutation-identity
        // docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
        let changed_mutations = FnLatticeKey::new(
            &db,
            selected.body(&db).clone(),
            selected.qname(&db).clone(),
            selected.params(&db).clone(),
            selected.context(&db),
            selected.lexer_config(&db),
            selected.dialect(&db).clone(),
            selected.snapshot(&db).clone(),
            selected.param_constants(&db).clone(),
            selected.module_facts(&db).clone(),
            FnLatticeEntry {
                plain_command_dispatch: selected.entry(&db).plain_command_dispatch,
                source_metadata_input: selected.entry(&db).source_metadata_input.clone(),
                irules_event_body: selected.entry(&db).irules_event_body.clone(),
                command_trust: tcl_compiler::command_binding::ModuleCommandMutations::distrust_all(
                )
                .snapshot(),
            },
            selected.analysis_context(&db),
        );
        assert!(
            selected != changed_mutations,
            "unchanged event/source/Registry/body cannot erase mutation obligations"
        );
        assert!(!function_lattice(&db, selected).complexity_guarded);
        assert!(function_lattice(&db, selected).irules_event_body.is_some());
        assert!(function_lattice(&db, plain).irules_event_body.is_none());
        let changed_file = SourceFile::new(
            &db,
            format!("{source}\n# changed source"),
            "f5-irules".to_owned(),
            None,
        );
        let changed_unit = compilation_unit(&db, changed_file, config_key);
        let foreign = make_key(
            procedure.body.clone(),
            config,
            snapshot.clone(),
            changed_unit
                .ir_module
                .irules_event_bodies
                .get(qname)
                .cloned(),
        );
        assert!(
            selected != foreign,
            "complete original source participates in key equality"
        );
        assert!(function_lattice(&db, foreign).complexity_guarded);
        let wrong_body = make_key(
            Script::default(),
            config,
            snapshot.clone(),
            Some(Arc::clone(event)),
        );
        assert!(function_lattice(&db, wrong_body).complexity_guarded);
        let mut wrong_config = config;
        wrong_config.strict_quoting = !wrong_config.strict_quoting;
        let wrong_config_key = make_key(
            procedure.body.clone(),
            wrong_config,
            snapshot.clone(),
            Some(Arc::clone(event)),
        );
        assert!(function_lattice(&db, wrong_config_key).complexity_guarded);
        let wrong_registry = make_key(
            procedure.body.clone(),
            config,
            CompilerMemoSnapshot {
                profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
                registry: db.registry("tcl8.6").snapshot(),
            },
            Some(Arc::clone(event)),
        );
        assert!(function_lattice(&db, wrong_registry).complexity_guarded);
    }

    fn supplied_cfg_lattice_key<'db>(
        db: &'db dyn TclDb,
        module: &tcl_compiler::ir::Module,
        input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
    ) -> FnLatticeKey<'db> {
        let registry = module.resolved_registry();
        let (upvars, params, globals, bindings) =
            tcl_compiler::cfg_builder::prepare_cfg_context_with_registry(module, registry);
        assert!(upvars.is_empty() && params.is_empty() && globals.is_empty());
        let context = CfgContext::new(db, Vec::new(), Vec::new(), Vec::new(), bindings);
        FnLatticeKey::new(
            db,
            module.top_level.clone(),
            "::subject".to_owned(),
            Vec::new(),
            context,
            module.lexer_config.normalized(),
            module.dialect.clone().unwrap(),
            CompilerMemoSnapshot {
                profile: module
                    .dialect_profile
                    .map(tcl_dialect::DialectProfile::cache_key),
                registry: registry.snapshot(),
            },
            Vec::new(),
            FnLatticeModuleFacts {
                known_classes: Vec::new(),
                traced_variables: Vec::new(),
                has_dynamic_variable_trace: false,
            },
            FnLatticeEntry {
                plain_command_dispatch: module.plain_command_dispatch,
                source_metadata_input: input,
                irules_event_body: None,
                command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                    .snapshot(),
            },
            ValueTransferContext::of(
                db,
                tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                    &tcl_compiler::command_binding::ModuleCommandMutations::default(),
                    registry,
                ),
                0,
            ),
        )
    }

    #[test]
    fn function_lattice_retains_supplied_availability_and_refuses_missing_or_foreign_input() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Conditional source CFG completion only; no Native execution or handler grant.
        let db = TclDatabase::default();
        let generation = tcl_registry::model::ingress::context_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let mut lowerer =
            tcl_compiler::lowering::Lowerer::with_config(generation.commands(), config)
                .with_dialect(generation.commands().profile())
                .with_context_registry(Arc::clone(&generation));
        let module = lowerer.lower("throw ERROR payload; set reached 1").clone();
        let input = module.source_metadata_input.as_ref().unwrap();
        assert!(Arc::ptr_eq(&input.context_registry(), &generation));
        let selected = supplied_cfg_lattice_key(&db, &module, Some(input.clone()));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(generation.commands())),
        );
        assert!(Arc::ptr_eq(older.commands(), generation.commands()));
        let unavailable = supplied_cfg_lattice_key(
            &db,
            &module,
            Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older,
                input.lexer_config(),
            )),
        );
        let foreign = supplied_cfg_lattice_key(
            &db,
            &module,
            Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                tcl_registry::model::ingress::context_for_profile(
                    tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
                ),
                input.lexer_config(),
            )),
        );
        let missing = supplied_cfg_lattice_key(&db, &module, None);
        let expected = Some(tcl_compiler::cfg::Terminator::Complete {
            route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::CompletionCode::Error,
            ),
            span: Some(module.top_level.statements[0].span()),
        });
        let current = function_lattice(&db, selected);
        assert_eq!(current.cfg.blocks[&current.cfg.entry].terminator, expected);
        assert_eq!(current.cfg.blocks[&current.cfg.entry].statements.len(), 1);
        for negative in [unavailable, foreign, missing] {
            assert!(
                selected != negative,
                "complete source input owns memo identity"
            );
            let conservative = function_lattice(&db, negative);
            let entry = &conservative.cfg.blocks[&conservative.cfg.entry];
            assert_ne!(entry.terminator, expected);
            assert_eq!(
                entry.statements.len(),
                2,
                "withheld completion keeps the following call"
            );
        }
        assert!(Arc::ptr_eq(&current, &function_lattice(&db, selected)));
    }

    fn semantic_bundle_has_selected_invocation(
        unit: &tcl_compiler::compilation_unit::FunctionUnit,
    ) -> bool {
        unit.semantic_facts.executable().invocations().any(|call| {
            matches!(
                call.resolution,
                tcl_compiler::executable_ir::InvocationResolution::Resolved(_)
            )
        })
    }

    #[test]
    fn function_lattice_semantic_bundle_uses_retained_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let db = TclDatabase::default();
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut puts = registry.get("puts").unwrap().clone();
        puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(puts);
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
        let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(current.commands(), config)
            .with_dialect(current.commands().profile())
            .with_context_registry(Arc::clone(&current));
        let module = lowerer.lower("puts VALUE").clone();
        let input = module.source_metadata_input.as_ref().unwrap();
        let selected = supplied_cfg_lattice_key(&db, &module, Some(input.clone()));
        assert!(semantic_bundle_has_selected_invocation(&function_lattice(
            &db, selected
        )));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        let unavailable = supplied_cfg_lattice_key(
            &db,
            &module,
            Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older,
                input.lexer_config(),
            )),
        );
        assert!(selected != unavailable);
        assert!(!semantic_bundle_has_selected_invocation(&function_lattice(
            &db,
            unavailable
        )));
        let foreign = supplied_cfg_lattice_key(
            &db,
            &module,
            Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                tcl_registry::model::ingress::resolve_environment("tcl9.1")
                    .default_context_registry(),
                input.lexer_config(),
            )),
        );
        let missing = supplied_cfg_lattice_key(&db, &module, None);
        for negative in [foreign, missing] {
            assert!(selected != negative);
            assert!(matches!(
                function_lattice(&db, negative).semantic_facts.executable(),
                tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
            ));
        }
    }

    #[test]
    fn function_lattice_key_keeps_the_exact_normalized_lexer_config() {
        let db = TclDatabase::default();
        let context = CfgContext::new(
            &db,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ModuleCommandBindings::default(),
        );
        let make_key = |config: tcl_lexer::LexerConfig| {
            FnLatticeKey::new(
                &db,
                Script::default(),
                "::subject".to_owned(),
                Vec::new(),
                context,
                config.normalized(),
                "tcl8.6".to_owned(),
                CompilerMemoSnapshot {
                    profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
                    registry: db.registry("tcl8.6").snapshot(),
                },
                Vec::new(),
                FnLatticeModuleFacts {
                    known_classes: Vec::new(),
                    traced_variables: Vec::new(),
                    has_dynamic_variable_trace: false,
                },
                FnLatticeEntry {
                    plain_command_dispatch: false,
                    source_metadata_input: None,
                    irules_event_body: None,
                    command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                        .snapshot(),
                },
                ValueTransferContext::of(
                    &db,
                    tcl_compiler::value_transfer::AnalysisContextKey::detached(),
                    0,
                ),
            )
        };

        let tcl_key = make_key(tcl_lexer::LexerConfig::for_dialect("tcl8.6"));
        let irules_key = make_key(tcl_lexer::LexerConfig::for_dialect("f5-irules"));
        let jim_config = tcl_lexer::LexerConfig::for_dialect("jim");
        let jim_key = make_key(jim_config);
        assert!(
            tcl_key != irules_key,
            "the iRules ghost-separator axis is part of memo identity"
        );
        assert!(
            tcl_key != jim_key,
            "Jim's vertical-tab word-separator axis is part of memo identity"
        );

        let shifted_jim_key = make_key(tcl_lexer::LexerConfig {
            base_offset: 91,
            base_line: 7,
            base_col: 13,
            ..jim_config
        });
        assert!(
            jim_key == shifted_jim_key,
            "position-only relocation must normalize out of memo identity"
        );

        let _cold_other_config = function_lattice(&db, tcl_key);
        let jim_cold = function_lattice(&db, jim_key);
        let jim_warm = function_lattice(&db, jim_key);
        assert!(
            Arc::ptr_eq(&jim_cold, &jim_warm),
            "the warm exact-config demand must reuse the cold lattice"
        );
    }

    /// An unchanged proc body must re-intern to the *same*
    /// `ItemBodyKey` (so the memo hits) and must not cost a fresh copy of the
    /// body text to get there — the key shares the analyser's `Arc<str>`.
    #[test]
    fn unchanged_body_reinterns_to_the_same_key_without_copying_its_text() {
        let db = TclDatabase::default();
        let body: Arc<str> = Arc::from("set s 1\nreturn $s\n");
        let make = || {
            ItemBodyKey::new(
                &db,
                Arc::clone(&body),
                "::".to_owned(),
                "alpha".to_owned(),
                Vec::new(),
                false,
                false,
                Vec::new(),
                None,
                ((Vec::new(), Vec::new()), None, None, None),
                "tcl8.6".to_owned(),
                Vec::new(),
                NonAsciiMode::Default,
            )
        };
        assert!(make() == make(), "unchanged text must re-intern to one key");
        assert!(
            Arc::ptr_eq(make().body_text(&db), &body),
            "the interned key must share the caller's body text, not copy it",
        );
        // A different body must not collide with it.
        let other: Arc<str> = Arc::from("set s 2\nreturn $s\n");
        let other_key = ItemBodyKey::new(
            &db,
            other,
            "::".to_owned(),
            "alpha".to_owned(),
            Vec::new(),
            false,
            false,
            Vec::new(),
            None,
            ((Vec::new(), Vec::new()), None, None, None),
            "tcl8.6".to_owned(),
            Vec::new(),
            NonAsciiMode::Default,
        );
        assert!(
            make() != other_key,
            "distinct bodies must intern distinctly"
        );
    }

    #[test]
    fn document_symbols_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = document_symbols(&db, file, cfg(&db));
        let expected = tcl_lsp_core::document_symbols::document_symbols(
            SRC,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        );
        assert_eq!(got, expected);
    }

    #[test]
    fn semantic_tokens_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = semantic_tokens(&db, file, cfg(&db));
        let reg = db.registry("tcl");
        let expected = tcl_lsp_core::semantic_tokens::full(
            SRC,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            reg,
        );
        assert_eq!(got, expected);
        assert!(!got.data.is_empty());
    }

    /// TP: the enriched `semantic_tokens` query (backed by a
    /// [`CompilationUnit`] and SSA/SCCP-derived constant-string facts) retags
    /// the `.*abc` literal at its originating `set` as regex source, because
    /// `regexp $my_re` provably reads that constant — see
    /// [`tcl_compiler::regex_source`]. The cheap coarse tier
    /// (`tcl_lsp_core::semantic_tokens::full`, no `CompilationUnit`) has no
    /// SSA facts to do this with, so the two streams differ — proving the
    /// enrichment `semantic_tokens` performs over the coarse walk is real,
    /// not a no-op: the fast-path fallback in
    /// `Backend::semantic_tokens_core_data` trades this enrichment away,
    /// so it must exist for the trade to mean anything.
    #[test]
    fn semantic_tokens_retags_constant_regex_source_true_positive() {
        let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let enriched = semantic_tokens(&db, file, cfg(&db));
        let reg = db.registry("tcl9.0");
        let coarse = tcl_lsp_core::semantic_tokens::full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            reg,
        );
        assert_ne!(
            enriched, coarse,
            "the CompilationUnit-informed regex-source retag must change the \
             token stream relative to the coarse (no-CU) tier"
        );
    }

    /// TN: a `regexp` pattern read from a variable that is *not* provably
    /// constant (reassigned from a proc parameter) gets no retag from either
    /// tier — the coarse and enriched streams agree, because there is no
    /// constant fact for the enriched tier to add. Guards against the retag
    /// firing indiscriminately on every `regexp $var` call.
    #[test]
    fn semantic_tokens_skips_retag_for_non_constant_pattern_true_negative() {
        let src = "proc match {re s} {\n    regexp $re $s\n}\n";
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let enriched = semantic_tokens(&db, file, cfg(&db));
        let reg = db.registry("tcl9.0");
        let coarse = tcl_lsp_core::semantic_tokens::full(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            reg,
        );
        assert_eq!(
            enriched, coarse,
            "a non-constant pattern source must not be retagged by either tier"
        );
    }

    /// `semantic_tokens` must depend on
    /// the incremental, per-item-memoised `file_analysis_incremental` — not
    /// the coarse `file_analysis` — so a token request that lands after the
    /// diagnostics worker has already analysed this revision is a cache hit
    /// (no second whole-file walk), and a token request that lands *first*
    /// still primes the exact query diagnostics will reuse. Also asserts the
    /// coarse, uncancellable `file_analysis` is never invoked by either order.
    #[test]
    fn semantic_tokens_shares_incremental_analysis_with_diagnostics() {
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
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);

        // Diagnostics-first order: the worker analyses, then a token request
        // arrives for the same revision.
        let _ = file_analysis_incremental(&db, file, cfg);
        let after_diagnostics = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            after_diagnostics
                .iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            1,
            "diagnostics analyses the one proc body: {after_diagnostics:?}"
        );

        let _ = semantic_tokens(&db, file, cfg);
        let after_tokens = std::mem::take(&mut *log.lock().unwrap());
        assert!(
            after_tokens
                .iter()
                .all(|s| !s.contains("item_body_analysis")),
            "a token request for the same revision must be a cache hit against \
             the diagnostics worker's analysis, not a second walk: {after_tokens:?}"
        );
        assert!(
            after_tokens.iter().all(|s| !s.contains("file_analysis(")),
            "semantic_tokens must never invoke the coarse, uncancellable \
             file_analysis query: {after_tokens:?}"
        );

        // Token-first order (a cold open before the debounced diagnostics
        // worker has run): editing then re-requesting tokens still only
        // recomputes the one changed body, proving the per-item firewall
        // covers the token path too, not just the diagnostics path.
        file.set_text(&mut db)
            .to("proc greet {name} {\n    puts \"hi $name!!\"\n}\n# c\nset x 1\n".to_owned());
        let _ = std::mem::take(&mut *log.lock().unwrap());
        let _ = semantic_tokens(&db, file, cfg);
        let after_edit = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            after_edit
                .iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            1,
            "a single-body edit must recompute exactly one item via the token \
             path: {after_edit:?}"
        );
    }

    /// End-to-end through the query graph: `lib.tcl` has no
    /// `package provide`, its two in-file callers agree on `"prod"`, and
    /// `main.tcl` — which the single-file compilation unit can never see —
    /// calls `helper dev`.  With the project's evidence set on the file, the
    /// I230 "condition is always true/false" fold must not happen.
    mod cross_file_call_sites {
        use super::*;

        const LIB: &str = "proc helper {mode} {\n\
                           if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                           }\n\
                           helper prod\n\
                           helper prod\n";

        /// Build a two-file project and return `lib`'s compilation unit with
        /// the project evidence applied, exactly as the server's
        /// `sync_cross_file_evidence` does.
        fn lib_unit_with_project(main_src: &str) -> Arc<CompilationUnit> {
            use salsa::Setter as _;
            let mut db = TclDatabase::default();
            let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
            let main = SourceFile::new(&db, main_src.to_owned(), "tcl8.6".to_owned(), None);
            let project = Project::new(&db, vec![lib, main]);
            let evidence = file_external_call_sites(&db, lib, project);
            lib.set_external_call_sites(&mut db).to(Some(evidence));
            document_compilation_unit(&db, lib)
        }

        /// Build a project from `(path, source)` pairs and return the
        /// compilation unit of the first, with project evidence applied.
        fn unit_with_paths(files: &[(&str, &str)]) -> Arc<CompilationUnit> {
            use salsa::Setter as _;
            let mut db = TclDatabase::default();
            let handles: Vec<SourceFile> = files
                .iter()
                .map(|(path, src)| {
                    SourceFile::new(
                        &db,
                        (*src).to_owned(),
                        "tcl8.6".to_owned(),
                        Some((*path).to_owned()),
                    )
                })
                .collect();
            let project = Project::new(&db, handles.clone());
            let target = handles[0];
            let evidence = file_external_call_sites(&db, target, project);
            target.set_external_call_sites(&mut db).to(Some(evidence));
            document_compilation_unit(&db, target)
        }

        /// A sourced library's *unreadable* dispatch reaches the procedures of
        /// the file that sources it: `lib.tcl`'s script runs in `main.tcl`'s
        /// interpreter, so `$cmd` can name `::helper`.
        ///
        /// The inbound direction. Bounding an unenumerable dispatch by what
        /// the scanning file itself loads gets this wrong — `lib.tcl` declares
        /// no linkage at all — which is why the bound is the `source`-
        /// connected component (`file_dispatch_reach`), not the file's own
        /// linkage traits.
        #[test]
        fn an_unreadable_dispatch_in_a_sourced_library_clears_the_sourcing_files_fold() {
            let cu = unit_with_paths(&[
                (
                    "/w/main.tcl",
                    "proc helper {mode} {\n\
                     if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                     }\n\
                     source lib.tcl\n\
                     helper prod\n\
                     helper prod\n",
                ),
                ("/w/lib.tcl", "set cmd [gets stdin]\n$cmd dev\n"),
            ]);
            assert!(
                !folds_mode(&cu),
                "lib.tcl runs in main.tcl's interpreter; $cmd may name ::helper",
            );
        }

        /// TN control: the same unreadable dispatch in a file nothing sources
        /// and which sources nothing shares no interpreter, so it must leave
        /// an unrelated file's sound seed alone. Without the component bound
        /// this is the case that breaks — one `eval $script` anywhere in a
        /// workspace would withdraw every seed in every file.
        #[test]
        fn an_unreadable_dispatch_in_an_unlinked_file_leaves_the_fold_alone() {
            let cu = unit_with_paths(&[
                (
                    "/w/main.tcl",
                    "proc helper {mode} {\n\
                     if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n\
                     }\n\
                     helper prod\n\
                     helper prod\n",
                ),
                ("/w/unrelated.tcl", "set cmd [gets stdin]\n$cmd dev\n"),
            ]);
            assert!(
                folds_mode(&cu),
                "an unlinked file shares no interpreter with main.tcl",
            );
        }

        fn folds_mode(cu: &CompilationUnit) -> bool {
            !cu.procedures
                .get("::helper")
                .expect("helper analysed")
                .sccp
                .constant_branches
                .is_empty()
        }

        #[test]
        fn caller_in_another_file_with_a_differing_literal_retracts_the_fold() {
            let cu = lib_unit_with_project("source lib.tcl\nhelper dev\n");
            assert!(
                !folds_mode(&cu),
                "main.tcl calls helper with \"dev\"; the seed is unsound"
            );
        }

        #[test]
        fn caller_in_another_file_agreeing_still_folds() {
            let cu = lib_unit_with_project("source lib.tcl\nhelper prod\n");
            assert!(
                folds_mode(&cu),
                "every caller in the project passes \"prod\""
            );
        }

        /// The per-file slice is keyed on the file's own declarations *and*
        /// carries only calls from **other** files.
        ///
        /// `main` both declares and calls `::other`, so that call is in-unit
        /// evidence for `main` and never reaches its external slice — leaving
        /// it empty, which is still the meaningful "the project was
        /// enumerated, nobody else calls in" claim. `lib`'s slice does carry
        /// `::helper`, because `main` calls it without declaring it.
        #[test]
        fn slice_carries_only_other_files_calls_to_this_files_procs() {
            let db = TclDatabase::default();
            let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
            let main = SourceFile::new(
                &db,
                "proc other {x} { return $x }\nother 1\nhelper dev\n".to_owned(),
                "tcl8.6".to_owned(),
                None,
            );
            let project = Project::new(&db, vec![lib, main]);
            let lib_slice = file_external_call_sites(&db, lib, project);
            assert_eq!(lib_slice.callees().collect::<Vec<_>>(), vec!["::helper"]);
            let main_slice = file_external_call_sites(&db, main, project);
            assert!(
                main_slice.is_empty(),
                "main's call to its own `other` is in-unit evidence: {main_slice:?}"
            );
        }

        /// A workspace pools every file into one project, so two *unrelated*
        /// files reusing a common helper name (`helper`, `init`, `run` — endemic
        /// in Tcl) must not pool their call sites.  Here the second file declares
        /// its own zero-arity `::helper`; without the self-declared-callee
        /// exclusion its call contributes `arg_counts {0}`, `binds_position(0)`
        /// fails, and the first file loses a fold it is entitled to — and would
        /// regain only when the unrelated file changed.
        ///
        /// This exact collision surfaces in the VS Code suite, where ~200
        /// fixtures share one workspace folder.
        #[test]
        fn an_unrelated_file_reusing_a_proc_name_does_not_poison_the_seed() {
            use salsa::Setter as _;
            let mut db = TclDatabase::default();
            let ctl = SourceFile::new(
                &db,
                "proc helper {mode} {\n if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\n\
             proc c1 {} { helper prod }\nproc c2 {} { helper prod }\n"
                    .to_owned(),
                "tcl8.6".to_owned(),
                None,
            );
            let unrelated = SourceFile::new(
                &db,
                "proc helper {} {}\nproc caller {} { helper }\n".to_owned(),
                "tcl8.6".to_owned(),
                None,
            );
            let project = Project::new(&db, vec![ctl, unrelated]);
            let evidence = file_external_call_sites(&db, ctl, project);
            assert!(
                evidence.get("::helper").is_none(),
                "the unrelated file's own `helper` is in-unit evidence there, not \
             evidence about this file's proc: {evidence:?}"
            );
            ctl.set_external_call_sites(&mut db).to(Some(evidence));
            let cu = document_compilation_unit(&db, ctl);
            assert!(
                !cu.procedures
                    .get("::helper")
                    .expect("helper analysed")
                    .sccp
                    .constant_branches
                    .is_empty(),
                "every caller that can actually reach this proc passes \"prod\""
            );
        }

        /// A body edit that moves no call-site literal must leave
        /// `project_call_site_evidence` equal, so it backdates and no
        /// compilation unit downstream of it is invalidated.
        #[test]
        fn evidence_backdates_across_an_unrelated_body_edit() {
            use salsa::Setter as _;
            let mut db = TclDatabase::default();
            let lib = SourceFile::new(&db, LIB.to_owned(), "tcl8.6".to_owned(), None);
            let main = SourceFile::new(
                &db,
                "source lib.tcl\nhelper dev\n".to_owned(),
                "tcl8.6".to_owned(),
                None,
            );
            let project = Project::new(&db, vec![lib, main]);
            let before = file_external_call_sites(&db, lib, project);
            main.set_text(&mut db)
                .to("source lib.tcl\nset unrelated 1\nhelper dev\n".to_owned());
            let after = file_external_call_sites(&db, lib, project);
            assert_eq!(*before, *after, "an unrelated edit must not move evidence");
        }

        /// The component bound itself: both ends of a `source` edge see the
        /// same merged set — and see the *same* `Arc`, so a component is
        /// stored once however many files belong to it.  An unlinked file
        /// shares no interpreter, so it reaches only its own declarations.
        #[test]
        fn dispatch_reach_is_the_shared_source_component_merge() {
            let db = TclDatabase::default();
            let a = SourceFile::new(
                &db,
                "source b.tcl\nproc a {} {}\n".to_owned(),
                "tcl8.6".to_owned(),
                Some("/w/a.tcl".to_owned()),
            );
            let b = SourceFile::new(
                &db,
                "proc b {} {}\n".to_owned(),
                "tcl8.6".to_owned(),
                Some("/w/b.tcl".to_owned()),
            );
            let unlinked = SourceFile::new(
                &db,
                "proc c {} {}\n".to_owned(),
                "tcl8.6".to_owned(),
                Some("/w/c.tcl".to_owned()),
            );
            let project = Project::new(&db, vec![a, b, unlinked]);
            let reach_a = file_dispatch_reach(&db, a, project);
            let reach_b = file_dispatch_reach(&db, b, project);
            let reach_c = file_dispatch_reach(&db, unlinked, project);
            assert_eq!(
                reach_a.iter().map(String::as_str).collect::<Vec<_>>(),
                vec!["::a", "::b"],
                "the sourcing file reaches its own and the sourced file's procs"
            );
            assert!(
                Arc::ptr_eq(&reach_a, &reach_b),
                "component members share one merged set: {reach_b:?}"
            );
            assert_eq!(
                reach_c.iter().map(String::as_str).collect::<Vec<_>>(),
                vec!["::c"],
                "an unlinked file shares no interpreter"
            );
        }

        /// The `source`-graph decomposition is *project* work, not
        /// per-file work.  Demanding every file's reach must execute
        /// [`project_dispatch_components`] exactly **once** — the property that
        /// keeps the union-find-and-merge at `O(N)` overall rather than paying
        /// it once per file — and a decl change must recompute it once for the
        /// project rather than once per file.  A body edit backdates its
        /// inputs and recomputes nothing.
        #[test]
        fn dispatch_components_are_computed_once_per_project_revision() {
            use salsa::Setter as _;
            const FILES: usize = 6;
            let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let mut db = {
                let sink = Arc::clone(&log);
                TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key))
            };
            // A single `source` chain, so every file lands in one component —
            // the shape a per-file union-find-and-merge would re-pay for
            // every file.
            let body = |i: usize| {
                if i + 1 < FILES {
                    format!("source f{}.tcl\nproc p{i} {{}} {{ set r 1 }}\n", i + 1)
                } else {
                    format!("proc p{i} {{}} {{ set r 1 }}\n")
                }
            };
            let files: Vec<SourceFile> = (0..FILES)
                .map(|i| {
                    SourceFile::new(
                        &db,
                        body(i),
                        "tcl8.6".to_owned(),
                        Some(format!("/w/f{i}.tcl")),
                    )
                })
                .collect();
            let project = Project::new(&db, files.clone());
            let drain = || std::mem::take(&mut *log.lock().unwrap());
            let decompositions = |events: &[String]| {
                events
                    .iter()
                    .filter(|key| key.contains("project_dispatch_components"))
                    .count()
            };

            for &file in &files {
                let _ = file_dispatch_reach(&db, file, project);
            }
            assert_eq!(
                decompositions(&drain()),
                1,
                "cold: one decomposition serves every file's reach"
            );

            files[3]
                .set_text(&mut db)
                .to("source f4.tcl\nproc p3 {} { set r 999 }\n".to_owned());
            for &file in &files {
                let _ = file_dispatch_reach(&db, file, project);
            }
            assert_eq!(
                decompositions(&drain()),
                0,
                "a body edit leaves file_decls / file_link_targets equal"
            );

            files[3]
                .set_text(&mut db)
                .to("source f4.tcl\nproc p3 {} { set r 999 }\nproc extra {} {}\n".to_owned());
            for &file in &files {
                let _ = file_dispatch_reach(&db, file, project);
            }
            assert_eq!(
                decompositions(&drain()),
                1,
                "a decl change recomputes the decomposition once, not once per file"
            );
            assert!(
                file_dispatch_reach(&db, files[0], project).contains("::extra"),
                "the new proc joins every component member's reach"
            );
        }
    }

    /// Build a database whose executed-query keys land in the returned log.
    fn logging_db() -> (TclDatabase, Arc<Mutex<Vec<String>>>) {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let l = Arc::clone(&log);
            move |ev: salsa::Event| {
                if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                    l.lock().unwrap().push(format!("{database_key:?}"));
                }
            }
        };
        let db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        (db, log)
    }

    /// [`project_class_index`] / [`project_proc_var_index`] read
    /// *every* file in the project, so the tier they read decides what an
    /// interactive `semanticTokens` request costs on a large workspace. They
    /// must stay on the light [`file_token_facts`] tier and never reach the deep
    /// one — reading `file_analysis_incremental` there would mean a
    /// whole-workspace deep analysis (CFG/SSA units, per-body lattices,
    /// diagnostic emitters) behind a token request for one file, which on an
    /// 883-file checkout would never complete before the next `set_text`
    /// cancelled it.
    #[test]
    fn project_indexes_never_touch_the_deep_tier() {
        let (db, log) = logging_db();
        let lib = SourceFile::new(
            &db,
            "oo::configurable create ::Pin { property node }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let main = SourceFile::new(
            &db,
            "set p [::Pin new]\n$p configure -node n1\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![lib, main]);

        let _ = project_class_index(&db, project);
        let _ = project_proc_var_index(&db, project);
        let log_snapshot = log.lock().unwrap().clone();
        assert!(
            log_snapshot.iter().any(|s| s.contains("file_token_facts")),
            "project indexes must read the light per-file tier: {log_snapshot:?}"
        );
        assert!(
            log_snapshot.iter().all(|s| !s.contains("file_analysis")),
            "project indexes must never analyse a project file at the deep tier \
             (#1163): {log_snapshot:?}"
        );
    }

    /// The per-file [`file_token_facts`] firewall: a body edit that changes no
    /// class and no parameter role backdates, so neither project index
    /// re-executes — the property that keeps the cross-file token aggregates off
    /// the per-keystroke path once they are warm.
    #[test]
    fn project_indexes_backdate_on_a_role_neutral_body_edit() {
        use salsa::Setter as _;
        let (mut db, log) = logging_db();
        let lib = SourceFile::new(
            &db,
            "oo::configurable create ::Pin { property node }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let main = SourceFile::new(
            &db,
            "proc ::helper {a b} { return [expr {$a + $b}] }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![lib, main]);
        let _ = project_class_index(&db, project);
        let _ = project_proc_var_index(&db, project);

        log.lock().unwrap().clear();
        main.set_text(&mut db)
            .to("proc ::helper {a b} { return [expr {$a * $b}] }\n".to_owned());
        let _ = project_class_index(&db, project);
        let _ = project_proc_var_index(&db, project);
        let after = log.lock().unwrap().clone();
        assert!(
            after.iter().any(|s| s.contains("file_token_facts")),
            "the edited file's own facts must be recomputed: {after:?}"
        );
        assert!(
            after.iter().all(
                |s| !s.contains("project_class_index") && !s.contains("project_proc_var_index")
            ),
            "a role-neutral body edit must backdate at the per-file firewall, \
             leaving the project aggregates memoised: {after:?}"
        );
    }

    /// The merged project role index must equal one built from every file's
    /// procs at once — including the abstentions each file made on its own, so
    /// a name one file already dropped as ambiguous is not re-adopted from
    /// another file's unambiguous entry.
    #[test]
    fn merged_role_index_equals_the_all_at_once_build() {
        let sources = [
            // `::a::grow` and `::b::grow` disagree, so this file abstains on the
            // bare `grow` key all by itself.
            "namespace eval a { proc grow {v} { upvar 1 $v x; set x 1 } }\n\
             namespace eval b { proc grow {n v} { upvar 1 $v x; set x 1 } }\n",
            // A third `grow`, unambiguous within its own file.
            "namespace eval c { proc grow {v} { upvar 1 $v x; set x 2 } }\n",
        ];
        let db = TclDatabase::default();
        let files: Vec<SourceFile> = sources
            .iter()
            .map(|s| SourceFile::new(&db, (*s).to_owned(), "tcl8.6".to_owned(), None))
            .collect();
        let project = Project::new(&db, files.clone());

        let all_procs: Vec<Arc<AnalysisResult>> = sources
            .iter()
            .map(|s| Arc::new(Analyser::new().structure_only().analyse(s, "tcl8.6")))
            .collect();
        let expected =
            VarNameArgRoles::from_procs(all_procs.iter().flat_map(|a| a.all_procs.values()));
        assert_eq!(*project_proc_var_index(&db, project), expected);
        // Order-independent: reversing the project's files changes nothing.
        let reversed = Project::new(&db, files.into_iter().rev().collect());
        assert_eq!(*project_proc_var_index(&db, reversed), expected);
    }

    #[test]
    fn cross_file_object_dispatch_resolves_via_project_index() {
        // A class defined in one file, dispatched on via a direct constructor in
        // another: `semantic_tokens_project` resolves the method through the
        // workspace-merged `project_class_index`, so its output differs from the
        // local-only `semantic_tokens` (which leaves the method a plain string).
        let db = TclDatabase::default();
        let lib = SourceFile::new(
            &db,
            "oo::configurable create ::Pin { property node }\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let user = SourceFile::new(
            &db,
            "[::Pin new] configure -node 5\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![lib, user]);
        let cross = semantic_tokens_project(&db, user, project, cfg(&db));
        let local = semantic_tokens(&db, user, cfg(&db));
        assert_ne!(
            cross.data, local.data,
            "cross-file index must resolve the method the local pass leaves unresolved"
        );
        // The merged index sees the class from the other file.
        assert!(
            project_class_index(&db, project)
                .classes
                .contains_key("::Pin"),
            "project index should contain ::Pin from the library file"
        );
    }

    #[test]
    fn project_index_abstains_on_duplicate_class_name() {
        // The same qualified class name defined in two files is genuinely
        // ambiguous — which `::Pin` does a cross-file dispatch mean?  The merged
        // index drops it (sound-by-abstention), deterministically regardless of
        // file order, rather than letting an order-dependent "winner" leak into
        // resolution.
        let db = TclDatabase::default();
        let a = SourceFile::new(
            &db,
            "oo::class create ::Pin { method a {} {} }\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let b = SourceFile::new(
            &db,
            "oo::class create ::Pin { method b {} {} }\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        // Both orderings must agree (and both must drop the ambiguous class).
        for files in [vec![a, b], vec![b, a]] {
            let project = Project::new(&db, files);
            assert!(
                !project_class_index(&db, project)
                    .classes
                    .contains_key("::Pin"),
                "an ambiguous cross-file class name must be dropped, not resolved"
            );
        }
    }

    #[test]
    fn folding_matches_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = folding_ranges(&db, file, cfg(&db));
        let reg = db.registry("tcl");
        let expected = tcl_lsp_core::folding::folding_ranges(
            SRC,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            reg,
        );
        assert_eq!(got, expected);
    }

    /// Folding served through salsa is memoised: a repeat request for the
    /// same revision must not re-execute the tracked query — the property
    /// `Backend::db_folding_ranges` relies on to avoid a fresh
    /// segmenter/registry walk on every `textDocument/foldingRange`.
    #[test]
    fn folding_ranges_memoised_across_repeat_queries() {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let db = {
            let sink = Arc::clone(&log);
            TclDatabase::with_event_logger(move |key| sink.lock().unwrap().push(key))
        };
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let drain = || std::mem::take(&mut *log.lock().unwrap());
        let executions = |events: &[String]| {
            events
                .iter()
                .filter(|key| key.contains("folding_ranges"))
                .count()
        };

        let _ = folding_ranges(&db, file, cfg(&db));
        assert_eq!(
            executions(&drain()),
            1,
            "cold: one execution for the first request"
        );
        let _ = folding_ranges(&db, file, cfg(&db));
        assert_eq!(
            executions(&drain()),
            0,
            "warm: memoised, no re-execution for a repeat request at the same revision"
        );
    }

    #[test]
    fn editing_text_recomputes() {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let config = cfg(&db);
        let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned(), None);
        assert!(
            file_analysis(&db, file, config)
                .all_procs
                .contains_key("::a")
        );

        file.set_text(&mut db).to("proc b {} {}\n".to_owned());
        let after = file_analysis(&db, file, config);
        assert!(after.all_procs.contains_key("::b"));
        assert!(!after.all_procs.contains_key("::a"));
    }

    #[test]
    fn file_decls_match_file_analysis() {
        use std::collections::BTreeSet;
        let db = TclDatabase::default();
        let src = "proc p {} {}\noo::class create K {}\nnamespace eval z { proc q {} {} }\n";
        let file = SourceFile::new(&db, src.to_owned(), "tcl".to_owned(), None);
        let decls = file_decls(&db, file);
        let analysis = file_analysis(&db, file, cfg(&db));
        let want_procs: BTreeSet<String> = analysis.all_procs.keys().cloned().collect();
        let want_classes: BTreeSet<String> = analysis.all_classes.keys().cloned().collect();
        let want_aliases: BTreeSet<String> = analysis.command_aliases.keys().cloned().collect();
        assert_eq!(decls.procs, want_procs);
        assert_eq!(decls.classes, want_classes);
        assert_eq!(decls.aliases, want_aliases);
        assert!(decls.namespaces.contains("::z"));
    }

    #[test]
    fn item_sigs_track_signatures() {
        use tcl_compiler::analyser::ItemKind;
        let db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            "proc greet {name} {}\n".to_owned(),
            "tcl".to_owned(),
            None,
        );
        let sigs = item_sigs(&db, file);
        let greet = sigs
            .iter()
            .find(|s| s.id.kind == ItemKind::Proc && s.id.key == "::greet")
            .expect("greet item");
        assert_eq!(greet.params.len(), 1);
        assert_eq!(greet.params[0].name, "name");
        assert_eq!(greet.namespace, "::");
    }

    #[test]
    fn item_tree_recomputes_on_edit() {
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned(), None);
        assert!(file_decls(&db, file).procs.contains("::a"));
        file.set_text(&mut db).to("proc b {} {}\n".to_owned());
        let after = file_decls(&db, file);
        assert!(after.procs.contains("::b"));
        assert!(!after.procs.contains("::a"));
    }

    #[test]
    fn file_analysis_incremental_matches_full() {
        let db = TclDatabase::default();
        let cfg = cfg(&db);
        for src in [
            SRC,
            "proc a {x} { return $x }\nproc b {} { a 1 }\n",
            "namespace eval n { proc f {y} { set z $y } }\nset g 1\nputs $g\n",
            "oo::class create K {\n  method m {a} { set n $a }\n}\nproc p {} { set q 1 }\n",
        ] {
            let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
            let inc = file_analysis_incremental(&db, file, cfg);
            let full = file_analysis(&db, file, cfg);
            assert_eq!(*inc, *full, "incremental != full for:\n{src}");
        }
    }

    #[test]
    fn sidecar_stubs_reach_incremental_proc_bodies_and_invalidate() {
        use salsa::Setter as _;

        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-sidecar-incremental-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let src_dir = root.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let sidecar = root.join("tcl8.6.tcl.stubs");
        std::fs::write(&sidecar, "stub sidecar_cmd {value}\n").unwrap();
        let path = src_dir.join("main.tcl");
        let mut db = TclDatabase::default();
        let config = cfg(&db);
        let file = SourceFile::new(
            &db,
            "proc f {} { sidecar_cmd 1 }\n".to_owned(),
            "tcl8.6".to_owned(),
            Some(path.display().to_string()),
        );

        let initial = file_analysis_incremental(&db, file, config);
        assert!(
            !initial.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "sidecar command must resolve inside a proc body: {:?}",
            initial.diagnostics
        );
        assert_eq!(*initial, *file_analysis(&db, file, config));

        std::fs::write(&sidecar, "stub replacement_cmd {value}\n").unwrap();
        file.set_sidecar_stubs_epoch(&mut db).to(1);
        let changed = file_analysis_incremental(&db, file, config);
        assert!(
            changed.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "the unchanged source must be reanalysed after its sidecar changes"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn file_analysis_incremental_carries_bigip_version_override() {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            Some("21.1.0".to_owned()),
            0,
            Vec::new(),
            Vec::new(),
        );
        let file = SourceFile::new(
            &db,
            "SSL::c3d cert_lifespan 24\n".to_owned(),
            "f5-irules".to_owned(),
            None,
        );

        let incremental = file_analysis_incremental(&db, file, cfg);
        let full = file_analysis(&db, file, cfg);

        assert_eq!(*incremental, *full);
        assert_eq!(
            incremental.library_versions.bigip_version.as_deref(),
            Some("21.1.0")
        );
    }

    /// `tclLsp.targets` (§5.4 range targeting) flows through the config
    /// into both analysis queries: a declared `tcl 8.5-9.0` range flags
    /// the 8.6-introduced `lmap` at the declared 8.5 target, and the
    /// undeclared default stays byte-identically silent.
    #[test]
    fn file_analysis_carries_declared_targets() {
        let db = TclDatabase::default();
        let ranged = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            vec![("tcl".to_owned(), "8.5-9.0".to_owned())],
            Vec::new(),
        );
        let bare = AnalyserConfig::new(
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
        let file = SourceFile::new(
            &db,
            "lmap x {1 2} {set x}\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );

        let incremental = file_analysis_incremental(&db, file, ranged);
        let full = file_analysis(&db, file, ranged);
        assert_eq!(*incremental, *full);
        assert!(
            incremental
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == "W150" && d.message.contains("8.5")),
            "the declared range flags lmap at the 8.5 target: {:?}",
            incremental.diagnostics
        );
        let undeclared = file_analysis_incremental(&db, file, bare);
        assert!(
            !undeclared
                .diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "W150" | "W151")),
            "no declaration, no range diagnostics"
        );
    }

    /// A body-only edit (same length, so other bodies keep their offset)
    /// recomputes exactly one `item_body_analysis`.
    #[test]
    fn body_edit_recomputes_one_item() {
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
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        let init = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            init.iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            2,
            "initial: both bodies analysed: {init:?}"
        );

        // Edit proc a's body, *changing its length* — this shifts proc b's
        // byte offset.  Offset-invariance means b's key (its body text) is
        // unchanged, so it stays a cache hit and only a recomputes.
        file.set_text(&mut db)
            .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
        let _ = file_analysis_incremental(&db, file, cfg);
        let after = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            after
                .iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            1,
            "length-changing body edit -> exactly ONE item recomputes (offset-invariant): {after:?}"
        );
    }

    /// The method firewall: a body edit to one OO method recomputes exactly one
    /// `item_body_analysis` — methods are isolated + memoised like procs, so an
    /// unedited sibling method (shifted by the edit) stays a cache hit.
    #[test]
    fn method_body_edit_recomputes_one_item() {
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
        let file = SourceFile::new(
            &db,
            "oo::class create K {\n  method a {} { set x 11111 }\n  method b {} { set y 22222 }\n}\n"
                .to_owned(),
            "tcl8.6".to_owned(),
         None,
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        let init = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            init.iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            2,
            "initial: both method bodies analysed: {init:?}"
        );

        // Edit method a's body length — shifts method b; b's offset-0 body is
        // unchanged, so its key is a cache hit and only a recomputes.
        file.set_text(&mut db).to(
            "oo::class create K {\n  method a {} { set x 9999999999 }\n  method b {} { set y 22222 }\n}\n"
                .to_owned(),
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        let after = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            after
                .iter()
                .filter(|s| s.contains("item_body_analysis"))
                .count(),
            1,
            "method body edit -> exactly ONE item recomputes: {after:?}"
        );
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

    /// The interprocedural taint cascade (`taint_cascade`, backlog #1) must stay
    /// byte-identical to the non-memoised `with_interprocedural` re-run **across
    /// edits** — the cold corpus differential only proves the reconstruction is
    /// complete, not that a stale cache can't survive an edit.  Drive a sequence
    /// of edits (including one that flips a callee's passthrough/global-write
    /// behaviour, which must invalidate its callers' cascades) and assert the
    /// memoised diagnostics equal a fresh uncached build at every step.
    #[test]
    fn taint_cascade_matches_uncached_under_edits() {
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
                pass_variants[s[0]],
                leaf_variants[s[1]],
                caller_variants[s[2]],
                calc_variants[s[3]],
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

    /// The per-procedure optimiser memo (`function_optimisations`) must
    /// skip an unrelated procedure across edits: a body edit that does not change a
    /// proc's *opt-projection* (`OptDepsKey`) leaves every other proc's optimise a
    /// cache hit, while the edited proc's own re-keys (its `FnLatticeKey` changed).
    /// This is the per-edit incrementality win — the whole-module `optimise_unit`
    /// re-optimised all procs on any edit.
    #[test]
    fn function_optimisations_reused_on_unrelated_edit() {
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
        let runs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("function_optimisations"))
                .count()
        };
        let mut db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        // Three independent procs (no foldable cross-proc calls, no pure-non-const
        // proc → the memo path, not the whole-module fallback).
        let file = SourceFile::new(
            &db,
            "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let _ = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            runs(&log),
            3,
            "cold build: every procedure's optimise runs once"
        );

        // Edit only `b`'s body (a literal change that does not alter b's
        // opt-projection) — `a`/`c` are cache hits; only `b`'s optimise re-runs.
        file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned());
        let _ = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            runs(&log),
            1,
            "unrelated body edit -> exactly ONE function_optimisations recomputes"
        );
    }

    /// The opt projection a memoised procedure reads its callees through keeps
    /// what the dead-store passes ask of them: the completion, and each
    /// parameter's default, which the call-by-name reads take for an omitted
    /// `Name` argument.
    #[test]
    fn the_opt_projection_keeps_completion_and_defaults() {
        let mut summary = ProcSummary::unknown("::bumpd");
        summary.params = vec!["name".to_owned()];
        summary
            .param_defaults
            .insert("name".to_owned(), "n".to_owned());
        summary.completes = true;
        let projected = opt_callee_to_summary(&opt_callee_from_summary(&summary));
        assert_eq!(projected.param_defaults, summary.param_defaults);
        assert!(projected.completes);
    }

    /// A callee that completes whatever its arguments hold makes the unused
    /// store of its call dead in the per-procedure optimiser memo as in the
    /// whole-module build: the memo reads a callee's summary through its
    /// opt projection, which carries the completion with the purity.
    #[test]
    fn a_completing_callee_reaches_the_optimiser_memo() {
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let l = Arc::clone(&log);
            move |ev: salsa::Event| {
                if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                    l.lock().unwrap().push(format!("{database_key:?}"));
                }
            }
        };
        let db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        // `k` is pure with an argument-independent constant return and `f`
        // prints, so no procedure is an argument-sensitive fold target and the
        // module keeps the memo path rather than the whole-module fallback.
        let text = "proc k {} { return 1 }\nproc f {} {\n    set u [k]\n    puts done\n}\n";
        let file = SourceFile::new(&db, text.to_owned(), "tcl8.6".to_owned(), None);
        let memoised = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            log.lock()
                .unwrap()
                .iter()
                .filter(|key| key.contains("function_optimisations"))
                .count(),
            2,
            "each procedure's optimisations come from the memo"
        );
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let whole = compiler_check_diagnostics_uncached(text, registry, "tcl8.6", None, None);
        let codes = |optimisations: &[Optimisation]| -> Vec<(String, u32, u32)> {
            optimisations
                .iter()
                .map(|o| (o.code.as_str().to_owned(), o.span.start(), o.span.end()))
                .collect()
        };
        assert_eq!(codes(&memoised.optimisations), codes(&whole.optimisations));
        assert!(
            memoised
                .optimisations
                .iter()
                .any(|o| o.code.as_str() == "O126"),
            "the unused store of a completing call goes; got {:?}",
            memoised.optimisations
        );
    }

    /// The per-procedure body-lowering memo
    /// (`lower_proc_body`) must skip an unchanged proc's body lowering across a
    /// body-only edit. For a context-free file (no `namespace`/`oo::`/nested
    /// `proc`), `build_unit_with_keys` lowers each top-level proc's static body
    /// through `lower_proc_body`, keyed on the offset-0 body text — so editing one
    /// proc's body re-lowers only that body; the others are cache hits. A shift
    /// (prepended blank line) re-lowers nothing (offset-invariant key).
    #[test]
    fn lower_proc_body_reused_on_unrelated_edit() {
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
        let runs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("lower_proc_body"))
                .count()
        };
        let mut db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        let file = SourceFile::new(
            &db,
            "proc a {} { puts 11111 }\n\
             proc b {} { puts 22222 }\n\
             proc c {} { puts 33333 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let _ = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            runs(&log),
            3,
            "cold build: every top-level proc body lowers once through the memo"
        );

        // Edit only `b`'s body — `a`/`c` bodies are cache hits.
        file.set_text(&mut db).to("proc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned());
        let _ = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            runs(&log),
            1,
            "unrelated body edit -> exactly ONE lower_proc_body recomputes"
        );

        // Prepend a blank line: every body shifts but none changes — the offset-0
        // body key is identical, so no body re-lowers.
        file.set_text(&mut db).to("\nproc a {} { puts 11111 }\n\
             proc b {} { puts 99999999 }\n\
             proc c {} { puts 33333 }\n"
            .to_owned());
        let _ = compiler_check_diagnostics(&db, file, cfg(&db));
        assert_eq!(
            runs(&log),
            0,
            "pure offset shift -> no body re-lowers (offset-invariant key)"
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
        // `proc_summary_cascade` re-executes (its `FnLatticeKey` changed).
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
        caller: String,
    ) -> SummaryDependencyProbe {
        let dialect = file.dialect(db);
        let registry = db.registry(dialect);
        let declared = declared_command_surface(db, file);
        let (cu, keys) = build_unit_with_keys(
            db,
            file.text(db),
            unit_build_options(db, file, cfg, registry, None, &declared),
        );
        let known: HashSet<_> = cu.ir_module.procedures.keys().cloned().collect();
        let mut summaries: HashMap<_, _> = known
            .iter()
            .map(|name| (name.clone(), ReturnTaintSummary::untainted(name, &[])))
            .collect();
        let provider = tcl_compiler::taint_interproc::infer_proc_summary(
            "::producer",
            &[],
            cu.function("::producer")
                .expect("actual provider source unit"),
            registry,
            cu.interproc.as_ref(),
            tcl_lsp_core::stated_profile_for_dialect(dialect),
            &known,
            &summaries,
        );
        summaries.insert("::producer".to_owned(), provider.clone());
        let body = cu.ir_module.procedures[&caller]
            .body_source
            .as_deref()
            .expect("actual caller source body");
        let dependencies = summary_deps_key(
            db,
            &caller,
            cu.function(&caller).expect("actual caller source unit"),
            Some(body),
            cu.interproc.as_ref(),
            &summaries,
            &known,
            dialect,
        );
        let summary = proc_summary_cascade(db, keys[&caller], dependencies);
        SummaryDependencyProbe {
            caller_name: caller,
            caller_body: body.to_owned(),
            provider,
            dependencies: dependencies.callee_summaries(db).clone(),
            caller: (*summary).clone(),
        }
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
        let source = |provider: &str| {
            format!(
                "proc producer {{}} {{{provider}}}\n\
             proc quoted {{}} {{return [\"producer\"]}}\n\
             proc braced {{}} {{return [{{producer}}]}}\n"
            )
        };
        let initial = source("return CLEAN");
        let changed = source("return [gets stdin]");
        let file = SourceFile::new(&db, initial, "tcl".to_owned(), None);
        let config = lexer_cfg_key(&db, "tcl");
        let before: Vec<_> = ["::quoted", "::braced"]
            .into_iter()
            .map(|caller| summary_dependency_probe(&db, file, config, caller.to_owned()))
            .collect();
        let runs = || {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|query| query.contains("proc_summary_cascade"))
                .count()
        };
        assert_eq!(runs(), 2, "two actual caller summary queries");
        for caller in ["::quoted", "::braced"] {
            let _ = summary_dependency_probe(&db, file, config, caller.to_owned());
        }
        assert_eq!(runs(), 0, "unchanged document retains the summary memos");
        file.set_text(&mut db).to(changed.clone());
        let after: Vec<_> = ["::quoted", "::braced"]
            .into_iter()
            .map(|caller| summary_dependency_probe(&db, file, config, caller.to_owned()))
            .collect();
        assert_eq!(
            runs(),
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
        for (caller, expected) in ["::quoted", "::braced"].into_iter().zip(after) {
            assert_eq!(
                summary_dependency_probe(&fresh, fresh_file, fresh_config, caller.to_owned()),
                expected
            );
        }
    }

    /// The project proc-name set lifted into salsa
    /// over `file_decls` must extend the signature firewall *across files* — a
    /// body edit in any file recomputes **zero** `project_proc_names` (its
    /// `file_decls` backdates), while a decl change (a new proc) recomputes it
    /// exactly once.  This is the property that stops a keystroke in one file from
    /// waking the whole workspace.
    #[test]
    fn project_proc_names_firewall() {
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
        let runs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("project_proc_names"))
                .count()
        };
        let mut db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        let a = SourceFile::new(
            &db,
            "proc a {} { set x 1 }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let b = SourceFile::new(
            &db,
            "proc b {} { set y 2 }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![a, b]);

        assert_eq!(
            project_proc_names(&db, project).len(),
            2,
            "cold: union of both files' procs"
        );
        let _ = runs(&log);

        // BODY edit to `a` — `file_decls(a)` is byte-identical, so it backdates and
        // the project set does NOT recompute (the firewall, extended across files).
        a.set_text(&mut db)
            .to("proc a {} { set x 999 }\n".to_owned());
        let _ = project_proc_names(&db, project);
        assert_eq!(
            runs(&log),
            0,
            "body edit in one file must not recompute project_proc_names"
        );

        // DECL change to `a` (add `proc c`) — `file_decls(a)` changes, so the
        // project set recomputes exactly once and gains the new proc.
        a.set_text(&mut db)
            .to("proc a {} { set x 999 }\nproc c {} {}\n".to_owned());
        let names = project_proc_names(&db, project);
        assert_eq!(
            runs(&log),
            1,
            "decl change must recompute project_proc_names exactly once"
        );
        assert_eq!(names.len(), 3, "the new proc joins the project set");
    }

    /// Cross-file arity firewall: `project_command_arities`
    /// depends only on each file's `item_sigs`, so a body edit anywhere must
    /// recompute **zero** — while a *signature* edit (changing a proc's parameter
    /// list) must recompute it exactly once and flow the new arity through.  This is
    /// the firewall that keeps the workspace arity table from waking on a keystroke
    /// inside a proc body.
    #[test]
    fn project_command_arities_firewall() {
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
        let runs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("project_command_arities"))
                .count()
        };
        let mut db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        let a = SourceFile::new(
            &db,
            "proc helper {x y} { set z 1 }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let b = SourceFile::new(
            &db,
            "proc other {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![a, b]);

        // Cold: `helper` is a 2-param proc → arity (2, 2).
        assert_eq!(
            project_command_arities(&db, project).get("helper").cloned(),
            Some(vec![(2, 2)]),
            "cold: helper has arity (2, 2)"
        );
        let _ = runs(&log);

        // BODY edit to `helper` — `item_sigs(a)` is byte-identical (signatures
        // unchanged), so the arity table backdates and does NOT recompute.
        a.set_text(&mut db)
            .to("proc helper {x y} { set z 99999 }\n".to_owned());
        let _ = project_command_arities(&db, project);
        assert_eq!(
            runs(&log),
            0,
            "body edit must not recompute project_command_arities"
        );

        // SIGNATURE edit — drop a parameter — `item_sigs(a)` changes, so the table
        // recomputes exactly once and the new arity (1, 1) flows through.
        a.set_text(&mut db)
            .to("proc helper {x} { set z 99999 }\n".to_owned());
        let arities = project_command_arities(&db, project);
        assert_eq!(
            runs(&log),
            1,
            "signature edit must recompute project_command_arities exactly once"
        );
        assert_eq!(
            arities.get("helper").cloned(),
            Some(vec![(1, 1)]),
            "the new parameter list flows through to arity (1, 1)"
        );
    }

    /// Per-symbol cross-file precision: editing an *unrelated* proc's signature in
    /// the defining file must **not** re-run a calling file's `project_diagnostics`.
    ///
    /// `b` calls only `foo` (defined in `a`); `a` also defines `bar`, which `b`
    /// never calls.  Because `project_diagnostics(b)` demands `command_arity` only
    /// for the tails `b` references (`foo`), a signature edit to `bar` recomputes
    /// the whole-project arity table and the `command_arity(foo)` accessor, but the
    /// accessor's projected output for `foo` is unchanged → salsa backdates it →
    /// `project_diagnostics(b)` does **not** re-execute (the per-symbol early-cutoff
    /// the whole-table dependency could not give).  Editing `foo` itself, by
    /// contrast, *must* re-run `b` and flow the new arity through.
    #[test]
    fn project_diagnostics_per_symbol_cutoff() {
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
        // Count `project_diagnostics` re-executions only (not the cheap accessor /
        // aggregate, which legitimately re-run on a signature edit).
        let runs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("project_diagnostics"))
                .count()
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
        // `a` defines `foo` (1 param) and an unrelated `bar`.
        let a = SourceFile::new(
            &db,
            "proc foo {x} {}\nproc bar {y} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        // `b` calls `foo` with one arg — resolves cross-file, fits arity (1, 1),
        // so its diagnostics are empty.  It never references `bar`.
        let b = SourceFile::new(&db, "foo 1\n".to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);

        assert!(
            project_diagnostics(&db, b, cfg, project).is_empty(),
            "cold: foo resolves cross-file (1 arg fits arity 1) → no diagnostics"
        );
        let _ = runs(&log);

        // Signature edit to the UNRELATED `bar` (add a param).  The arity table and
        // `command_arity(bar)` recompute, but `command_arity(foo)` backdates → `b`'s
        // cross-file diagnostics must NOT re-run.
        a.set_text(&mut db)
            .to("proc foo {x} {}\nproc bar {y z} {}\n".to_owned());
        let after_unrelated = project_diagnostics(&db, b, cfg, project);
        assert_eq!(
            runs(&log),
            0,
            "unrelated proc's signature edit must not re-run the caller's project_diagnostics"
        );
        assert!(
            after_unrelated.is_empty(),
            "b's diagnostics unchanged by an edit to a proc it never calls"
        );

        // Control: a signature edit to `foo` itself (now needs 2 args) MUST re-run
        // `b` and surface the cross-file arity error.
        a.set_text(&mut db)
            .to("proc foo {x w} {}\nproc bar {y z} {}\n".to_owned());
        let after_foo = project_diagnostics(&db, b, cfg, project);
        assert_eq!(
            runs(&log),
            1,
            "the called proc's signature edit must re-run the caller's project_diagnostics"
        );
        assert!(
            after_foo.iter().any(|d| d.code == DiagCode::E002),
            "foo now needs 2 args but b passes 1 → cross-file E002 (too few)"
        );
    }

    /// Project diagnostics for a diagnostic-vec comparison: `(code, start, end,
    /// message)` per diagnostic, in order (the analyser emits deterministically).
    #[cfg(test)]
    fn diag_keys(
        diags: &[tcl_compiler::analyser::types::Diagnostic],
    ) -> Vec<(String, u32, u32, String)> {
        diags
            .iter()
            .map(|d| {
                (
                    d.code.to_string(),
                    d.span.start(),
                    d.span.end(),
                    d.message.clone(),
                )
            })
            .collect()
    }

    /// Cross-file W123: a command unresolved locally but
    /// defined as a `proc` in another project file must have its W123 suppressed
    /// when (and only when) that file is in the `Project`.
    #[test]
    fn project_diagnostics_suppresses_cross_file_w123() {
        let db = TclDatabase::default();
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
        let a = SourceFile::new(
            &db,
            "helper foo bar\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let has_helper_w123 = |diags: &[tcl_compiler::analyser::types::Diagnostic]| {
            diags
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("helper"))
        };
        // A alone: `helper` is unresolved → W123 present.
        let proj_a = Project::new(&db, vec![a]);
        assert!(
            has_helper_w123(&project_diagnostics(&db, a, cfg, proj_a)),
            "helper must be unresolved (W123) when B is not in the project"
        );
        // A + B (B defines `proc helper`): cross-file resolved → W123 suppressed.
        let proj_ab = Project::new(&db, vec![a, b]);
        assert!(
            !has_helper_w123(&project_diagnostics(&db, a, cfg, proj_ab)),
            "helper must resolve cross-file (no W123) when B defines proc helper"
        );
    }

    /// The multi-file `incremental == fresh`
    /// differential.  Drive a 2-file project through edits to the *defining* file
    /// (`b`) — adding/removing the procs the calling file (`a`) invokes — and
    /// assert the calling file's cross-file diagnostics always match a fresh
    /// whole-project rebuild.  Catches any untracked read / non-deterministic
    /// fold in `project_diagnostics` or its cross-file dependency edges.
    /// Reduce a `u64` PRNG draw to an index into a small slice. The modulo
    /// is done in `u64` and the result (always `< len`) converts back
    /// losslessly, so there is no truncating cast.
    fn pick_index(r: u64, len: usize) -> usize {
        usize::try_from(r % len as u64).unwrap()
    }

    #[test]
    fn project_diagnostics_incremental_matches_fresh_under_edits() {
        use salsa::Setter as _;
        // `a` calls four commands; `set` is a builtin, the rest resolve only if
        // `b` defines them.
        let a_text = "alpha 1\nbeta 2\ngamma 3\nset x 4\ndelta 5\n";
        let b_variants = [
            "",
            "proc alpha {x} {}\n",
            "proc alpha {x} {}\nproc beta {y} {}\n",
            "proc beta {y} {}\nproc gamma {z} {}\n",
            "namespace eval ns { proc delta {q} {} }\n",
            "proc alpha {x} {}\nproc beta {y} {}\nproc gamma {z} {}\nproc delta {q} {}\n",
        ];
        let mk = |b_text: &str| -> Vec<(String, u32, u32, String)> {
            let db = TclDatabase::default();
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
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
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
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);

        let mut rng = 0x1234_5678_9abc_def0_u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..60 {
            let b_text = b_variants[pick_index(next(), b_variants.len())];
            b.set_text(&mut db).to(b_text.to_owned());
            let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
            let fresh = mk(b_text);
            assert_eq!(inc, fresh, "incremental != fresh after b = {b_text:?}");
        }
    }

    /// Cross-file differential, both files edited: the
    /// stronger sibling of the B-only fuzzer above — drive a 2-file project through
    /// independent edits to **both** the calling file (`a`) and the defining file
    /// (`b`), asserting `a`'s cross-file diagnostics always match a fresh
    /// whole-project rebuild.  Editing the caller changes the call sites (and their
    /// arg counts) while editing the callee changes the resolution/arity domain;
    /// catches any stale cross-file edge that a single-file fuzzer would miss.
    #[test]
    fn project_diagnostics_incremental_matches_fresh_both_files_edited() {
        use salsa::Setter as _;
        // Caller variants: vary which commands are called and with how many args
        // (so the cross-file arity error path is exercised, not just W123).
        let a_variants = [
            "alpha 1\nbeta 2\n",
            "alpha 1 2 3\nbeta\n", // wrong arg counts → arity error once resolved
            "alpha\nset x 1\ngamma 9\n", // gamma may be unresolved → W123
            "beta 1 2\nalpha 7\n",
            "",
        ];
        // Defining variants: vary which procs exist and their arities.
        let b_variants = [
            "",
            "proc alpha {x} {}\n",
            "proc alpha {x} {}\nproc beta {y z} {}\n",
            "proc alpha {a b c} {}\nproc beta {} {}\nproc gamma {q} {}\n",
            "proc alpha {args} {}\nproc beta {x {y 1}} {}\n",
        ];
        let mk = |a_text: &str, b_text: &str| -> Vec<(String, u32, u32, String)> {
            let db = TclDatabase::default();
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
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
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
        let a = SourceFile::new(&db, a_variants[0].to_owned(), "tcl8.6".to_owned(), None);
        let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned(), None);
        let project = Project::new(&db, vec![a, b]);

        let mut rng = 0x0f0f_1234_dead_beef_u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..80 {
            // Edit one file (sometimes both) per round.
            let pick = next() % 3;
            let a_text = a_variants[pick_index(next(), a_variants.len())];
            let b_text = b_variants[pick_index(next(), b_variants.len())];
            if pick != 1 {
                a.set_text(&mut db).to(a_text.to_owned());
            }
            if pick != 0 {
                b.set_text(&mut db).to(b_text.to_owned());
            }
            // Read back the *current* committed texts for the fresh comparison.
            let cur_a = a.text(&db).clone();
            let cur_b = b.text(&db).clone();
            let inc = diag_keys(&project_diagnostics(&db, a, cfg, project));
            let fresh = mk(&cur_a, &cur_b);
            assert_eq!(inc, fresh, "incremental != fresh: a={cur_a:?} b={cur_b:?}");
        }
    }

    /// Cross-file arity: a call to a workspace-defined
    /// proc with the wrong argument count emits the analyser's arity error
    /// (`E003` too many here) — *not* the unrelated `W124` IP-literal warning — and
    /// the W123 it would have drawn is suppressed; a correct count emits neither.
    #[test]
    fn project_diagnostics_emits_cross_file_arity() {
        let db = TclDatabase::default();
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
        // B defines `proc helper {x y}` — arity exactly 2.
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags
                .iter()
                .any(|d| d.code.as_str() == code && d.message.contains("helper"))
        };

        // 3 args to a 2-param proc → E003 (too many), and the W123 is suppressed.
        let a3 = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
        let p3 = Project::new(&db, vec![a3, b]);
        let d3 = project_diagnostics(&db, a3, cfg, p3);
        assert!(
            has(&d3, "E003"),
            "3 args to a 2-param cross-file proc must emit E003"
        );
        assert!(
            !has(&d3, "W124"),
            "must not reuse W124 (the IP-literal warning)"
        );
        assert!(
            !has(&d3, "W123"),
            "W123 must be suppressed once resolved cross-file"
        );

        // Correct arity (2 args) → no arity error and no W123.
        let a2 = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
        let p2 = Project::new(&db, vec![a2, b]);
        let d2 = project_diagnostics(&db, a2, cfg, p2);
        assert!(
            !has(&d2, "E002") && !has(&d2, "E003"),
            "correct arity → no arity error"
        );
        assert!(!has(&d2, "W123"), "resolved cross-file → no W123");
    }

    /// Mixed proc / non-proc tail: when a class (or alias
    /// / ensemble) and a proc share a tail name, a call may dispatch to the
    /// arity-less class command, so no arity error may fire even when the arg count
    /// fits no proc arity — while the call still resolves (no W123).
    #[test]
    fn cross_file_arity_suppressed_for_mixed_proc_nonproc_tail() {
        let db = TclDatabase::default();
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
        // B: a class `Widget` AND a proc whose tail is also `Widget` (arity 1).
        let b = SourceFile::new(
            &db,
            "oo::class create Widget { method draw {} {} }\n\
             namespace eval ns { proc Widget {x} {} }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        // The arity table must record `Widget` as resolvable but arity-less
        // (mixed), so it can never draw an arity error.
        let proj_names = Project::new(&db, vec![b]);
        assert_eq!(
            project_command_arities(&db, proj_names)
                .get("Widget")
                .cloned(),
            Some(Vec::new()),
            "a mixed proc/non-proc tail must carry an empty arity list"
        );
        // A calls `Widget new extra` (3 args) — fits no proc arity, but resolves to
        // the class → neither an arity error nor W123.
        let a = SourceFile::new(
            &db,
            "Widget new extra\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let proj = Project::new(&db, vec![a, b]);
        let d = project_diagnostics(&db, a, cfg, proj);
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
            "a mixed proc/non-proc tail must never draw an arity error"
        );
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
            "the call still resolves cross-file (no W123)"
        );
    }

    /// Object-instance method callbacks resolve cross-file — including when the
    /// dispatch is **inside a proc body**, which the incremental per-item
    /// firewall (`file_analysis_incremental`, used by `project_diagnostics`)
    /// otherwise missed because it defers instance creation to the graft.
    /// Registry object-factories now bind eagerly in an isolated body, so the
    /// in-body `$g walk … -command cb` records the callback and its arity is
    /// resolved against the other file.
    #[test]
    fn cross_file_in_proc_instance_method_callback_arity() {
        let db = TclDatabase::default();
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
        // B defines onNode with 2 params; `graph walk -command` appends 3.
        let b = SourceFile::new(
            &db,
            "proc onNode {a b} { }\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let has_e003 = |src: &str| {
            let a = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
            let p = Project::new(&db, vec![a, b]);
            project_diagnostics(&db, a, cfg, p)
                .iter()
                .any(|d| d.code.as_str() == "E003" && d.message.contains("onNode"))
        };
        // Named factory + dispatch both inside a proc body.
        assert!(
            has_e003("proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n"),
            "in-proc `struct::graph g; g walk -command onNode` must resolve cross-file arity (E003)"
        );
        // Handle form (`set g [struct::graph]`) inside a proc body.
        assert!(
            has_e003("proc build {} {\n set g [struct::graph]\n $g walk root -command onNode\n}\n"),
            "in-proc `set g [struct::graph]; $g walk -command onNode` must resolve cross-file arity"
        );
        // Correct arity (3 params) is silent.
        let b3 = SourceFile::new(
            &db,
            "proc onNode {a b c} { }\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let a_ok = SourceFile::new(
            &db,
            "proc build {} {\n struct::graph g\n g walk root -command onNode\n}\n".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let p_ok = Project::new(&db, vec![a_ok, b3]);
        assert!(
            !project_diagnostics(&db, a_ok, cfg, p_ok)
                .iter()
                .any(|d| (d.code.as_str() == "E003" || d.code.as_str() == "E002")
                    && d.message.contains("onNode")),
            "a correct 3-param in-proc instance callback must be silent cross-file"
        );
    }

    /// Cross-file arity honours `disabled_diagnostics`:
    /// the synthesised arity error is produced *after* the analyser's own code
    /// filter (and the LSP lift doesn't re-filter), so it must replicate it —
    /// disabling `E003` (while keeping W123) must drop the cross-file arity error,
    /// yet the call still resolves (no W123).
    #[test]
    fn cross_file_arity_honors_disabled_code() {
        let db = TclDatabase::default();
        // B defines a 2-param proc; A calls it with 3 args (too many → E003).
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let a = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags
                .iter()
                .any(|d| d.code.as_str() == code && d.message.contains("helper"))
        };

        // E003 enabled (default): wrong arity surfaces as E003, W123 suppressed.
        let cfg_on = AnalyserConfig::new(
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
        let d_on = project_diagnostics(&db, a, cfg_on, proj);
        assert!(has(&d_on, "E003"), "baseline: E003 present when enabled");
        assert!(!has(&d_on, "W123"), "baseline: W123 suppressed (resolved)");

        // E003 disabled (W123 left enabled): no E003, W123 still suppressed — the
        // call genuinely resolves cross-file regardless of the arity-code toggle.
        let cfg_off = AnalyserConfig::new(
            &db,
            vec!["E003".to_owned()],
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        );
        let d_off = project_diagnostics(&db, a, cfg_off, proj);
        assert!(
            !has(&d_off, "E003"),
            "disabling E003 must suppress the cross-file arity error"
        );
        assert!(
            !has(&d_off, "W123"),
            "W123 stays suppressed even with E003 disabled"
        );
    }

    /// Cross-file arity independent of the W123 toggle:
    /// disabling W123 (unknown-command) must NOT also silence cross-file arity —
    /// the analyser drops the W123 markers the arity pass keys off, so
    /// `project_diagnostics` drives off a W123-forced analysis.  A wrong-arg
    /// cross-file call must still report `E003` (matching local arity), with no
    /// W123 leaking through.
    #[test]
    fn cross_file_arity_survives_w123_disabled() {
        let db = TclDatabase::default();
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags.iter().any(|d| d.code.as_str() == code)
        };
        // W123 disabled, E003 left enabled.
        let cfg = AnalyserConfig::new(
            &db,
            vec!["W123".to_owned()],
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        );

        // Wrong arity (3 args to a 2-param proc) → E003 still fires; no W123.
        let bad = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), None);
        let pb = Project::new(&db, vec![bad, b]);
        let d_bad = project_diagnostics(&db, bad, cfg, pb);
        assert!(
            has(&d_bad, "E003"),
            "cross-file arity must survive W123 being disabled, got: {:?}",
            d_bad.iter().map(|d| &d.code).collect::<Vec<_>>()
        );
        assert!(
            !has(&d_bad, "W123"),
            "W123 stays suppressed (it is disabled)"
        );

        // Correct arity → no arity error and no W123 (resolved, W123 disabled).
        let ok = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned(), None);
        let pok = Project::new(&db, vec![ok, b]);
        let d_ok = project_diagnostics(&db, ok, cfg, pok);
        assert!(
            !has(&d_ok, "E002") && !has(&d_ok, "E003"),
            "correct arity → no arity error"
        );
        assert!(
            !has(&d_ok, "W123"),
            "no W123 (disabled, and resolved anyway)"
        );
    }

    /// `tclLsp.extraCommands` threaded through `AnalyserConfig` makes a named
    /// command known, so calling it never draws a W123.
    #[test]
    fn extra_commands_config_suppresses_w123() {
        let db = TclDatabase::default();
        let src = "mylibsend foo\n";
        let has_w123 = |cfg: AnalyserConfig| {
            let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
            file_analysis_incremental(&db, file, cfg)
                .diagnostics
                .iter()
                .any(|d| d.code.as_str() == "W123")
        };
        // Baseline: unknown command → W123.
        let base = AnalyserConfig::new(
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
        assert!(has_w123(base), "baseline W123 expected");
        // With the command declared extra → suppressed.
        let cfg = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            vec!["mylibsend".to_owned()],
            None,
            None,
            0,
            Vec::new(),
            Vec::new(),
        );
        assert!(!has_w123(cfg), "extraCommands should suppress W123");
    }

    /// Cross-file classes: a command resolving to a
    /// class (the class command) defined in another project file is resolved —
    /// W123 suppressed — and, being a non-proc, never draws an arity error.
    #[test]
    fn project_diagnostics_resolves_cross_file_class() {
        let db = TclDatabase::default();
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
        // B defines a TclOO class `Widget`.
        let b = SourceFile::new(
            &db,
            "oo::class create Widget { method draw {} {} }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        // A invokes the `Widget` class command (defined cross-file).
        let a = SourceFile::new(&db, "Widget new\n".to_owned(), "tcl8.6".to_owned(), None);
        let proj = Project::new(&db, vec![a, b]);
        let d = project_diagnostics(&db, a, cfg, proj);
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::W123 && x.message.contains("Widget")),
            "cross-file class command must resolve (no W123)"
        );
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
            "a class command has no proc arity → no arity error"
        );
    }

    /// Cross-file arity edge cases: exercise the
    /// `proc_arity` `(min, max)` computation across required-only, optional
    /// defaults, a trailing `args` (unbounded), and the no-parameter proc — the
    /// subtle part where an off-by-one would mis-fire the arity error.  Checks the
    /// *specific* code too: too-few → `E002`, too-many → `E003`.
    #[test]
    fn cross_file_arity_edge_cases() {
        let db = TclDatabase::default();
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
        let b = SourceFile::new(
            &db,
            "proc two {a b} {}\nproc opt {a {b 1}} {}\nproc variadic {a args} {}\nproc none {} {}\n"
                .to_owned(),
            "tcl8.6".to_owned(),
         None,
        );
        // The cross-file arity code drawn by calling `src` (file A over {A, B}):
        // Some("E002") too few, Some("E003") too many, None if it fits.
        let arity_code = |src: &str| -> Option<String> {
            let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
            let proj = Project::new(&db, vec![a, b]);
            project_diagnostics(&db, a, cfg, proj)
                .iter()
                .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
                .map(|d| d.code.to_string())
        };
        let e002 = || Some("E002".to_owned());
        let e003 = || Some("E003".to_owned());
        // two {a b} → arity (2, 2).
        assert_eq!(
            arity_code("two 1\n"),
            e002(),
            "1 arg to a 2-param proc → too few"
        );
        assert_eq!(arity_code("two 1 2\n"), None, "2 args → ok");
        assert_eq!(arity_code("two 1 2 3\n"), e003(), "3 args → too many");
        // opt {a {b 1}} → arity (1, 2).
        assert_eq!(arity_code("opt\n"), e002(), "0 args to (1,2) → too few");
        assert_eq!(arity_code("opt 1\n"), None, "1 arg to (1,2) → ok");
        assert_eq!(arity_code("opt 1 2\n"), None, "2 args to (1,2) → ok");
        assert_eq!(
            arity_code("opt 1 2 3\n"),
            e003(),
            "3 args to (1,2) → too many"
        );
        // variadic {a args} → arity (1, unbounded).
        assert_eq!(
            arity_code("variadic\n"),
            e002(),
            "0 args to (1,∞) → too few"
        );
        assert_eq!(arity_code("variadic 1\n"), None, "1 arg → ok");
        assert_eq!(
            arity_code("variadic 1 2 3 4 5\n"),
            None,
            "many → ok (trailing args)"
        );
        // none {} → arity (0, 0).
        assert_eq!(arity_code("none\n"), None, "0 args to a no-param proc → ok");
        assert_eq!(
            arity_code("none 1\n"),
            e003(),
            "1 arg to a 0-param proc → too many"
        );
    }

    /// A proc whose parameter-list word is **computed** has
    /// unknown formals, so the *cross-file* arity check must abstain.
    /// `ItemSig` must carry an "unknown, not none" flag distinct from a
    /// genuinely empty parameter list — without it the cross-file table
    /// would read the empty list as "takes no arguments" and every call
    /// would draw a false `E003` — on code both tclsh 9.0.4 and 8.6.16 run
    /// (`proc makeargs {} {return {a b}}`; `proc p [makeargs] {…}`;
    /// `info args p` → `a b`; `p 1 2` → runs).
    #[test]
    fn cross_file_arity_abstains_for_computed_parameter_lists() {
        let db = TclDatabase::default();
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
        let b = SourceFile::new(
            &db,
            "proc makeargs {} { return {a b} }\n\
             proc computed [makeargs] { return 1 }\n\
             set params {x y}\n\
             proc dollar $params { return 1 }\n\
             proc literal {a b} { return 1 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let arity_code = |src: &str| -> Option<String> {
            let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
            let proj = Project::new(&db, vec![a, b]);
            project_diagnostics(&db, a, cfg, proj)
                .iter()
                .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
                .map(|d| d.code.to_string())
        };
        // FP guards — every call count is acceptable for an unknown signature.
        for call in [
            "computed\n",
            "computed 1\n",
            "computed 1 2\n",
            "dollar 1 2\n",
        ] {
            assert_eq!(
                arity_code(call),
                None,
                "computed parameter list must abstain, not draw an arity error for {call:?}"
            );
        }
        // TP control — a literal list in the same file still checks arity.
        assert_eq!(arity_code("literal 1 2\n"), None, "2 args → ok");
        assert_eq!(
            arity_code("literal 1 2 3\n"),
            Some("E003".to_owned()),
            "a literal list is still checked"
        );
    }

    /// A required parameter
    /// positioned *after* a defaulted one does not lower the minimum by
    /// the defaulted parameters ahead of it — Tcl's argument binding is
    /// strictly positional, so supplying a value for the later required
    /// parameter requires also supplying one for every position before
    /// it, including the "optional" one. Confirmed against real `tclsh`
    /// 9.0.4: `proc opt {a {b 5} c} {}` accepts exactly 3 arguments,
    /// never 2 — computing `min` as the count of non-default params (2)
    /// would silently accept a 2-argument call that real Tcl rejects.
    #[test]
    fn cross_file_arity_required_after_default_forces_exact_count() {
        let db = TclDatabase::default();
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
        let b = SourceFile::new(
            &db,
            "proc opt {a {b 5} c} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let arity_code = |src: &str| -> Option<String> {
            let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
            let proj = Project::new(&db, vec![a, b]);
            project_diagnostics(&db, a, cfg, proj)
                .iter()
                .find(|d| d.code == DiagCode::E002 || d.code == DiagCode::E003)
                .map(|d| d.code.to_string())
        };
        assert_eq!(
            arity_code("opt 1\n"),
            Some("E002".to_owned()),
            "1 arg → too few (min is 3, not 2)"
        );
        assert_eq!(
            arity_code("opt 1 2\n"),
            Some("E002".to_owned()),
            "2 args → still too few — real tclsh rejects this exact call"
        );
        assert_eq!(arity_code("opt 1 2 3\n"), None, "3 args → ok");
        assert_eq!(
            arity_code("opt 1 2 3 4\n"),
            Some("E003".to_owned()),
            "4 args → too many"
        );
    }

    /// Conservative arity: a `{*}`-expanded call has an
    /// unknown runtime arg count (`argc == None`), so it must never draw an arity
    /// error even though its literal word count looks wrong — while still resolving
    /// (no W123).
    #[test]
    fn cross_file_arity_skips_expanded_call() {
        let db = TclDatabase::default();
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
        let b = SourceFile::new(
            &db,
            "proc two {a b} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        // `two {*}$lst` — one literal word, `{*}`-expanded → runtime arity unknown.
        let a = SourceFile::new(
            &db,
            "set lst {1 2 3}\ntwo {*}$lst\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let proj = Project::new(&db, vec![a, b]);
        let d = project_diagnostics(&db, a, cfg, proj);
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::E002 || x.code == DiagCode::E003),
            "a {{*}}-expanded call must not draw an arity error (argc unknown)"
        );
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::W123 && x.message.contains("two")),
            "the call still resolves cross-file (no W123)"
        );
    }

    /// Nested cross-file arity: a wrong-arg call to a
    /// workspace proc *inside a command substitution* (`set x [helper a b c]`)
    /// must still draw the cross-file arity error — the nested call's argument
    /// count is statically known.
    #[test]
    fn cross_file_arity_in_command_substitution() {
        let db = TclDatabase::default();
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
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        // `helper` called with 3 args inside a `[…]` substitution → too many.
        let a = SourceFile::new(
            &db,
            "set x [helper a b c]\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let proj = Project::new(&db, vec![a, b]);
        let d = project_diagnostics(&db, a, cfg, proj);
        assert!(
            d.iter()
                .any(|x| x.code == DiagCode::E003 && x.message.contains("helper")),
            "nested cross-file call must draw E003, got: {:?}",
            d.iter().map(|x| (&x.code, &x.message)).collect::<Vec<_>>()
        );
        assert!(
            !d.iter()
                .any(|x| x.code == DiagCode::W123 && x.message.contains("helper")),
            "the nested call still resolves cross-file (no W123)"
        );
    }

    /// Analyse a single-file project and return its diagnostic codes+messages.
    fn callback_arity_diagnostics(src: &str) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
        let db = TclDatabase::default();
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
        let f = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let proj = Project::new(&db, vec![f]);
        project_diagnostics(&db, f, cfg, proj).as_ref().clone()
    }

    fn callback_arity_codes(src: &str) -> Vec<(String, String)> {
        callback_arity_diagnostics(src)
            .iter()
            .map(|d| (d.code.as_str().to_owned(), d.message.clone()))
            .collect()
    }

    fn assert_callback_source_count(
        source: &str,
        input: &[u8],
        expected: tcl_compiler::analyser::SourceCallbackArgumentCounts,
    ) {
        let diagnostics = callback_arity_diagnostics(source);
        let subject = diagnostics
            .iter()
            .find_map(|diagnostic| {
                let subject = diagnostic.callback_source_arity()?;
                (diagnostic.code == DiagCode::E003
                    && subject.prefix().name_input().bytes() == input)
                    .then_some(subject)
            })
            .unwrap_or_else(|| {
                panic!("{source}: no genuine E003 source subject in {diagnostics:?}")
            });
        assert_eq!(subject.argument_counts(), &expected);
        assert!(subject.source_lookup().is_some());
    }

    #[test]
    fn original_callback_diagnostics_keep_typed_subject_and_ignore_mutable_counts() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        // Current source headers only; no callback registration or execution.
        let db = TclDatabase::default();
        let file = SourceFile::new(
            &db,
            "proc cb {a b} {return 0}\nlsort -command {cb fixed} {3 1 2}".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let project = Project::new(&db, vec![file]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(file.text(&db), "tcl9.0");
        let mut invocations = analysis.command_invocations;
        for invocation in &mut invocations {
            if invocation.original_callback_prefix.is_some() {
                invocation.name = "wrong presentation".to_owned();
                invocation.range = tcl_lexer::Span::new(0, 1);
                invocation.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
                invocation.callback_baked_args = usize::MAX;
                invocation.original_name_input = None;
                invocation.original_lookup = None;
            }
        }
        let mut output = Vec::new();
        apply_original_callback_arity(&db, project, &mut output, &invocations, |_| false);
        let diagnostic = output
            .first()
            .expect("authentic prefix, not mutable scalar counts");
        assert_eq!(output.len(), 1);
        assert_eq!(diagnostic.code, DiagCode::E003);
        let subject = diagnostic.callback_source_arity().unwrap();
        assert_eq!(subject.prefix().baked_argument_count(), 1);
        assert_eq!(
            subject.argument_counts(),
            &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3])
        );
        assert_eq!(&file.text(&db)[diagnostic.span.as_range()], "cb");
        assert!(
            callback_command_keys(
                invocations
                    .iter()
                    .find(|inv| inv.original_callback_prefix.is_some())
                    .unwrap()
            )
            .is_empty()
        );
        assert_eq!(
            tcl_lsp_core::diagnostic_subject::diagnostic_subject_data(diagnostic).unwrap()["subject"]
                ["purpose"],
            "signature"
        );
    }

    #[test]
    fn original_callback_signature_query_updates_headers_without_tail_substitution() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        use salsa::Setter as _;
        let mut db = TclDatabase::default();
        let definitions = SourceFile::new(&db,"namespace eval a {proc cb {one} {return FIRST}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned(),"tcl9.0".to_owned(),None);
        let call_source = "lsort -command ::a::cb {3 1 2}";
        let calls = SourceFile::new(&db, call_source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![definitions, calls]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(call_source, "tcl9.0");
        let prefix = analysis
            .command_invocations
            .iter()
            .find_map(|inv| inv.original_callback_prefix.as_ref())
            .unwrap();
        let lookup = prefix.lookup().unwrap();
        let first = original_lookup_command_signatures(&db, project, lookup).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(
            first[0].formal_count_projection().arity(),
            tcl_registry::Arity::exact(1)
        );
        definitions.set_text(&mut db).to("namespace eval a {proc cb {one} {return LATER}}\nnamespace eval b {proc cb {one two} {return 0}}".to_owned());
        assert_eq!(
            original_lookup_command_signatures(&db, project, lookup).unwrap(),
            first,
            "body-free header equality"
        );
        definitions.set_text(&mut db).to("namespace eval a {proc cb {one two} {return LATER}}\nnamespace eval b {proc cb {one} {return 0}}".to_owned());
        let updated = original_lookup_command_signatures(&db, project, lookup).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(
            updated[0].formal_count_projection().arity(),
            tcl_registry::Arity::exact(2)
        );
        assert!(
            tcl_compiler::analyser::SourceCallbackAritySubject::from_source_signatures(
                Arc::clone(prefix),
                &updated
            )
            .is_none()
        );
    }

    #[test]
    fn original_callback_signature_gap_retains_disjoint_source_headers() {
        // naming.diagnostic.original-callback-signature-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-callback-signature-subject.md
        // Two current source candidates, not two installed implementations.
        let db = TclDatabase::default();
        let one = SourceFile::new(&db, "proc cb {a} {}".to_owned(), "tcl9.0".to_owned(), None);
        let three = SourceFile::new(
            &db,
            "proc cb {a b c} {}".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let source = "lsort -command cb {1 2}";
        let call = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![one, three, call]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let mut diagnostics = Vec::new();
        apply_original_callback_arity(
            &db,
            project,
            &mut diagnostics,
            &analysis.command_invocations,
            |_| false,
        );
        let diagnostic = diagnostics
            .first()
            .expect("finite count falls in a signature gap");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostic.code, DiagCode::E005);
        let subject = diagnostic.callback_source_arity().unwrap();
        assert_eq!(subject.declarations().len(), 2);
        assert_eq!(
            subject.issue(),
            tcl_compiler::analyser::SourceCallbackArityIssue::NoCompatibleSignature { supplied: 2 }
        );
    }

    #[test]
    fn qualified_callback_arity_keeps_same_tailed_namespaces_separate() {
        let diagnostics = callback_arity_codes(
            "namespace eval a {proc cb {one} {return 0}}\n\
             namespace eval b {proc cb {one two} {return 0}}\n\
             lsort -command ::a::cb {3 1 2}\n",
        );
        assert!(
            diagnostics.iter().any(|(code, _)| code == "E003"),
            "{diagnostics:?}"
        );
        let absent = callback_arity_codes(
            "namespace eval b {proc cb {one} {return 0}}\n\
             lsort -command ::absent::cb {3 1 2}\n",
        );
        assert!(
            absent
                .iter()
                .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005")),
            "{absent:?}"
        );
    }

    #[test]
    fn callback_arity_mismatch_draws_e002() {
        // `lsort -command` appends 2 (Exactly(2)); `badCb` needs 3 → E002 too few.
        let d =
            callback_arity_codes("proc badCb {a b c} { return 0 }\nlsort -command badCb {3 1 2}\n");
        assert!(
            d.iter().any(|(c, m)| c == "E002" && m.contains("badCb")),
            "a callback proc needing 3 args used where 2 are appended must draw E002; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_correct_is_silent() {
        // A 2-arg callback matches lsort's Exactly(2) → no arity error (TN).
        let d =
            callback_arity_codes("proc goodCb {a b} { return 0 }\nlsort -command goodCb {3 1 2}\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correctly-sized callback must draw no arity error; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_too_many_draws_e003() {
        // A 1-param callback where lsort appends 2 → E003 too many.
        let d =
            callback_arity_codes("proc oneArg {a} { return 0 }\nlsort -command oneArg {3 1 2}\n");
        assert!(
            d.iter().any(|(c, m)| c == "E003" && m.contains("oneArg")),
            "a 1-param callback fed 2 args must draw E003; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_args_catchall_is_silent() {
        // FP guard: an `args` catch-all accepts any count → no arity error.
        let d =
            callback_arity_codes("proc anyN {args} { return 0 }\nlsort -command anyN {3 1 2}\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "an `args`-catchall callback must draw no arity error; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_braced_prefix_bakes_extra_args_too_few() {
        // `-command {cb 99}` bakes 1 extra arg ahead of `lsort`'s own
        // appended 2, for 3 total; `cb` needs 4 → E002.  A braced
        // multi-word prefix must be recorded as an invocation, or this
        // would draw nothing at all.
        let d = callback_arity_codes(
            "proc cb {a b c d} { return 0 }\nlsort -command {cb 99} {3 1 2}\n",
        );
        assert!(
            d.iter().any(|(c, m)| c == "E002" && m.contains("cb")),
            "a braced prefix's baked arg must count toward the total; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_braced_prefix_bakes_extra_args_exact_match_is_silent() {
        // Same shape, but `cb` needs exactly 3 (1 baked + 2 appended) — TN.
        let d =
            callback_arity_codes("proc cb {a b c} { return 0 }\nlsort -command {cb 99} {3 1 2}\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "1 baked + 2 appended matching cb's 3 params must be silent; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_braced_prefix_bakes_extra_args_too_many() {
        // `-command {cb 99 88}` bakes 2 + appended 2 = 4, but `cb` takes
        // only 3 → E003.
        let d = callback_arity_codes(
            "proc cb {a b c} { return 0 }\nlsort -command {cb 99 88} {3 1 2}\n",
        );
        assert!(
            d.iter().any(|(c, m)| c == "E003" && m.contains("cb")),
            "2 baked + 2 appended against a 3-param cb must draw E003; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_braced_prefix_dynamic_head_is_never_checked() {
        // FP guard: `{$cb 99}` — a dynamic head inside the braces — can't be
        // resolved to a proc; must never be flagged (and must not panic on
        // the list-parse).
        let d = callback_arity_codes(
            "proc cb {a b c d} { return 0 }\nset cb cb\nlsort -command {$cb 99} {3 1 2}\n",
        );
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a dynamic braced-prefix head must never be arity-checked; got {d:?}"
        );
    }

    #[test]
    fn execution_trace_source_counts_keep_unavailable_future_frame() {
        // naming.callback.lookup-scope-owner
        // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
        // Registration syntax retains suffix metadata; it does not observe
        // the command table or namespace of a future triggering frame.
        for (operations, appended) in [
            ("enter", 2),
            ("enterstep", 2),
            ("leave", 4),
            ("leavestep", 4),
        ] {
            let source = format!(
                "proc h {{a b c}} {{return 0}}\ntrace add execution somecmd {operations} h"
            );
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
            let prefix = analysis
                .command_invocations
                .iter()
                .find_map(|inv| inv.original_callback_prefix.as_ref())
                .expect(operations);
            assert_eq!(
                prefix.appended_arity(),
                Some(tcl_registry::AppendedArity::Exactly(appended))
            );
            assert_eq!(
                prefix.scope(),
                Some(tcl_registry::ScriptLookupScope::TriggerFrame)
            );
            assert!(prefix.lookup().is_none());
            assert!(
                callback_arity_codes(&source)
                    .iter()
                    .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
            );
        }
    }

    #[test]
    fn mixed_execution_trace_source_counts_do_not_issue_a_future_lookup() {
        // naming.callback.lookup-scope-owner
        // docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md
        for params in [
            "a b",
            "a b c",
            "a b c d",
            "a b {c default} {d default}",
            "a b args",
        ] {
            let source = format!(
                "proc h {{{params}}} {{return 0}}\ntrace add execution somecmd {{enter leave}} h"
            );
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl9.0");
            let prefix = analysis
                .command_invocations
                .iter()
                .find_map(|inv| inv.original_callback_prefix.as_ref())
                .expect(params);
            assert_eq!(
                prefix
                    .appended_arity()
                    .unwrap()
                    .exact_counts()
                    .unwrap()
                    .collect::<Vec<_>>(),
                vec![2, 4]
            );
            assert!(prefix.lookup().is_none());
            assert!(
                callback_arity_codes(&source)
                    .iter()
                    .all(|(code, _)| !matches!(code.as_str(), "E002" | "E003" | "E005"))
            );
        }
    }

    #[test]
    fn execution_trace_dynamic_and_malformed_operation_lists_abstain() {
        for source in [
            "proc h {a b c} { return 0 }\nset operations {enter leave}\ntrace add execution somecmd $operations h\n",
            "proc h {a b c} { return 0 }\ntrace add execution somecmd \"{enter\" h\n",
        ] {
            let diagnostics = callback_arity_codes(source);
            assert!(
                !diagnostics
                    .iter()
                    .any(|(code, _)| matches!(code.as_str(), "E002" | "E003" | "E005")),
                "an unproved operation list must not drive callback arity: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn callback_arity_namespace_unknown_zero_param_draws_e003() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        // The absolute head selects a conditional source declaration without
        // borrowing the unentered trigger's namespace. AtLeast(1) cannot fit
        // the zero-parameter source header; registration itself calls nothing.
        assert_callback_source_count(
            "proc h {} { return 0 }\nnamespace unknown ::h\n",
            b"::h",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
        );
    }

    #[test]
    fn callback_arity_namespace_unknown_relative_trigger_does_not_borrow_installer_frame() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let diagnostics = callback_arity_codes("proc h {} {}; namespace unknown h");
        assert!(
            !diagnostics
                .iter()
                .any(|(code, _)| code == "E002" || code == "E003")
        );
    }

    #[test]
    fn callback_arity_unavailable_package_descriptors_do_not_select_local_headers() {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        for source in [
            "proc cb {} {}; scale .s -command cb",
            "proc cb {x y} {}; math::calculus::integral 0 1 100 cb",
            "proc cb {x} {}; smtp::sendmessage $t -tlspolicy cb",
            "proc cb {x} {}; tcl::chan::halfpipe -write-command cb",
            "proc cb {} {}; mime::getbody $t -command cb",
            "proc cb {x y} {}; struct::graph g; g walk root -command cb",
            "proc cb {x y} {}; struct::tree t; t walkproc root cb",
            "package require smtp; proc cb {x} {}; smtp::sendmessage $t -tlspolicy ::cb",
            "package require mime; proc cb {} {}; mime::getbody $t -command ::cb",
            "package require struct::graph; proc cb {x y} {}; struct::graph g; g walk root -command ::cb",
            "package require struct::tree; proc cb {x y} {}; struct::tree t; t walkproc root ::cb",
        ] {
            let diagnostics = callback_arity_codes(source);
            assert!(
                !diagnostics
                    .iter()
                    .any(|(code, _)| code == "E002" || code == "E003"),
                "{source}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn callback_arity_package_unknown_variadic_handler_is_silent() {
        // FP guard: `package unknown` appends AtLeast(1); an `args` handler
        // absorbs any count → no arity error (the canonical handler shape).
        let d = callback_arity_codes("proc h {args} { return 0 }\npackage unknown h\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a variadic `package unknown` handler must draw no arity error; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_tk_scale_command_arity_checked() {
        // Conditional source descriptor: `scale -command` appends Exactly(1).
        // A 0-param callback can't accept it → E003; a bareword 1-param callback
        // is silent (TN).
        let bad = callback_arity_codes(
            "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {} { }\nscale .s -command ::onChange\n",
        );
        assert!(
            bad.iter()
                .any(|(c, m)| c == "E003" && m.contains("onChange")),
            "a 0-param `scale -command` callback (1 appended) must draw E003; got {bad:?}"
        );
        let ok = callback_arity_codes(
            "# tcl-lsp: requires Tk\npackage require Tk\nproc onChange {v} { }\nscale .s -command ::onChange\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 1-param scale callback must be silent; got {ok:?}"
        );
        // A braced widget-path scroll callback is never arity-checked (not a
        // literal bareword head) — no false arity error.
        let widget = callback_arity_codes(
            "# tcl-lsp: requires Tk\npackage require Tk\nlistbox .lb -yscrollcommand {.sb set}\n",
        );
        assert!(
            !widget.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a braced widget-path scroll callback must not draw an arity error; got {widget:?}"
        );
    }

    #[test]
    fn callback_arity_tcllib_calculus_func_arity_checked() {
        // `math::calculus::integral begin end nosteps func` calls `func x`
        // (Exactly(1), man-page-pinned).  A 2-param func is under-fed → E002.
        let d = callback_arity_codes(
            "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x y} { expr {$x + $y} }\nmath::calculus::integral 0 1 100 ::f\n",
        );
        assert!(
            d.iter().any(|(c, m)| c == "E002" && m.contains("'::f'")),
            "a 2-param func where calculus::integral appends 1 must draw E002; got {d:?}"
        );
        // The correct 1-param shape is silent (TN).
        let ok = callback_arity_codes(
            "# tcl-lsp: requires math::calculus\npackage require math::calculus\nproc f {x} { expr {$x * 2} }\nmath::calculus::integral 0 1 100 ::f\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 1-param calculus func must be silent; got {ok:?}"
        );
    }

    #[test]
    fn callback_arity_unknown_appended_never_fires() {
        // FP guard: `coroprobe` carries `Unknown` appended arity (depends on
        // the yield point), so the injected command is a reference only —
        // never arity-checked, whatever its param count. `coroinject` is not
        // an example of this guard: its own implementation always appends a
        // verified `Exactly(2)` (see `coroinject.rs`), so a 0-param `h` there
        // correctly draws E003 instead.
        let d = callback_arity_codes("proc h {} { return 0 }\ncoroprobe myCoro h\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "an Unknown-arity callback must never draw an arity error; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_option_value_exactly_checked() {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        // Required package metadata supplies source grammar, never loader or
        // future callback execution. The subject retains the actual prefix.
        // `smtp -tlspolicy` and `halfpipe -write-command` both append exactly 2;
        // a 1-param callback is over-fed → E003, a 2-param callback is silent.
        assert_callback_source_count(
            "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
            b"::pol",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
        );
        let smtp_ok = callback_arity_codes(
            "# tcl-lsp: requires smtp\npackage require smtp\nproc pol {code diag} { }\nsmtp::sendmessage $t -tlspolicy ::pol\n",
        );
        assert!(
            !smtp_ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 2-param -tlspolicy callback must be silent; got {smtp_ok:?}"
        );
        assert_callback_source_count(
            "# tcl-lsp: requires tcl::chan::halfpipe\npackage require tcl::chan::halfpipe\nproc w {chan} { }\ntcl::chan::halfpipe -write-command ::w\n",
            b"::w",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![2]),
        );
    }

    #[test]
    fn callback_arity_mime_getbody_command_atleast_one() {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        // Required package metadata supplies source grammar, never loader or
        // future callback execution. The subject retains the actual prefix.
        // `mime::getbody -command` appends AtLeast(1) (reason keyword + optional
        // payload).  A 0-param callback can't accept the reason word → E003; the
        // canonical `{reason args}` shape is silent (open-ended max ⇒ no
        // false "too many").
        assert_callback_source_count(
            "# tcl-lsp: requires mime\npackage require mime\nproc cb {} { }\nmime::getbody $t -command ::cb\n",
            b"::cb",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::AtLeast(1),
        );
        let ok = callback_arity_codes(
            "# tcl-lsp: requires mime\npackage require mime\nproc cb {reason args} { }\nmime::getbody $t -command ::cb\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a `{{reason args}}` mime -command callback must be silent; got {ok:?}"
        );
        // FP guard: comm's 14-arg reply callback is virtually always `{args}` —
        // the catch-all absorbs all 14 → no arity error.
        let comm = callback_arity_codes(
            "# tcl-lsp: requires comm\npackage require comm\nproc reply {args} { }\ncomm::comm send -command ::reply $id {list x}\n",
        );
        assert!(
            !comm.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a variadic comm -command ::reply handler must be silent; got {comm:?}"
        );
    }

    #[test]
    fn callback_arity_struct_graph_walk_command_checked() {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        // Required package metadata supplies source grammar, never loader or
        // future callback execution. The subject retains the actual prefix.
        // `$g walk … -command ::cb` (object instance method) appends 3 (action
        // graphName node).  A 2-param callback is over-fed → E003; a 3-param one
        // is silent.  Exercises both the named (`struct::graph name`) and handle
        // (`set g [struct::graph]`) instance forms.
        assert_callback_source_count(
            "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc twoP {a b} { }\nstruct::graph myG\nmyG walk root -command ::twoP\n",
            b"::twoP",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
        );
        let ok = callback_arity_codes(
            "# tcl-lsp: requires struct::graph\npackage require struct::graph\nproc threeP {a b c} { }\nset g [struct::graph]\n$g walk root -command ::threeP\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 3-param graph walk callback must be silent; got {ok:?}"
        );
    }

    #[test]
    fn callback_arity_struct_tree_walkproc_checked() {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        // Required package metadata supplies source grammar, never loader or
        // future callback execution. The subject retains the actual prefix.
        // `$t walkproc … cmdprefix` (trailing positional prefix) appends 3 (tree
        // node action).  A 2-param callback → E003.
        assert_callback_source_count(
            "# tcl-lsp: requires struct::tree\npackage require struct::tree\nproc twoP {a b} { }\nstruct::tree myT\nmyT walkproc root ::twoP\n",
            b"::twoP",
            tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![3]),
        );
    }

    /// Regression (whole-file-shift determinism): a pure prepend that shifts every
    /// procedure must leave every `function_lattice` a cache hit — reliably, not by
    /// HashMap-seed luck.  Before `prepare_cfg_context` was made deterministic, the
    /// memo key flaked and this could re-execute *all* procedures.  Two procedures
    /// share the short name `x` (the collision that exposed the nondeterminism).
    #[test]
    fn function_lattice_reused_on_whole_file_shift() {
        use salsa::Setter as _;
        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let l = Arc::clone(&log);
        let sink = move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        };
        let mut db = TclDatabase {
            storage: salsa::Storage::new(Some(Box::new(sink))),
        };
        let src = "namespace eval ::a { proc x {p} { set q $p; return $q } }\n\
                   namespace eval ::b { proc x {p} { set q $p; return $q } }\n\
                   proc top {} { set z 1 }\n";
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
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let _ = file_analysis_incremental(&db, file, cfg);
        log.lock().unwrap().clear();
        // Pure whole-file shift: prepend a blank line (every proc shifts, none
        // change).  All offset-0 lattice keys are unchanged -> all cache hits.
        file.set_text(&mut db).to(format!("\n{src}"));
        let _ = file_analysis_incremental(&db, file, cfg);
        let reexec = std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("function_lattice"))
            .count();
        assert_eq!(
            reexec, 0,
            "whole-file shift must reuse every proc lattice (deterministic key): {reexec} re-ran"
        );
    }

    /// A length-changing body edit to one procedure shifts the others but must
    /// recompute exactly ONE `function_lattice` (the salsa-native per-procedure
    /// lattice is offset-invariant: an unedited-but-shifted body interns to the
    /// same key and is a cache hit, rebased to its new offset).
    #[test]
    fn function_lattice_reused_on_body_shift() {
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
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        let init = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            init.iter()
                .filter(|s| s.contains("function_lattice"))
                .count(),
            2,
            "initial: both procedures' lattices built: {init:?}"
        );

        // Edit a's body length — shifts b's offset; b's offset-0 body is
        // unchanged, so its `function_lattice` key is unchanged (cache hit).
        file.set_text(&mut db)
            .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
        let _ = file_analysis_incremental(&db, file, cfg);
        let after = std::mem::take(&mut *log.lock().unwrap());
        assert_eq!(
            after
                .iter()
                .filter(|s| s.contains("function_lattice"))
                .count(),
            1,
            "length-changing body edit -> exactly ONE lattice recomputes (offset-invariant): {after:?}"
        );
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

    /// A procedure that takes a parameter every caller passes the same literal
    /// for (interprocedural `param_constants`) must engage the salsa-native
    /// lattice memo — historically it bypassed it and was rebuilt fresh every
    /// edit.  Folding the encoded seeds into [`FnLatticeKey`] means: (1) all
    /// three procedures' lattices are `function_lattice` executions on the cold
    /// build; (2) a length-changing edit to an *unrelated* proc shifts the
    /// param-constant callee but reuses its lattice (offset-invariant cache
    /// hit), recomputing exactly one; (3) changing the caller's literal
    /// re-interns the callee's key, so its lattice rebuilds.
    #[test]
    fn function_lattice_memoises_param_constant_procs() {
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
        let lattice_execs = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .into_iter()
                .filter(|s| s.contains("function_lattice"))
                .count()
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
        // `other` first so editing it shifts the two below; `target` takes a
        // param `caller` always passes the literal `42` for -> param_constants.
        let file = SourceFile::new(
            &db,
            "proc other {} { set z 1 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
            None,
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        assert_eq!(
            lattice_execs(&log),
            3,
            "cold build: every proc (incl. the param-constant callee) memoised"
        );

        // Length-changing edit to `other` shifts `target`/`caller`; their
        // offset-0 bodies (and `target`'s param_constants) are unchanged, so
        // they are cache hits — exactly one lattice recomputes.
        file.set_text(&mut db)
            .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
                .to_owned());
        let _ = file_analysis_incremental(&db, file, cfg);
        assert_eq!(
            lattice_execs(&log),
            1,
            "unrelated body edit -> param-constant callee reused (offset-invariant hit)"
        );

        // Change the caller's literal -> `target`'s param_constants change ->
        // its `FnLatticeKey` re-interns -> its lattice rebuilds.
        file.set_text(&mut db)
            .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 99 }\n"
                .to_owned());
        let _ = file_analysis_incremental(&db, file, cfg);
        assert!(
            lattice_execs(&log) >= 1,
            "caller literal change rebuilds the param-constant callee's lattice"
        );
    }

    /// Both diagnostics consumers must **share one `compilation_unit` build per
    /// edit** when their lexer configs coincide (every dialect but `tcl8.4` /
    /// `f5-irules`): demanding `file_analysis_incremental` then
    /// `compiler_check_diagnostics` in the same revision executes
    /// `compilation_unit` exactly once.  For `tcl8.4` the configs differ
    /// (`expand_syntax`), so each consumer builds its own — executed twice.
    #[test]
    fn compilation_unit_shared_across_consumers() {
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
        let src = "proc a {x} { return $x }\nproc b {} { a 1 }\n";
        let count_cu = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .iter()
                .filter(|s| s.contains("compilation_unit"))
                .count()
        };

        // tcl8.6: both consumers intern the document's environment id, so
        // they share one build.
        let file86 = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let _ = file_analysis_incremental(&db, file86, cfg);
        let _ = compiler_check_diagnostics(&db, file86, cfg);
        assert_eq!(
            count_cu(&log),
            1,
            "tcl8.6: both consumers share exactly one compilation_unit build"
        );

        // `tcl8.4` is the interesting case: its `expand_syntax` differs from
        // `LexerConfig::default()`, so keying on the truncated three-field
        // config (rather than the environment id) would intern two separate
        // entries — one for the analyser tail's default config and one for
        // the checks pass's `for_dialect("tcl8.4")`. Keying on the
        // environment id instead means both consumers lex under the
        // document's own environment, so `tcl8.4` shares one build like
        // every other environment.
        let file84 = SourceFile::new(&db, src.to_owned(), "tcl8.4".to_owned(), None);
        let _ = file_analysis_incremental(&db, file84, cfg);
        let _ = compiler_check_diagnostics(&db, file84, cfg);
        assert_eq!(
            count_cu(&log),
            1,
            "tcl8.4: one environment id -> one shared build, as for every dialect"
        );

        // A fresh edit re-shares for tcl8.6 (one build for the new revision).
        file86
            .set_text(&mut db)
            .to("proc a {x} { return $x }\nproc b {} { a 2 }\n".to_owned());
        let _ = file_analysis_incremental(&db, file86, cfg);
        let _ = compiler_check_diagnostics(&db, file86, cfg);
        assert_eq!(
            count_cu(&log),
            1,
            "after an edit, tcl8.6 again shares one build across both consumers"
        );
    }
}

#[cfg(test)]
mod compiler_snapshot_memo_tests {
    use super::*;
    use tcl_dialect::model::{DialectPoint, Release};

    #[test]
    fn same_name_body_memos_preserve_the_selected_execution_snapshot() {
        let db = TclDatabase::default();
        let mut keys = Vec::new();
        for release in [Release::JIM_0_80, Release::JIM_0_84] {
            let point = DialectPoint::canonical(release);
            let profile =
                tcl_dialect::DialectProfile::projected_from_point("jim", &[], "Jim", point)
                    .intern();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let key = ProcBodyKey::new(
                &db,
                "return DONE".to_owned(),
                "::".to_owned(),
                "jim".to_owned(),
                CompilerMemoSnapshot {
                    profile: Some(profile.cache_key()),
                    registry: registry.snapshot(),
                },
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            );
            assert_eq!(
                key.snapshot(&db).profile.unwrap().profile().core_point,
                Some(point)
            );
            assert_eq!(
                key.snapshot(&db)
                    .registry
                    .registry()
                    .profile()
                    .unwrap()
                    .core_point,
                Some(point)
            );
            let body = lower_proc_body(&db, key);
            let dialect = body
                .statements
                .iter()
                .filter_map(|statement| statement.tokens())
                .filter_map(|tokens| tokens.source_binding.as_ref())
                .find_map(|binding| binding.variable_context.invocation_dialect)
                .unwrap();
            assert_eq!(dialect.core_point, Some(point));
            keys.push(key);
        }
        assert!(
            keys[0] != keys[1],
            "distinct selected profiles must intern different body keys"
        );
    }

    #[test]
    fn body_memo_registry_identity_preserves_authored_overrides_and_pins() {
        let db = TclDatabase::default();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry = CommandRegistry::build_default().project_for_profile(profile);
        let make_key = |snapshot| {
            ProcBodyKey::new(
                &db,
                "return DONE".to_owned(),
                "::".to_owned(),
                "tcl8.6".to_owned(),
                CompilerMemoSnapshot {
                    profile: Some(profile.cache_key()),
                    registry: snapshot,
                },
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
        };
        let original = make_key(registry.snapshot());
        registry.insert_ambient_package("MemoPackage", "1.0");
        let changed = make_key(registry.snapshot());
        assert!(
            original != changed,
            "ambient pins participate in memo equality"
        );
        assert!(
            !original
                .snapshot(&db)
                .registry
                .registry()
                .ambient_package_rows()
                .contains(&("MemoPackage", "1.0"))
        );
        assert!(
            changed
                .snapshot(&db)
                .registry
                .registry()
                .ambient_package_rows()
                .contains(&("MemoPackage", "1.0"))
        );
        let mut spec = registry.get("set").unwrap().clone();
        spec.traits = tcl_registry::Traits::PURE;
        registry.insert(spec);
        let authored = make_key(registry.snapshot());
        assert!(
            changed != authored,
            "authored overrides participate in memo equality"
        );
        assert_eq!(
            authored
                .snapshot(&db)
                .registry
                .registry()
                .get("set")
                .unwrap()
                .traits,
            tcl_registry::Traits::PURE
        );
    }
    #[test]
    fn original_callback_alias_diagnostics_keep_target_and_separate_counts() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let db = TclDatabase::default();
        let source = "proc target {a b c} {}\ninterp alias {} cb {} target FIXED\nlsort -command {cb BAKED} {2 1}";
        let file = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![file]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let mut invocations = analysis.command_invocations;
        for row in &mut invocations {
            if row
                .original_callback_signature_lookup
                .as_ref()
                .is_some_and(|selection| {
                    selection.prefix().appended_arity()
                        == Some(tcl_registry::AppendedArity::Exactly(2))
                })
            {
                row.original_callback_prefix = None;
                row.original_lookup = None;
                row.original_name_input = None;
                row.name = "unrelated presentation".to_owned();
                row.range = tcl_lexer::Span::new(0, 1);
                row.callback_arity = Some(tcl_registry::AppendedArity::Unknown);
                row.callback_baked_args = usize::MAX;
            }
        }
        let mut output = Vec::new();
        apply_original_callback_arity(&db, project, &mut output, &invocations, |_| false);
        let diagnostic = output
            .iter()
            .find(|diagnostic| {
                diagnostic.callback_source_arity().is_some_and(|subject| {
                    subject.prefix().appended_arity()
                        == Some(tcl_registry::AppendedArity::Exactly(2))
                })
            })
            .unwrap();
        assert_eq!(diagnostic.code, DiagCode::E003);
        let subject = diagnostic.callback_source_arity().unwrap();
        assert_eq!(subject.prefix().baked_argument_count(), 1);
        assert_eq!(
            subject
                .source_lookup()
                .unwrap()
                .captured_argument_count()
                .unwrap()
                .minimum,
            1
        );
        assert_eq!(
            subject.argument_counts(),
            &tcl_compiler::analyser::SourceCallbackArgumentCounts::Finite(vec![4])
        );
        assert_eq!(
            subject.declarations()[0].name().slot().simple.as_bytes(),
            b"target"
        );
        assert_eq!(&source[diagnostic.span.as_range()], "cb");
        assert!(
            invocations
                .iter()
                .all(|row| row.original_callback_signature_lookup.is_none()
                    || callback_command_keys(row).is_empty())
        );
    }

    #[test]
    fn original_callback_alias_external_headers_use_held_target_slot() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let db = TclDatabase::default();
        let library = SourceFile::new(
            &db,
            "proc external {a b c d} {}\nproc cb {a b c} {}".to_owned(),
            "tcl9.0".to_owned(),
            None,
        );
        let source = "interp alias {} cb {} external FIXED\nlsort -command cb {2 1}";
        let caller = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![library, caller]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let mut output = Vec::new();
        apply_original_callback_arity(
            &db,
            project,
            &mut output,
            &analysis.command_invocations,
            |_| false,
        );
        let subject = output
            .iter()
            .filter_map(|diagnostic| diagnostic.callback_source_arity())
            .find(|subject| {
                subject.prefix().appended_arity() == Some(tcl_registry::AppendedArity::Exactly(2))
            })
            .unwrap();
        assert_eq!(
            subject.issue(),
            tcl_compiler::analyser::SourceCallbackArityIssue::TooFew {
                supplied: 3,
                expected_minimum: 4
            }
        );
        assert_eq!(subject.declarations().len(), 1);
        assert_eq!(
            subject.declarations()[0].name().slot().simple.as_bytes(),
            b"external"
        );
        assert_eq!(subject.source_lookup().unwrap().original().target().unwrap().kind(), tcl_compiler::command_binding::OriginalSourceCallbackProcedureTargetKind::ExternalSourceName);
    }

    #[test]
    fn original_callback_alias_database_refuses_deleted_and_unowned_horizons() {
        // naming.source.original-callback-procedure-target
        // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
        let db = TclDatabase::default();
        let source = "proc cb {a b c} {}\nrename cb {}\nlsort -command cb {2 1}";
        let file = SourceFile::new(&db, source.to_owned(), "tcl9.0".to_owned(), None);
        let project = Project::new(&db, vec![file]);
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        let mut output = Vec::new();
        apply_original_callback_arity(
            &db,
            project,
            &mut output,
            &analysis.command_invocations,
            |_| false,
        );
        assert!(
            output
                .iter()
                .all(|diagnostic| diagnostic.callback_source_arity().is_none())
        );
        let source = "proc cb {a b c} {}\nlsort -command cb {2 1}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl9.0");
        for row in &mut analysis.command_invocations {
            row.original_callback_signature_lookup = None;
        }
        output.clear();
        apply_original_callback_arity(
            &db,
            project,
            &mut output,
            &analysis.command_invocations,
            |_| false,
        );
        assert!(
            output.is_empty(),
            "lookup geometry without a source horizon cannot borrow a project header"
        );
    }
}

#[cfg(test)]
mod original_project_callback_projection_tests;
