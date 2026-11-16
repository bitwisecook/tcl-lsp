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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use tcl_compiler::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::cfg_builder::{
    OriginalProcedureSummaryOwners, PreparedCfgContext, build_cfg_function_with_prepared_context,
};
#[cfg(test)]
use tcl_compiler::command_binding::ModuleCommandBindings;
use tcl_compiler::compilation_unit::{
    CompilationUnit, FunctionUnit, LatticeRequest, ModuleTraceFacts, UnitBuildOptions,
};
use tcl_compiler::compiler_checks::{DiagCode, Diagnostic as CompilerCheck};
#[cfg(test)]
use tcl_compiler::interprocedural::ProcSummary;
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

fn document_analyser_without_factory_oracle(
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
        .with_script_encoding(config.script_encoding(db))
        .with_file_path(file.path(db).clone())
        .with_workspace_subclass_methods(file.workspace_subclass_methods(db).clone())
        .with_workspace_declared_commands(file.workspace_declared_commands(db).clone())
        .with_workspace_source_modules(file.workspace_source_modules(db).clone())
}

fn document_analyser(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Analyser {
    document_analyser_without_factory_oracle(db, file, config)
        .with_workspace_class_factories(file.workspace_class_factories(db).clone())
}

/// [`document_analysis_input`] for a consumer that walks the source. The input
/// carries the document's workspace modules without comparing on them, so the
/// consumer also depends on the field itself.
pub(crate) fn walked_document_input(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Result<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>, OverlayMiss> {
    let _modules = file.workspace_source_modules(db);
    document_analysis_input(db, file, config)
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
    document_analyser_without_factory_oracle(db, file, config)
        .prepare_analysis_input(file.dialect(db))
        .map(Arc::new)
}

fn unavailable_document_analysis(dialect: &str, miss: OverlayMiss) -> Arc<AnalysisResult> {
    let mut result = AnalysisResult::default();
    dialect.clone_into(&mut result.dialect);
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
    let input = match walked_document_input(db, file, config) {
        Ok(input) => input,
        Err(miss) => return unavailable_document_analysis(file.dialect(db), miss),
    };
    Arc::new(
        document_analyser(db, file, config)
            .with_resolved_input((*input).clone())
            .analyse(file.text(db), file.dialect(db)),
    )
}

/// Complete current source owner for independently retained procedure headers.
/// Declarations supply source correspondence, never loaded implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOriginalProcedureFacts {
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    image: tcl_lexer::SourceImage,
    declarations: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    >,
}

impl FileOriginalProcedureFacts {
    #[must_use]
    pub fn input(&self) -> &tcl_compiler::analyser::ResolvedAnalysisInput {
        &self.input
    }

    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    #[must_use]
    pub fn declarations(
        &self,
    ) -> &[tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ProcDef,
    >] {
        &self.declarations
    }
}

#[salsa::tracked(returns(clone))]
pub fn file_original_procedure_facts(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<FileOriginalProcedureFacts>> {
    let input = walked_document_input(db, file, config).ok()?;
    let analysis = file_analysis(db, file, config);
    let (image, source_config) =
        tcl_compiler::source_graph::current_analysis(file.text(db), &analysis)?;
    if analysis.resolved_input.as_ref() != Some(input.as_ref())
        || source_config != input.lexer_config()
    {
        return None;
    }
    let context = input.context_registry();
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        context.commands(),
        &input,
        source_config,
        Some(input.unit_profile()),
    )?;
    let declarations = analysis
        .original_procedure_declarations()
        .filter(|declaration| {
            declaration.name_input().source_image() == &image
                && declaration.name_input().lexer_config() == source_config
        })
        .cloned()
        .collect();
    Some(Arc::new(FileOriginalProcedureFacts {
        input: (*input).clone(),
        image,
        declarations,
    }))
}

/// Each configured file retains its own source declarations and input owner.
/// Missing mappings remain unavailable beside the project's label roster.
#[salsa::tracked(returns(clone))]
pub fn project_original_procedure_facts(
    db: &dyn TclDb,
    project: Project,
) -> Arc<Vec<(SourceFile, Arc<FileOriginalProcedureFacts>)>> {
    Arc::new(
        project
            .files(db)
            .iter()
            .filter_map(|&file| {
                let config = project_token_inputs::configuration_for_file(db, project, file)?;
                Some((file, file_original_procedure_facts(db, file, config)?))
            })
            .collect(),
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
/// Duplicate qualified names retain no unique provider and withdraw that hint.
#[salsa::tracked(returns(clone))]
pub fn project_class_factories(
    db: &dyn salsa::Database,
    project: Project,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut providers = HashSet::new();
    merge_class_factories(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_class_factories(db, file)),
    )
}

fn merge_class_factories(
    parts: impl IntoIterator<Item = Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
) -> Arc<tcl_compiler::analyser::ClassFactoryIndex> {
    let mut merged = tcl_compiler::analyser::ClassFactoryIndex::new();
    let mut ambiguous = HashSet::new();
    for part in parts {
        for (qname, factory) in part.iter() {
            if ambiguous.contains(qname) {
                continue;
            }
            match merged.entry(qname.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(factory.clone());
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    entry.remove();
                    ambiguous.insert(qname.clone());
                }
            }
        }
    }
    Arc::new(merged)
}

/// The coverage-relevant projection of one `source` site.
///
/// Original ownership remains attached; the derived link query keeps unchanged
/// project associations equal when original source geometry moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSourceTarget {
    /// Genuine conditional current-interpreter path, independent of report text.
    pub original_source_load:
        Option<Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>>,
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
    let result = Analyser::new()
        .structure_only()
        .analyse(file.text(db), file.dialect(db));
    Arc::new(
        result
            .source_targets
            .into_iter()
            .map(|target| FileSourceTarget {
                original_source_load: target.original_project_source_load,
                raw_path: target.raw_path,
                is_literal: target.is_literal,
            })
            .collect(),
    )
}

/// Conditional script-directory source associations for display consumers.
/// An empty compatibility projection does not establish complete source coverage.
/// Dispatch bounds use the checked projection and retain unknown paths.
#[salsa::tracked(returns(clone))]
pub fn file_link_targets(db: &dyn TclDb, file: SourceFile) -> Arc<BTreeSet<String>> {
    checked_file_link_targets(db, file).unwrap_or_default()
}

#[salsa::tracked(returns(clone))]
fn checked_file_link_targets(db: &dyn TclDb, file: SourceFile) -> Option<Arc<BTreeSet<String>>> {
    conditional_source_links(file.path(db).as_deref(), &file_source_targets(db, file)).map(Arc::new)
}

fn conditional_source_links(
    path: Option<&str>,
    targets: &[FileSourceTarget],
) -> Option<BTreeSet<String>> {
    targets
        .iter()
        .map(|target| {
            let load = target.original_source_load.as_ref()?;
            let target = tcl_lsp_core::source_graph::conditional_project_source_target(
                std::path::Path::new(path?),
                load,
            )?;
            Some(target.to_str()?.to_owned())
        })
        .collect()
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
    /// Conditional procedure names in this represented source component.
    /// Missing project membership supplies no bound or actual interpreter proof.
    #[must_use]
    pub fn reach(&self, index: usize) -> Option<&Arc<BTreeSet<String>>> {
        self.procs.get(*self.component_of.get(index)?)
    }
}

/// Decompose the declared project source associations once per revision.
/// Each genuine literal path uses the explicit script-directory policy, without
/// proving actual loads, shared interpreters or runtime working directories.
/// Dynamic/unowned paths and targets outside the represented project withdraw
/// the finite bound. Parameter evidence then retains only unknown callers.
/// Per-file checked associations and declarations retain signature early cutoff.
#[salsa::tracked(returns(clone))]
pub fn project_dispatch_components(
    db: &dyn TclDb,
    project: Project,
) -> Option<Arc<DispatchComponents>> {
    let rows = project
        .files(db)
        .iter()
        .map(|&file| {
            Some((
                file.path(db).clone(),
                checked_file_link_targets(db, file)?,
                file_decls(db, file),
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    merge_dispatch_components(rows)
}

fn merge_dispatch_components(
    rows: impl IntoIterator<Item = (Option<String>, Arc<BTreeSet<String>>, Arc<FileDecls>)>,
) -> Option<Arc<DispatchComponents>> {
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
            let &j = by_path.get(target.as_str())?;
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
    Some(Arc::new(DispatchComponents {
        component_of,
        procs: merged.into_iter().map(Arc::new).collect(),
    }))
}

/// Conditional project procedure names for an unenumerable source dispatch.
/// A missing association bound remains unavailable, including unrepresented loads.
///
/// An index into [`project_dispatch_components`], which does the work; see
/// there for what the component bound means and why it is not computed here.
/// Missing project membership or incomplete source paths leave the bound unavailable.
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
) -> Option<Arc<BTreeSet<String>>> {
    let me = project
        .files(db)
        .iter()
        .position(|member| *member == file)?;
    let components = project_dispatch_components(db, project)?;
    components.reach(me).cloned()
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
    let known: HashSet<String> = project_proc_names(db, project).iter().cloned().collect();
    let Some(reach) = file_dispatch_reach(db, file, project) else {
        let mut unavailable = CallSiteEvidence::default();
        unavailable.record_unenumerable_caller(&known.iter().cloned().collect::<Vec<_>>());
        return Arc::new(unavailable);
    };
    let reach = reach.iter().cloned().collect::<Vec<_>>();
    let config = project_token_inputs::configuration_for_file(db, project, file);
    let own = config.and_then(|config| file_original_procedure_facts(db, file, config));
    let scanned = match config {
        Some(config) => {
            if let Ok(input) = walked_document_input(db, file, config) {
                let declared = declared_command_surface(
                    db,
                    file,
                    input.borrowed_context_registry().commands(),
                );
                let external: Vec<_> = project_original_procedure_facts(db, project)
                    .iter()
                    .filter(|(provider, _)| *provider != file)
                    .filter_map(|(_, facts)| {
                        tcl_compiler::unit_scope::ExternalSourceProcedureFacts::capture(
                            facts.input(),
                            facts.image(),
                            facts.declarations(),
                        )
                    })
                    .collect();
                tcl_compiler::unit_scope::scan_source_call_sites_with_original_source(
                    file.text(db),
                    Some(&declared),
                    &known,
                    &reach,
                    &input,
                    None,
                    &external,
                )
            } else {
                let mut unavailable = CallSiteEvidence::default();
                unavailable.record_unenumerable_caller(&known.iter().cloned().collect::<Vec<_>>());
                unavailable
            }
        }
        None => tcl_compiler::unit_scope::scan_source_call_sites(
            file.text(db),
            db.registry(file.dialect(db)),
            Some(&declared_command_surface(
                db,
                file,
                db.registry(file.dialect(db)),
            )),
            tcl_lsp_core::profile_for_dialect(file.dialect(db)),
            &known,
            &reach,
        ),
    };
    let sites: Vec<_> = own
        .iter()
        .flat_map(|facts| facts.declarations())
        .map(|declaration| declaration.declaration_site().clone())
        .collect();
    Arc::new(scanned.without_original_declarations(&sites))
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
    let all = project_call_site_evidence(db, project);
    let own = project_token_inputs::configuration_for_file(db, project, file)
        .and_then(|config| file_original_procedure_facts(db, file, config));
    let sites: Vec<_> = own
        .iter()
        .flat_map(|facts| facts.declarations())
        .map(|declaration| declaration.declaration_site().clone())
        .collect();
    Arc::new(all.slice_for_original_declarations(&sites))
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
        table.iter(),
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
        table.iter(),
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

#[salsa::tracked(returns(clone))]
fn original_command_arity_for_inputs<'db>(
    db: &'db dyn TclDb,
    project: Project,
    name: OriginalCommandSlot<'db>,
) -> Option<Arc<Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    let headers = original_command_signatures_for_inputs(db, project, name)?;
    let mut arities = if headers.iter().any(|header| header.kind() != ItemKind::Proc) {
        Vec::new()
    } else {
        headers
            .iter()
            .map(|header| proc_arity(header.formal_count_projection()))
            .collect()
    };
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
    original_lookup_command_arities_with_mode(
        (db, project, ProjectSourceInputMode::Standalone),
        lookup,
    )
}

fn original_lookup_command_arities_with_mode(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    lookup: &tcl_compiler::command_binding::OriginalCommandLookup,
) -> Option<Arc<Vec<(usize, usize)>>> {
    let (db, project, mode) = source;
    let mut slots = Vec::new();
    for candidate in lookup.candidates().iter().flatten() {
        if slots.iter().any(|(slot, _)| slot == candidate) {
            continue;
        }
        let key = OriginalCommandSlot::new(db, lookup.policy(), candidate.clone());
        let arities = match mode {
            ProjectSourceInputMode::Standalone => original_command_arity(db, project, key),
            ProjectSourceInputMode::Supplied => original_command_arity_for_inputs(db, project, key),
        };
        if let Some(arities) = arities {
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

/// A Logical source slot carries no Native lookup policy or publication.
#[salsa::interned]
struct LogicalSourceCommandSlot<'db> {
    #[returns(ref)]
    slot: tcl_core_types::ByteCommandSlot,
}

// Each entry is a body-free projection of independently checked source headers.
// Missing command-like header ownership keeps the project inventory unavailable.
fn merge_logical_source_arities(
    signatures: impl IntoIterator<Item = Arc<Vec<ItemSig>>>,
) -> Option<HashMap<tcl_core_types::ByteCommandSlot, Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    let mut entries: HashMap<_, (Vec<_>, bool)> = HashMap::new();
    for signatures in signatures {
        for signature in signatures.iter().filter(|signature| {
            matches!(
                signature.id.kind,
                ItemKind::Proc | ItemKind::Class | ItemKind::Alias | ItemKind::Ensemble
            )
        }) {
            let slot = signature.logical_source_coordinate.as_ref()?;
            let entry = entries.entry(slot.clone()).or_default();
            if signature.id.kind == ItemKind::Proc {
                entry.0.push(proc_arity(signature.formal_count));
            } else {
                entry.1 = true;
            }
        }
    }
    Some(
        entries
            .into_iter()
            .map(|(slot, (arities, non_proc))| (slot, if non_proc { Vec::new() } else { arities }))
            .collect(),
    )
}

type ProjectLogicalSourceArities =
    Option<Arc<HashMap<tcl_core_types::ByteCommandSlot, Vec<(usize, usize)>>>>;

#[salsa::tracked(returns(clone))]
fn project_logical_source_arities(
    db: &dyn TclDb,
    project: Project,
    supplied: bool,
) -> ProjectLogicalSourceArities {
    let mut signatures = Vec::new();
    for &file in project.files(db) {
        signatures.push(if supplied {
            let config = project_token_inputs::configuration_for_file(db, project, file)?;
            project_source_inputs::checked_item_sigs_for_config(db, file, config)?
        } else {
            item_sigs(db, file)
        });
    }
    merge_logical_source_arities(signatures).map(Arc::new)
}

#[salsa::tracked(returns(clone))]
fn logical_source_command_arity<'db>(
    db: &'db dyn TclDb,
    project: Project,
    key: LogicalSourceCommandSlot<'db>,
    supplied: bool,
) -> Option<Arc<Vec<(usize, usize)>>> {
    project_logical_source_arities(db, project, supplied)?
        .get(key.slot(db))
        .cloned()
        .map(Arc::new)
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
/// accepts it. Unlike the direct-call count envelope above, this also
/// handles a genuine gap between disjoint selected proc arities: every
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

#[cfg(test)]
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
    use tcl_compiler::analyser::{
        Diagnostic, DiagnosticSubject, Severity, SourceCallbackArityIssue as Issue,
        SourceCallbackAritySubject,
    };
    let (db, project, mode) = source;
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

/// Standalone project source-header advice for retained original occurrences.
/// Foreign headers use each file's explicit compatibility defaults. Use
/// [`project_diagnostics_for_inputs`] for checked per-file project inputs.
/// Native source lookups keep their independent retained policy/slot purpose;
/// Logical calls require their genuine Document/body frame recipe and complete
/// original argv. Missing ownership and earlier opaque mutations preserve the
/// original diagnostics. Project headers supply no installed lookup or entry.
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

    Arc::new(project_source_diagnostics_for_analysis(
        (db, project, ProjectSourceInputMode::Standalone),
        file.text(db),
        &analysis,
        |code| disabled.iter().any(|disabled| disabled == code),
    ))
}

/// Project source-header advice using every member's checked document input.
/// Missing member configuration or source ownership preserves diagnostics;
/// compatibility defaults never replace an unavailable supplied header.
#[salsa::tracked(returns(clone))]
pub fn project_diagnostics_for_inputs(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
    let analysis = file_analysis_incremental(db, file, config);
    if project_token_inputs::configuration_for_file(db, project, file) != Some(config) {
        return Arc::new(analysis.diagnostics.clone());
    }
    Arc::new(project_source_diagnostics_for_analysis(
        (db, project, ProjectSourceInputMode::Supplied),
        file.text(db),
        &analysis,
        |code| {
            config
                .disabled_diagnostics(db)
                .iter()
                .any(|disabled| disabled == code)
        },
    ))
}

type OriginalSourceAritySites = HashMap<(u32, u32), Option<Arc<Vec<(usize, usize)>>>>;
type LogicalSourceAritySites = BTreeMap<
    (u32, u32),
    (
        tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        Arc<Vec<(usize, usize)>>,
    ),
>;

fn project_source_arity_sites(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    unresolved: &HashSet<(u32, u32)>,
    logical_source: bool,
) -> (OriginalSourceAritySites, LogicalSourceAritySites) {
    let (db, project, mode) = source;
    let original_sites = analysis
        .command_invocations
        .iter()
        .filter(|invocation| {
            !logical_source
                && invocation.original_name_input.is_some()
                && unresolved.contains(&(invocation.range.start(), invocation.range.end()))
        })
        .map(|invocation| {
            (
                (invocation.range.start(), invocation.range.end()),
                invocation.original_lookup.as_ref().and_then(|lookup| {
                    original_lookup_command_arities_with_mode((db, project, mode), lookup)
                }),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut logical_sites = BTreeMap::new();
    for candidate in
        tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
            text, analysis,
        )
        .into_iter()
        .flatten()
    {
        let Some(head) = candidate.original_head() else {
            continue;
        };
        let span = head.span();
        let key = (span.start(), span.end());
        if original_sites.contains_key(&key) {
            // A present original Native source owner is terminal even when it
            // cannot supply a project signature.
            continue;
        }
        let Some(coordinates) = candidate.external_header_coordinates() else {
            continue;
        };
        if candidate.has_earlier_unknown_mutations() || candidate.has_lookup_operand_effects() {
            continue;
        }
        let Some(arities) = coordinates.iter().find_map(|slot| {
            logical_source_command_arity(
                db,
                project,
                LogicalSourceCommandSlot::new(db, slot.clone()),
                matches!(mode, ProjectSourceInputMode::Supplied),
            )
        }) else {
            continue;
        };
        logical_sites.insert(key, (candidate, arities));
    }
    (original_sites, logical_sites)
}

fn retained_project_diagnostics(
    analysis: &AnalysisResult,
    original_sites: &OriginalSourceAritySites,
    logical_sites: &LogicalSourceAritySites,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let mut diagnostics = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.code != DiagCode::W123
                || (!original_sites.contains_key(&(diagnostic.span.start(), diagnostic.span.end()))
                    && !logical_sites
                        .contains_key(&(diagnostic.span.start(), diagnostic.span.end())))
        })
        .cloned()
        .collect::<Vec<_>>();
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
    diagnostics
}

fn project_source_diagnostics_for_analysis(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let (db, project, mode) = source;
    let Some(input) = analysis.resolved_input.as_ref() else {
        return analysis.diagnostics.clone();
    };
    if !analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(text),
        input.lexer_config(),
    ) {
        return analysis.diagnostics.clone();
    }
    // Each original coordinate is demanded independently so unrelated header
    // changes keep the per-slot query's early cutoff.
    let unresolved = analysis
        .unresolved_command_sites
        .iter()
        .map(|(span, _)| (span.start(), span.end()))
        .collect::<std::collections::HashSet<_>>();
    let logical_source = analysis.allows_retained_logical_declaration_advice();
    let (original_sites, logical_sites) = project_source_arity_sites(
        (db, project, mode),
        text,
        analysis,
        &unresolved,
        logical_source,
    );
    let mut diagnostics = retained_project_diagnostics(analysis, &original_sites, &logical_sites);
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
            && !is_disabled(diagnostic.code.as_str())
        {
            diagnostics.push(diagnostic);
        }
    }
    for (candidate, arities) in logical_sites.values() {
        if arities.is_empty() {
            continue;
        }
        let Some(argc) = candidate.argument_count() else {
            continue;
        };
        let Some(head) = candidate.original_head() else {
            continue;
        };
        let Some(name) = tcl_syntax::word_rules::original_static_word_unicode_value(head) else {
            continue;
        };
        let name = String::from_utf8_lossy(&name);
        if let Some(diagnostic) =
            cross_file_arity_diagnostic(&name, head.span(), (argc, Some(argc)), arities)
            && !is_disabled(diagnostic.code.as_str())
        {
            diagnostics.push(diagnostic);
        }
    }
    apply_logical_source_callback_arity(
        (db, project, mode),
        text,
        analysis,
        &mut diagnostics,
        &is_disabled,
    );
    apply_original_callback_arity_with_mode(
        (db, project, mode),
        &mut diagnostics,
        retained_callback_invocations(analysis),
        &is_disabled,
    );
    diagnostics
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
/// callback owner. Logical prefixes require their independently selected source
/// role, frame recipe and exact declaration coordinate; reported names and counts
/// supply no lookup assistance.
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

fn apply_logical_source_callback_arity(
    source: (&dyn TclDb, Project, ProjectSourceInputMode),
    text: &str,
    analysis: &AnalysisResult,
    out: &mut Vec<tcl_compiler::analyser::Diagnostic>,
    is_disabled: impl Fn(&str) -> bool,
) {
    let (db, project, mode) = source;
    let Some(callbacks) =
        tcl_compiler::registry_invocation::source_structure::source_logical_callback_candidates(
            text, analysis,
        )
    else {
        return;
    };
    for callback in callbacks {
        // Earlier opaque mutations retain their unresolved alternatives. A
        // conditional source prefix is not permission to discard that risk.
        if callback.registration().obligations().contains(
            &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
        ) { continue; }
        if callback.has_lookup_operand_effects() {
            continue;
        }
        let appended = callback.appended_arity();
        if !appended.is_checkable() {
            continue;
        }
        let Some(coordinates) = callback.external_header_coordinates() else {
            continue;
        };
        let Some(candidates) = coordinates.iter().find_map(|slot| {
            logical_source_command_arity(
                db,
                project,
                LogicalSourceCommandSlot::new(db, slot.clone()),
                matches!(mode, ProjectSourceInputMode::Supplied),
            )
        }) else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }
        let Some(span) = callback.head().source_span() else {
            continue;
        };
        let baked = callback.baked_argument_count();
        let name = callback.head().value();
        let diagnostic = if let Some(counts) = appended.exact_counts() {
            counts
                .map(|count| baked + usize::from(count))
                .find_map(|count| callback_exact_arity_diagnostic(name, span, count, &candidates))
        } else {
            let minimum = baked + appended.min() as usize;
            let maximum = appended.max().map(|maximum| baked + maximum as usize);
            cross_file_arity_diagnostic(name, span, (minimum, maximum), &candidates)
        };
        if let Some(diagnostic) = diagnostic
            && !is_disabled(diagnostic.code.as_str())
        {
            out.push(diagnostic);
        }
    }
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
    let mut diagnostics = analysis.diagnostics.clone();
    apply_logical_source_callback_arity(
        (db, project, mode),
        source,
        analysis,
        &mut diagnostics,
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
    /// - `.4` — the procedures other documents declare to this one
    ///   ([`SourceFile::workspace_declared_commands`]), so a cross-file call
    ///   inside the body is read as the whole-file walk reads it.
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
        Option<Arc<DeclaredSurface>>,
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
        key.body_env(db).4.clone(),
    ))
}

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
    source: &str,
    options: UnitBuildOptions<'_>,
) -> CompilationUnit {
    build_unit_with_keys(db, source, options).0
}

/// Return the issued body's retained lattice key when its memo does not read
/// another procedure of the module. Module-reading lattices use the current
/// unit's result for checks, rewrites and the taint cascade.
fn memo_key<'db>(
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
fn build_unit_with_keys<'db>(
    db: &'db dyn TclDb,
    source: &str,
    options: UnitBuildOptions<'_>,
) -> (
    CompilationUnit,
    HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
) {
    build_unit_with_keys_and_input(db, source, options, None, None)
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

fn build_unit_with_keys_and_input<'db>(
    db: &'db dyn TclDb,
    source: &str,
    options: UnitBuildOptions<'_>,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
    settings: Option<&Arc<tcl_compiler::analyser::OriginalAnalyserSourceSettings>>,
) -> (
    CompilationUnit,
    HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
) {
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
        let body_memo = |body_text: &str, namespace: &str| {
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
            original_lower_proc_body(db, key)
        };
        let source_body_memo = |request: &tcl_compiler::lowering::SourceBodySyntaxRequest| {
            let key = SourceProcBodyKey::new(db, request.clone());
            lower_proc_body_source(db, key)
        };
        match input.zip(settings) {
            Some(input) => {
                CompilationUnit::build_memoized_with_original_body_lowering_and_source_settings(
                    source,
                    options,
                    &mut lattice_memo,
                    &body_memo,
                    &source_body_memo,
                    None,
                    input,
                )
            }
            None => CompilationUnit::build_memoized_with_original_body_lowering(
                source,
                options,
                &mut lattice_memo,
                &body_memo,
                &source_body_memo,
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

fn lattice_request_key<'db>(
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

fn summary_deps_key<'db>(
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

/// Hashable normalisation of a callee's `ConstantReturn` (`f64` isn't `Hash`/`Eq`)
/// for the interned [`OptDepsKey`].
#[cfg(test)]
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
#[cfg(test)]
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
    pub defined_callees: Vec<tcl_compiler::ir::ProcedureId>,
}

#[cfg(test)]
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

#[cfg(test)]
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

/// Original compilation owner for one body's source optimisations.
#[salsa::interned]
pub struct OptDepsKey<'db> {
    #[returns(ref)]
    pub capsule: tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule,
    /// The actual worker argument; None cannot be replaced by a module label.
    #[returns(copy)]
    pub effective_dialect: Option<tcl_dialect::DialectProfileKey>,
    /// Actual prepared worker context; None is a reporting-only purpose.
    #[returns(ref)]
    pub original_context: Option<tcl_compiler::value_transfer::AnalysisContextKey>,
}

#[salsa::tracked(lru = 512, returns(clone))]
pub fn function_optimisations<'db>(
    db: &'db dyn TclDb,
    deps: OptDepsKey<'db>,
) -> Arc<tcl_compiler::optimiser::OriginalBodyOptimiserReport> {
    let capsule = deps.capsule(db);
    let registry = capsule.registry_snapshot().shared_registry();
    let dialect = deps.effective_dialect(db).map(|profile| profile.profile());
    Arc::new(capsule.original_body_optimiser_report(&registry, dialect))
}

/// Whether `module` carries any whole-module trace fact (execution *or*
/// variable — `Module::traced_commands` / `has_dynamic_trace` /
/// `traced_variables` / `has_dynamic_variable_trace`) or any callback script
/// that writes a variable (`Module::deferred_writes`).
///
/// Whole-unit trace and deferred-write obligations require a whole-unit solve.
fn module_has_trace_facts(module: &tcl_compiler::ir::Module) -> bool {
    !module.traced_commands.is_empty()
        || module.has_dynamic_trace
        || !module.traced_variables.is_empty()
        || module.has_dynamic_variable_trace
        || !module.deferred_writes.is_clear()
}

/// Assemble a document's optimisations from checked per-body memo owners.
/// Each selected body retains its current lattice key and complete capsule;
/// raw optimisations are computed on the authentic full unit, filtered to that
/// body's groups and rebased before whole-module finalisation. Source mutation
/// and trace hazards remain dependencies of an independently certified readonly
/// source analysis. Other world hazards, missing keys, methods, cross-function
/// lattice reads and argument-sensitive folds require the whole-unit solve.
fn solve_optimisations<'db>(
    db: &'db dyn TclDb,
    cu: &CompilationUnit,
    lattice_keys: &HashMap<tcl_compiler::ir::ProcedureId, FnLatticeKey<'db>>,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<Optimisation> {
    let mutations = cu.command_mutations.clone();
    let source_bodies: Vec<_> = cu
        .all_body_function_units()
        .filter_map(|function| {
            let id = function.body_id?;
            let current = cu.function_for_body(Some(id))?;
            (std::ptr::eq(current, function) && cu.original_procedure_for_body(id).is_some())
                .then_some(id)
        })
        .collect();
    let every_proc_keyed = source_bodies.iter().all(|id| lattice_keys.contains_key(id))
        && cu
            .procedures
            .keys()
            .chain(cu.body_lowering_functions().map(|(id, _)| id))
            .all(|id| {
                cu.original_procedure_for_body(*id).is_some()
                    && cu.function_for_body(Some(*id)).is_some()
            });
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
        || has_arg_sensitive_target
        || !every_proc_keyed
        || reads_module
    {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    }

    let retained = Arc::new(cu.clone());
    let capsules: Option<HashMap<_, _>> = source_bodies
        .iter()
        .map(|id| {
            tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::retain(
                Arc::clone(&retained),
                *id,
                registry,
            )
            .map(|capsule| (*id, capsule))
        })
        .collect();
    let Some(capsules) = capsules else {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    };
    // Readonly source certificates retain the actual mutation/trace facts in
    // their dependency value. Reusing that same conservative analysis does not
    // turn unknown source effects into a quiet runtime or entered execution.
    let world_hazards = mutations
        != tcl_compiler::command_binding::ModuleCommandMutations::default()
        || module_has_trace_facts(&cu.ir_module);
    if world_hazards
        && (capsules.is_empty()
            || !capsules.values().all(tcl_compiler::body_analysis_capsule::BodyAnalysisCapsule::has_closed_readonly_source_correspondence))
    {
        return tcl_compiler::optimiser::optimise_unit(cu, registry, dialect_opt);
    }
    let body_capsules: Vec<_> = source_bodies
        .iter()
        .map(|id| capsules[id].clone())
        .collect();
    let mut assembly = tcl_compiler::optimiser::OriginalOptimiserAssembly::for_body_partition(
        cu,
        registry,
        dialect_opt,
        &body_capsules,
    );
    // Each entire memo report (including complete-empty) is consumed once.
    // Actual capsule correspondence owns span/group rebasing; callers do not
    // reconstruct a batch from mutable members or infer missing-body absence.
    for body_id in &source_bodies {
        if !lattice_keys.contains_key(body_id) || cu.original_procedure_for_body(*body_id).is_none()
        {
            continue;
        }
        let capsule = &capsules[body_id];
        let deps = OptDepsKey::new(
            db,
            capsule.clone(),
            dialect_opt.map(tcl_dialect::DialectProfile::cache_key),
            capsule.original_optimiser_analysis_context(registry, dialect_opt),
        );
        let report = function_optimisations(db, deps);
        assembly.append_body(capsule, report.as_ref());
    }

    // This parent-derived top view retains the actual current root witness.
    // The same single raw worker and unchanged whole-module tail are reused.
    let top_unit = top_level_only_unit(&assembly);
    let top_report =
        tcl_compiler::optimiser::optimise_unit_raw_report(&top_unit, registry, dialect_opt);
    assembly.append_top_level(&top_report);
    assembly.finish_report()
}

/// The source producer derives the existing top-level-only analysis view.
/// Its actual root owns the complete body roster, full prepared analyses and
/// effective dialect; a public clone-and-clear cannot supply that receipt.
fn top_level_only_unit(
    assembly: &tcl_compiler::optimiser::OriginalOptimiserAssembly,
) -> CompilationUnit {
    assembly.top_level_unit()
}

/// Interned complete-document lexer policy for the resolved environment.
/// The environment owns every grammar axis and the file BOM boundary; strict
/// quoting and sub-lexing offsets retain their default entry-point values.
/// Supplied document builds compare this policy with their genuine complete
/// analysis input before issuing a compilation unit. Nested bodies receive
/// their separately retained child input and policy.
#[salsa::interned]
pub struct LexerCfgKey<'db> {
    /// The resolved environment id (`tcl8.6`, `f5-irules`, …) whose grammar
    /// this build lexes under.
    #[returns(ref)]
    pub environment: String,
}

impl LexerCfgKey<'_> {
    /// The full complete-document policy selected by this environment key.
    fn to_config(self, db: &dyn TclDb) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig::for_file_grammar(
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

/// The exact shared registry retained by this lattice key's analysis snapshot.
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
        taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
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
fn own_declared_command_surface(db: &dyn TclDb, file: SourceFile) -> Arc<DeclaredSurface> {
    let _sidecar_stubs_epoch = file.sidecar_stubs_epoch(db);
    Arc::new(tcl_compiler::analyser::utils::document_declared_surface(
        file.text(db),
        file.path(db).as_deref(),
        file.dialect(db),
    ))
}

/// [`own_declared_command_surface`] over the workspace procedures the
/// document's `registry` admits
/// ([`DeclaredSurface::workspace_procedures_under`]): a workspace procedure
/// never shadows a shipped or pack command of the same name.
pub fn declared_command_surface(
    db: &dyn TclDb,
    file: SourceFile,
    registry: &CommandRegistry,
) -> Arc<DeclaredSurface> {
    let own = own_declared_command_surface(db, file);
    let Some(workspace) = file.workspace_declared_commands(db) else {
        return own;
    };
    Arc::new(Arc::unwrap_or_clone(own).over_workspace_procedures(Some(workspace), registry))
}

/// [`world_preserving_procedure_stubs`] as the declarations another document
/// adopts. A procedure this document also declares by stub, inline or in its
/// sidecar, travels as that stated declaration; every other one is inferred
/// from its formals ([`DeclaredCommand::inferred_from_procedure`]), which a
/// user pack covering the same command may answer for instead.
///
/// [`DeclaredCommand::inferred_from_procedure`]: tcl_registry::model::DeclaredCommand::inferred_from_procedure
#[salsa::tracked(returns(clone))]
pub fn world_preserving_procedure_declarations(
    db: &dyn TclDb,
    file: SourceFile,
) -> Arc<DeclaredSurface> {
    let rooted = |name: &str| {
        if tcl_syntax::naming::is_absolute(name) {
            name.to_owned()
        } else {
            tcl_syntax::naming::root_unrooted_key(name)
        }
    };
    let inferred = tcl_compiler::analyser::utils::workspace_procedure_declarations(
        &world_preserving_procedure_stubs(db, file).join("\n"),
    );
    let own = own_declared_command_surface(db, file);
    let mut surface = DeclaredSurface::new();
    for (name, command) in inferred.iter() {
        let stated = own
            .iter()
            .find(|(own_name, _)| rooted(own_name) == rooted(name))
            .map(|(_, stated)| {
                let mut stated = stated.clone();
                name.clone_into(&mut stated.name);
                stated.declaration.provenance = command.declaration.provenance;
                stated
            });
        surface.declare(stated.unwrap_or_else(|| command.clone()));
    }
    Arc::new(surface)
}

/// The workspace declarations a document adopts from the other documents'
/// [`world_preserving_procedure_declarations`], merged in `providers` order so
/// a name two documents define resolves the same way every time; `None` when
/// they declare nothing.
pub fn merged_workspace_procedure_declarations(
    db: &dyn TclDb,
    providers: impl IntoIterator<Item = SourceFile>,
) -> Option<Arc<DeclaredSurface>> {
    let mut merged = DeclaredSurface::new();
    for provider in providers {
        for (_, command) in world_preserving_procedure_declarations(db, provider).iter() {
            merged.declare(command.clone());
        }
    }
    (!merged.is_empty()).then(|| Arc::new(merged))
}

/// The procedures this document defines that a call from another document
/// may trust, as sidecar-syntax `stub` lines: every declaration-time walk of
/// the body leaves the command world unchanged. The argument counts follow the
/// document's own parameter grammar
/// ([`tcl_syntax::formal_params::formal_argument_count_shape`]): C Tcl's rest
/// formal is a final `args` and only its trailing defaults are optional; Jim's
/// `args` is a rest formal anywhere and any default is optional. A rest formal
/// has no stub spelling, so such a procedure is not declared, nor is any when
/// the grammar is unknown.
#[salsa::tracked(returns(clone))]
pub fn world_preserving_procedure_stubs(db: &dyn TclDb, file: SourceFile) -> Arc<Vec<String>> {
    let unit = document_compilation_unit(db, file);
    let Some(bindings) = unit.ir_module.retained_source_bindings.as_deref() else {
        return Arc::default();
    };
    let Some(grammar) = unit.ir_module.parameter_grammar() else {
        return Arc::default();
    };
    let mut lines = bindings
        .world_preserving_procedures()
        .filter_map(|(identity, formals)| {
            let shape = tcl_syntax::formal_params::formal_argument_count_shape(
                formals
                    .iter()
                    .map(|formal| (formal.name == "args", formal.default.is_some())),
                grammar,
            );
            shape.maximum?;
            let words = formals
                .iter()
                .enumerate()
                .map(|(index, formal)| {
                    let optional = formal.default.is_some()
                        && (grammar == tcl_dialect::ParameterGrammar::Jim
                            || index >= shape.minimum);
                    if optional {
                        format!("?{}?", formal.name)
                    } else {
                        formal.name.clone()
                    }
                })
                .collect::<Vec<_>>();
            // A global procedure is declared under the name a bare call
            // spells; a namespaced one keeps its qualified name.
            let name = tcl_syntax::naming::unroot_rooted_key(identity)
                .filter(|tail| !tcl_syntax::naming::is_qualified(tail.as_bytes()))
                .unwrap_or(identity);
            Some(format!("stub {name} {{{}}}", words.join(" ")))
        })
        .collect::<Vec<_>>();
    lines.sort_unstable();
    Arc::new(lines)
}

/// The shared, memoised [`CompilationUnit`] for a document under a given lexer
/// config and workspace pack overlay (`0` for none) — built via
/// `memoised_compilation_unit` (per-procedure lattices on the salsa-native
/// [`function_lattice`] graph) against the overlay's registry, so a pack's
/// declarations reach every lattice of the file and a pack edit, which
/// changes the overlay, invalidates them.  Tracked + keyed on
/// `(file, cfg, overlay)` so the analyser tail ([`file_analysis_incremental`]) and the
/// compiler-checks pass share the same complete-document config key when
/// their supplied source inputs agree. Original full-input guards remain
/// independent of memo sharing.
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
    let declared = declared_command_surface(db, file, &registry);
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
    Some(Arc::new(
        build_unit_with_keys_and_input(db, file.text(db), options, Some(&input), Some(&settings)).0,
    ))
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
    let (cu, keys) =
        build_unit_with_keys_and_input(db, file.text(db), options, Some(&input), Some(&settings));
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
    let workspace_declared_commands = file.workspace_declared_commands(db).clone();
    let input = match walked_document_input(db, file, config) {
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
                workspace_declared_commands.clone(),
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
#[derive(Clone)]
pub struct CompilerDiagnostics {
    /// `run_all_checks` output (GVN / shimmer / thunking / taint / iRules-flow /
    /// SCCP), severities preserved.
    pub checks: Vec<CompilerCheck>,
    /// `optimise_unit` rewrites (`O1xx`), surfaced as HINT-severity suggestions.
    pub optimisations: Vec<Optimisation>,
    /// Command-head replay availability for compiler checks, independent of warning count.
    /// Stored callbacks belong to the separate whole-unit taint report.
    pub taint_replays: tcl_compiler::taint::TaintReplayAnalysis,
}

impl PartialEq for CompilerDiagnostics {
    fn eq(&self, other: &Self) -> bool {
        self.checks == other.checks
            && self.optimisations == other.optimisations
            && self.taint_replays == other.taint_replays
            && same_optimisation_provenance(&self.optimisations, &other.optimisations)
    }
}

// Salsa must not retain an old source-action receipt when report text agrees.
// Optimisation's public payload equality remains independent of that currency.
fn same_optimisation_provenance(left: &[Optimisation], right: &[Optimisation]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.same_original_rewrite_provenance(right))
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
    let report = tcl_compiler::compiler_checks::compiler_check_report_with_generic_patterns(
        cu,
        registry,
        dialect,
        generic_patterns,
    );
    CompilerDiagnostics {
        checks: report.diagnostics,
        optimisations: tcl_compiler::optimiser::optimise_unit(cu, registry, dialect),
        taint_replays: report.taint_replays,
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
    compiler_check_diagnostics_if_available(db, file, config).unwrap_or_else(abstain_diagnostics)
}

/// The actual compiler diagnostic pass, preserving unavailable preparation.
/// `Some` follows successful checked unit preparation and the real solve/check
/// producer. `None` carries no completed empty report. This supplies no Native
/// execution, normal completion or closure of unavailable nested analyses.
#[salsa::tracked(lru = 64, returns(clone))]
pub fn compiler_check_diagnostics_if_available(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Option<Arc<CompilerDiagnostics>> {
    let dialect = file.dialect(db).clone();
    let Ok(input) = walked_document_input(db, file, config) else {
        return None;
    };
    let registry: &CommandRegistry = input.borrowed_context_registry().commands();
    let dialect_opt = Some(input.unit_profile());
    // The analyser tail and compiler checks share the complete document
    // policy key. The supplied input must still pass its exact guard.
    let cfg_key = lexer_cfg_key(db, &dialect);
    let _shared_unit = compilation_unit_for_config(db, file, cfg_key, config)?;
    // Both halves of `run_all_checks` come from the memoised [`proc_taint_solve`]:
    // the interprocedural taint solve (`solve.taints`)
    // and the per-procedure non-taint checks (`solve.fn_checks`, already rebased),
    // so an unchanged procedure contributes neither a re-solve nor a re-check.
    // The remaining taint-family + iRules module checks (which read the solved
    // taints) are appended over the solve's retained unit, then the set is sorted
    // into the same deterministic order `run_all_checks` produces.  Byte-identical
    // to the in-line build; guarded by the corpus differential.  Optimiser
    // unchanged.
    let solve = proc_taint_solve_for_config(db, file, cfg_key, config)?;
    // Procedure IDs belong to this solve's genuine construction. A separate
    // shared build retains its own source ingress and cannot select these maps.
    let cu = solve.unit.as_ref();
    let mut checks = solve.fn_checks.clone();
    let generic_patterns = config.generic_variable_patterns(db).as_deref();
    let taint_replays = tcl_compiler::compiler_checks::push_taint_and_module_checks(
        cu,
        registry,
        dialect_opt,
        &solve.taints,
        generic_patterns,
        &mut checks,
    );
    tcl_compiler::compiler_checks::retain_diagnostic_source_context(cu, registry, &mut checks);
    tcl_compiler::compiler_checks::sort_diagnostics(&mut checks);
    Some(Arc::new(CompilerDiagnostics {
        checks,
        optimisations: solve.optimisations.clone(),
        taint_replays,
    }))
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
    let cu = fresh_compiler_document_unit(text, registry, dialect, external_call_sites);
    compiler_diagnostics_from_unit(
        &cu,
        registry,
        tcl_lsp_core::stated_profile_for_dialect(dialect),
        generic_patterns,
    )
}

/// Fresh document source ingress over the exact caller-selected command store.
/// This driver resolves its own complete editing input; supplied-owner callers
/// use their independent checked route and cannot reopen this constructor.
fn fresh_compiler_document_unit(
    text: &str,
    registry: &CommandRegistry,
    dialect: &str,
    external_call_sites: Option<&CallSiteEvidence>,
) -> CompilationUnit {
    let environment = tcl_registry::model::ingress::resolve_environment(dialect);
    let baseline = environment.default_context_registry();
    let context =
        if baseline.commands().snapshot().semantic_key() == registry.snapshot().semantic_key() {
            baseline
        } else {
            Arc::new(baseline.with_command_store(registry.snapshot().shared_registry()))
        };
    let config = tcl_lexer::LexerConfig::for_file_grammar(environment.grammar());
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        context,
        config,
    );
    let declared = tcl_compiler::analyser::utils::document_declared_surface(text, None, dialect);
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        config,
        Some(input.unit_profile()),
    );
    CompilationUnit::build_with_analysis_input(
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
        &input,
    )
    .with_interprocedural(registry, Some(input.unit_profile()))
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
    if let Some(unit) = analysis.original_optimiser_source_unit(text, registry, external_call_sites)
    {
        return Some(unit);
    }
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
            taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
        };
    };
    let profile = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::unit_profile);
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
pub use project_source_inputs::{
    item_sigs_for_config, item_tree_for_config,
    project_callback_diagnostics_for_analysis_with_inputs, project_callback_diagnostics_for_inputs,
    project_class_factories_for_inputs, project_factory_header_census_for_inputs,
    project_original_command_signatures_for_inputs,
};
use project_source_inputs::{
    original_command_signatures_for_inputs, original_lookup_command_signatures_for_inputs,
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
/// The structure-only per-file boundary retains original source-display
/// receipts as well as compatibility projections. Original-image changes
/// invalidate those receipts; unchanged aggregate display maps can backdate
/// independently after their original providers have been checked.
#[derive(Clone, Default, PartialEq)]
pub struct FileTokenFacts {
    /// Classes declared in this file, keyed by qualified name.
    pub classes: HashMap<String, ClassDef>,
    /// The file's user-proc parameter roles.
    pub proc_roles: VarNameArgRoles,
    /// Readonly named-instance reports with separate existing compatibility
    /// and conditional source-display modes. Conditional labels supply no
    /// receiver identity; actual project token selection uses `class_cards`.
    pub named_instances: tcl_lsp_core::semantic_tokens::NamedInstanceMap,
    /// Genuine original class/factory/instance source-display owners.
    pub class_cards:
        Option<tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards>,
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
    let mut named_instances =
        tcl_lsp_core::semantic_tokens::NamedInstanceMap::from_analysis(&result);
    let class_cards =
        tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards::capture(
            &result,
        );
    if let Some(cards) = &class_cards {
        named_instances.extend_conditional_source_cards(cards);
    }
    Arc::new(FileTokenFacts {
        proc_roles: VarNameArgRoles::from_analysis(&result),
        class_cards,
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
    let mut providers = HashSet::new();
    merge_project_classes(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
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
    let mut providers = HashSet::new();
    merge_project_proc_roles(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
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

/// Workspace named-instance display reports, preserving each row's existing
/// compatibility or conditional source-display mode. Conditional labels never
/// enter legacy classification; actual project receiver selection uses the
/// independently retained original `FileTokenFacts::class_cards`.
///
/// Conflicting class labels are omitted from this report in either file order.
/// Agreement between labels supplies no command or receiver identity.
///
/// Reads the same light [`file_token_facts`] firewall as [`project_class_index`].
#[salsa::tracked(returns(clone))]
pub fn project_named_instance_index(
    db: &dyn TclDb,
    project: Project,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    let mut providers = HashSet::new();
    merge_project_named_instances(
        project
            .files(db)
            .iter()
            .filter(|&&file| providers.insert(file))
            .map(|&file| file_token_facts(db, file)),
    )
}

fn merge_project_named_instances(
    facts: impl IntoIterator<Item = Arc<FileTokenFacts>>,
) -> Arc<tcl_lsp_core::semantic_tokens::NamedInstanceMap> {
    let mut merged = tcl_lsp_core::semantic_tokens::NamedInstanceMap::default();
    for facts in facts {
        merged.merge_reports(&facts.named_instances);
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
    let mut seen = HashSet::new();
    let class_cards = project
        .files(db)
        .iter()
        .filter(|&&member| member != file && seen.insert(member))
        .map(|&file| file_token_facts(db, file).class_cards.clone())
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
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
            project_classes: Some(&class_cards),
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
mod tests;

#[cfg(test)]
mod compiler_snapshot_memo_tests;

#[cfg(test)]
mod original_project_callback_projection_tests;

#[cfg(test)]
mod readonly_source_optimisation_owner_tests;

#[cfg(test)]
mod original_optimiser_memo_transport_tests;
