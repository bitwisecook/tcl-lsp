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
//! Foundational phase: a single memoised query graph replaces the server's
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

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use tcl_compiler::cfg_builder::build_cfg_function_with_upvars;
use tcl_compiler::cfg_builder::global_write_info::GlobalWriteInfo;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::compilation_unit::{
    CompilationUnit, FunctionUnit, LatticeRequest, ModuleTraceFacts,
};
use tcl_compiler::compiler_checks::{DiagCode, Diagnostic as CompilerCheck};
use tcl_compiler::interprocedural::{InterproceduralAnalysis, ProcSummary};
use tcl_compiler::ir::Script;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::ssa::ValueKey;
use tcl_compiler::taint::TaintLattice;
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
use tcl_dialect::DialectSet;
use tcl_lsp_core::document_symbols::DocumentSymbol;
use tcl_lsp_core::folding::FoldingRange;
use tcl_lsp_core::semantic_tokens::{SemanticTokens, VarNameArgRoles};
use tcl_registry::CommandRegistry;

/// Database trait exposing the durable (non-salsa) command registry to
/// tracked queries.
#[salsa::db]
pub trait TclDb: salsa::Database {
    /// The dialect-loaded command registry (built once per canonical dialect
    /// key, then shared).  Immutable for the process lifetime.
    fn registry(&self, dialect: &str) -> Arc<CommandRegistry>;
}

/// The Tcl LSP query database.
///
/// Cloneable so a worker thread can run queries against a handle while the
/// main thread sets inputs (the rust-analyzer snapshot pattern).  The
/// `registries` map is shared across clones (it is a process-wide static
/// cache, not per-snapshot state).
#[salsa::db]
#[derive(Default, Clone)]
pub struct TclDatabase {
    storage: salsa::Storage<Self>,
    registries: Arc<Mutex<HashMap<String, Arc<CommandRegistry>>>>,
}

#[salsa::db]
impl salsa::Database for TclDatabase {}

impl TclDatabase {
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
        Self {
            storage,
            registries: Arc::default(),
        }
    }
}

#[salsa::db]
impl TclDb for TclDatabase {
    fn registry(&self, dialect: &str) -> Arc<CommandRegistry> {
        // Canonical key: parseable dialects keep their string; unparseable /
        // plain-Tcl collapse to "" (one shared base registry).  Mirrors the
        // server's former `registry_for_dialect`.
        let parsed = DialectSet::parse(dialect);
        let key = if parsed.is_some() { dialect } else { "" };
        let mut map = self.registries.lock().expect("registry cache poisoned");
        if let Some(r) = map.get(key) {
            return Arc::clone(r);
        }
        let mut registry = CommandRegistry::build_default();
        if let Some(d) = parsed {
            registry.load_dialect(d);
        }
        let arc = Arc::new(registry);
        map.insert(key.to_owned(), Arc::clone(&arc));
        arc
    }
}

/// A source document: text plus the dialect it is analysed under.
///
/// `set_text` (generated) is the single write on an edit — salsa cascades
/// invalidation to every query that read it.
#[salsa::input]
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
}

/// Analyser configuration mirrored from the editor (the former
/// `disabled_diagnostics` / `non_ascii_mode` server state).  One input
/// instance shared by every file's analysis; setting it recomputes all
/// analyses.
#[salsa::input]
pub struct AnalyserConfig {
    #[returns(ref)]
    pub disabled_diagnostics: Vec<String>,
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
}

/// Whole-file analysis, behind an `Arc` so reads bump a refcount rather than
/// deep-clone.
///
/// Wraps [`Analyser::analyse`] unchanged, uncancellable and with no per-item
/// memoisation.  Every production feature provider now reads
/// [`file_analysis_incremental`] instead (issue #829: this coarse query has no
/// interior salsa cancellation checkpoint, so a caller holding a read blocks a
/// concurrent edit's `set_text` until the whole walk finishes). `file_analysis`
/// itself stays live as the differential-fuzzer / corpus-gate ground truth
/// [`file_analysis_incremental`] is proven byte-identical against.
#[salsa::tracked]
pub fn file_analysis(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let disabled: HashSet<String> = config.disabled_diagnostics(db).iter().cloned().collect();
    let extra: HashSet<String> = config.extra_commands(db).iter().cloned().collect();
    let mut analyser = Analyser::with_disabled_diagnostics(disabled)
        .with_non_ascii_mode(config.non_ascii_mode(db))
        .with_extra_commands(extra)
        .with_file_path(file.path(db).clone());
    Arc::new(analyser.analyse(file.text(db), file.dialect(db)))
}

/// Offset-stable item tree — the per-item firewall's foundation (slice 1 of
/// `docs/design/rust/incremental-analysis.md`). One item per declaration, keyed
/// by stable name + kind so a shifted-but-unedited proc keeps its identity.
///
/// **Slice-1 anchor.** `ensemble_namespaces` lives on the `Analyser`, not the
/// returned `AnalysisResult`, so this query runs `analyse` directly and reads
/// the ensemble set off the instance rather than reusing [`file_analysis`]. The
/// item set therefore *cannot* diverge from `analyse`. Slices 2–3 re-home this
/// onto a cheap, independent CST extractor — guarded by the `file_decls` corpus
/// gate + the `incremental == fresh` differential fuzzer + the full-rebuild
/// fallback (item detection is config-independent, hence no `AnalyserConfig`).
#[salsa::tracked]
pub fn item_tree(db: &dyn salsa::Database, file: SourceFile) -> Arc<ItemTree> {
    // `structure_only` skips diagnostic emission (the dominant analyse cost)
    // while building the identical declaration/scope structure — a cheap,
    // non-divergent item extractor (gated by `file_decls_corpus`).
    let mut analyser = Analyser::new().structure_only();
    let result = analyser.analyse(file.text(db), file.dialect(db));
    Arc::new(ItemTree::from_analysis(
        &result,
        &analyser.ensemble_namespaces,
    ))
}

/// Item signatures — the cross-item-relevant headers, with bodies stripped
/// (`item_sig*` in the design graph). A body-only edit leaves these equal, so
/// [`file_decls`] and the future cross-item passes early-cutoff.
#[salsa::tracked]
pub fn item_sigs(db: &dyn salsa::Database, file: SourceFile) -> Arc<Vec<ItemSig>> {
    Arc::new(item_tree(db, file).sigs())
}

/// Aggregate declaration sets (`file_decls ← item_sig*`): the set of declared
/// procs / classes / aliases / ensembles + the namespace tree the cross-item
/// passes (W123 / arity) read read-only.
#[salsa::tracked]
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
#[salsa::input]
pub struct Project {
    /// The project's files (workspace + open documents).
    #[returns(ref)]
    pub files: Vec<SourceFile>,
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
#[salsa::tracked]
pub fn project_proc_names(db: &dyn salsa::Database, project: Project) -> Arc<BTreeSet<String>> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for &file in project.files(db) {
        names.extend(file_decls(db, file).procs.iter().cloned());
    }
    Arc::new(names)
}

/// Extract the unknown-command name from a W123 message
/// (`"Unknown command 'NAME'"`, optionally `+ "; did you mean 'X'?"`) — the first
/// single-quoted token, which is the bare name the analyser failed to resolve.
fn w123_command(message: &str) -> Option<&str> {
    let start = message.find('\'')? + 1;
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

/// Inclusive `(min, max)` argument-arity of a proc from its parameter list
/// (`max == usize::MAX` ⇒ a trailing `args` makes it unbounded).
///
/// Delegates to [`tcl_compiler::signature_scan::arity::arity_of`], the
/// single canonical computation shared with the same-file/TclOO-method
/// arity checks — Tcl's argument binding is strictly positional, so the
/// minimum is the position of the *last* required (non-default) parameter,
/// not a count of required parameters (a required parameter after a
/// defaulted one raises the minimum past the defaulted ones, since a
/// caller cannot supply a later position without also supplying every
/// position before it).
fn proc_arity(params: &[tcl_compiler::signature_scan::types::ParamDef]) -> (usize, usize) {
    let arity = tcl_compiler::signature_scan::arity::arity_of(params);
    let max = if arity.is_unlimited() {
        usize::MAX
    } else {
        usize::from(arity.max)
    };
    (usize::from(arity.min), max)
}

/// The project's cross-file command-resolution domain, keyed by **tail** name,
/// each carrying the `(min, max)` arities of any **procs** with that tail.
/// A bare command `foo` is resolved cross-file if some
/// project declaration has tail `foo` — procs, classes (the class command),
/// `interp alias`es, and ensembles, matching the analyser's *local* suppression
/// domains (`proc_tail_names` / `class_tail_names` / `alias_names` /
/// `ensemble_cmds`).  Non-proc kinds carry an **empty** arity list (resolved, but
/// no arg-count signature), so they suppress W123 without ever drawing an arity error.
///
/// **Mixed tails are arity-less.**  If a tail is claimed by *both* a proc and a
/// non-proc command (e.g. `oo::class create Widget` plus `proc ns::Widget`), a
/// call to it may dispatch to the class/alias/ensemble — which has no fixed
/// arity — so the proc arities are dropped (empty list): the tail still suppresses
/// W123 but never draws a (possibly wrong) arity error.
///
/// Depends only on each file's `item_sigs` (the signature firewall), so a body
/// edit anywhere recomputes nothing here.
#[salsa::tracked]
pub fn project_command_arities(
    db: &dyn TclDb,
    project: Project,
) -> Arc<HashMap<String, Vec<(usize, usize)>>> {
    use tcl_compiler::analyser::ItemKind;
    // tail -> (proc arities, has a non-proc command claiming this tail).
    let mut acc: HashMap<String, (Vec<(usize, usize)>, bool)> = HashMap::new();
    for &file in project.files(db) {
        for sig in item_sigs(db, file).iter() {
            // Command-resolvable kinds only — methods are object-dispatched and
            // namespaces aren't commands, so neither suppresses a bare-command W123.
            let resolvable = matches!(
                sig.id.kind,
                ItemKind::Proc | ItemKind::Class | ItemKind::Alias | ItemKind::Ensemble
            );
            if resolvable
                && let Some((_, tail)) = sig.id.key.rsplit_once("::")
                && !tail.is_empty()
            {
                let entry = acc.entry(tail.to_owned()).or_default();
                if sig.id.kind == ItemKind::Proc {
                    entry.0.push(proc_arity(&sig.params));
                } else {
                    entry.1 = true;
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

/// Interned identity of a single command **tail** name — the key for the
/// per-symbol cross-file resolution accessor [`command_arity`].
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
/// projects out **one** tail: `Some(arities)` when the workspace resolves a
/// command with this tail (`arities` empty ⇒ resolved by a non-proc — class /
/// alias / ensemble — so it suppresses W123 but draws no arity error); `None`
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
#[salsa::tracked]
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
        code,
        span,
        message,
        severity: Severity::Error,
        fixes: Vec::new(),
    })
}

/// Resolve a file's diagnostics against the project: a W123 (unknown
/// command) whose command tail is a workspace proc is **suppressed** (resolved
/// cross-file); if that call's arg count is known and fits **none** of the
/// proc's arities, a cross-file arity error (`E002`/`E003`, the analyser's own
/// codes) replaces it.  Pure; shared by the push ([`project_diagnostics`]) and
/// pull paths so both agree.  `arities` empty ⇒ no project context ⇒ status-quo
/// diagnostics.
///
/// `unresolved_sites` are the call sites of unknown commands
/// ([`AnalysisResult::unresolved_command_sites`]), recorded by the analyser
/// **regardless of whether W123 is disabled** — so cross-file arity is independent
/// of the W123 toggle (matching local arity), since the arity check keys off these
/// rather than the (possibly filtered) W123 diagnostic.
///
/// `is_disabled` honours the user's `disabled_diagnostics` for the synthesized
/// arity code: it is produced *after* the analyser applied its own
/// [`apply_disabled_diagnostics`](tcl_compiler::analyser::Analyser) filter (and the
/// LSP lift does not re-filter), so the filter must be replicated here.
#[must_use]
pub fn apply_cross_file_resolution<S: std::hash::BuildHasher>(
    diags: &[tcl_compiler::analyser::types::Diagnostic],
    unresolved_sites: &[(tcl_lexer::Span, String)],
    invocations: &[tcl_compiler::signature_scan::types::SignatureCommandInvocation],
    arities: &HashMap<String, Vec<(usize, usize)>, S>,
    is_disabled: impl Fn(&str) -> bool,
) -> Vec<tcl_compiler::analyser::types::Diagnostic> {
    if arities.is_empty() {
        return diags.to_vec();
    }
    // Suppress every W123 that resolves cross-file (its tail is a workspace
    // command); a genuinely-unknown command's W123 is kept.
    let mut out: Vec<tcl_compiler::analyser::types::Diagnostic> = diags
        .iter()
        .filter(|d| {
            d.code != DiagCode::W123
                || !w123_command(&d.message).is_some_and(|name| arities.contains_key(name))
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

/// Validate command-prefix **callback** arity: for each recorded callback head
/// (`lsort -command myCompare` → `myCompare` with `callback_arity =
/// Exactly(2)`), check that the referenced project proc accepts the arguments
/// the calling command appends.
///
/// The effective arg count is a RANGE — `baked` args already in the prefix
/// (`{myCmp extra}` bakes one; a bare word bakes zero) plus the command's
/// appended arity (`Exactly(n)` ⇒ `(n, Some(n))`, `AtLeast(n)` ⇒ `(n, None)`).
/// Skipped for `Unknown`/absent arities and for tails the project resolves
/// with a non-proc (empty candidate list), reusing the same E002/E003 codes,
/// disable filter, and message shape as the direct-call cross-file check.
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
        // Tail-resolve the callback head against the project arity table.
        let tail = inv.name.rsplit("::").next().unwrap_or(&inv.name);
        let Some(candidates) = arities.get(tail) else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }
        // Baked args already present in the prefix (0 for a bareword head,
        // N for a braced multi-word prefix like `-command {cb a b}`).
        let baked = inv.callback_baked_args;
        let lo = baked + appended.min() as usize;
        let hi = appended.max().map(|m| baked + m as usize);
        if let Some(diag) = cross_file_arity_diagnostic(&inv.name, inv.range, (lo, hi), candidates)
            && !is_disabled(diag.code.as_str())
        {
            out.push(diag);
        }
    }
}

/// Analyser diagnostics for `file` resolved against the project:
/// cross-file-resolvable W123 (unknown command) suppressed, plus a
/// cross-file arity error (`E002`/`E003`) for calls to workspace procs with a bad
/// arg count.
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
#[salsa::tracked]
pub fn project_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> Arc<Vec<tcl_compiler::analyser::types::Diagnostic>> {
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
    let mut tails: BTreeSet<&str> = BTreeSet::new();
    for diag in &analysis.diagnostics {
        if diag.code == DiagCode::W123
            && let Some(name) = w123_command(&diag.message)
        {
            tails.insert(name);
        }
    }
    for (_, name) in &analysis.unresolved_command_sites {
        tails.insert(name.as_str());
    }
    // Command-prefix callback heads (`lsort -command myCompare`) resolve (they
    // are not W123/unresolved), so their target proc's arity would not be
    // loaded — pull each callback tail in so `apply_callback_arity` can check it.
    for inv in &analysis.command_invocations {
        if inv.callback_arity.is_some() {
            tails.insert(inv.name.rsplit("::").next().unwrap_or(&inv.name));
        }
    }
    let mut arities: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    for tail in tails {
        if let Some(resolved) = command_arity(db, project, CommandTail::new(db, tail.to_owned())) {
            arities.insert(tail.to_owned(), (*resolved).clone());
        }
    }

    Arc::new(apply_cross_file_resolution(
        &analysis.diagnostics,
        &analysis.unresolved_command_sites,
        &analysis.command_invocations,
        &arities,
        |code| disabled.iter().any(|c| c == code),
    ))
}

/// Interned identity of a single `proc` body's isolated analysis — the per-item
/// firewall's memoisation key.  **Offset-invariant**: it holds only what the
/// offset-0 analysis consumes (body text + enclosing namespace / name / params +
/// config), *not* the body's position — so a shifted-but-unedited proc has the
/// same key and reuses the cached [`item_body_analysis`] (the aggregator rebases
/// the offset-0 facts by the body's real span).
#[salsa::interned]
pub struct ItemBodyKey<'db> {
    #[returns(ref)]
    pub body_text: String,
    #[returns(ref)]
    pub namespace: String,
    #[returns(ref)]
    pub scope_name: String,
    #[returns(ref)]
    pub params: Vec<ParamDef>,
    /// `true` for a `TclOO` method body (isolated in a `Method` scope with
    /// instance variables pre-bound); `false` for a `proc`.
    pub is_method: bool,
    /// Mirrors `Scope::oo_global_resolution`: `true` for a TclOO method
    /// body (bare commands resolve globally — object-namespace semantics),
    /// `false` for procs and snit / itcl members.
    pub oo_global_resolution: bool,
    /// Class instance variables pre-bound in a method body (empty for procs).
    #[returns(ref)]
    pub class_variables: Vec<String>,
    #[returns(ref)]
    pub dialect: String,
    #[returns(ref)]
    pub disabled: Vec<String>,
    pub non_ascii: NonAsciiMode,
}

/// Memoised offset-0 isolated analysis of one `proc` body.  A body-only edit
/// changes only that body's [`ItemBodyKey`], so salsa reuses every other body's
/// result; an edit that merely *shifts* a body leaves its key unchanged.
#[salsa::tracked]
pub fn item_body_analysis<'db>(db: &'db dyn TclDb, key: ItemBodyKey<'db>) -> Arc<BodyFragment> {
    // The isolated analysis works at offset 0 and ignores `body_tok` / scope
    // path (the aggregator supplies the real position when grafting), so a
    // placeholder token is fine.
    let body = DeferredBody {
        body_text: key.body_text(db).clone(),
        body_tok: tcl_lexer::Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, 0)),
        scope_path: Vec::new(),
        is_method: key.is_method(db),
        oo_global_resolution: key.oo_global_resolution(db),
        namespace: key.namespace(db).clone(),
        scope_name: key.scope_name(db).clone(),
        params: key.params(db).clone(),
        class_variables: key.class_variables(db).clone(),
    };
    let disabled: HashSet<String> = key.disabled(db).iter().cloned().collect();
    let overlay = tcl_compiler::analyser::types::build_stub_overlay(&[]);
    Arc::new(analyse_proc_body_isolated(
        &body,
        key.dialect(db),
        &disabled,
        key.non_ascii(db),
        Some(overlay),
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
}

/// Interned identity of one procedure's **offset-0** baseline lattice
/// (salsa-native lattice graph).  Holds the procedure's post-inline IR body
/// normalised to offset 0 plus the CFG-determining module [`CfgContext`] +
/// params + dialect — *not* its position — so a shifted-but-unchanged body
/// interns to the same key and reuses the cached [`function_lattice`] (the
/// builder rebases the result to the body's span).  Procedures with
/// interprocedural `param_constants` (caller-uniform-literal SCCP seeds) are
/// memoised too: the encoded seeds are part of the key, so such a procedure
/// reuses its cached lattice across edits and rebuilds only when a caller's
/// literal at that position changes (which re-interns to a new key).  The seeds
/// are position-independent (keyed by parameter name + SSA version), so they do
/// not break the offset-invariance of the body key.
#[salsa::interned]
pub struct FnLatticeKey<'db> {
    #[returns(ref)]
    pub body: Script,
    #[returns(ref)]
    pub qname: String,
    #[returns(ref)]
    pub params: Vec<String>,
    pub context: CfgContext<'db>,
    #[returns(ref)]
    pub dialect: String,
    /// Encoded interprocedural SCCP seeds (`(param, version, string)`, sorted);
    /// empty means none.  Decoded by
    /// [`tcl_compiler::compilation_unit::decode_param_constants`] in [`function_lattice`].
    #[returns(ref)]
    pub param_constants: Vec<(String, u32, String)>,
    /// Fully-qualified names of every class in the compilation unit (sorted) —
    /// a whole-unit fact identical for every procedure, folded into the key so
    /// adding/removing a class anywhere invalidates each procedure's lattice (a
    /// new class can change a body's constructor typing).  Threaded into the
    /// type-propagation pass in [`function_lattice`].
    #[returns(ref)]
    pub known_classes: Vec<String>,
    /// Literal variable-trace target names (sorted) from
    /// [`tcl_compiler::ir::Module::traced_variables`] — a whole-module fact
    /// identical for every procedure, folded into the key exactly like
    /// `known_classes`: SCCP's trace-safety gate (see
    /// [`tcl_compiler::sccp::sccp`]) treats a name in this set as never a
    /// compile-time constant, so a trace installed anywhere in the module can
    /// change any procedure's cached lattice.
    #[returns(ref)]
    pub traced_variables: Vec<String>,
    /// [`tcl_compiler::ir::Module::has_dynamic_variable_trace`] — `true` when
    /// a variable-trace install/remove call targets a non-literal name
    /// anywhere in the module. Folded into the key alongside
    /// `traced_variables`.
    pub has_dynamic_variable_trace: bool,
}

/// Memoised offset-0 baseline lattice (CFG → SSA → def-use → SCCP → type →
/// rendered → intra-procedural taint) for one procedure, built from its interned
/// offset-0 body + context.  A body-only edit changes only that procedure's
/// `FnLatticeKey`, so salsa reuses every other procedure's lattice; a shifted
/// body interns to the same key (cache hit).  Rebuilds the CFG via the same
/// `build_cfg_function_with_upvars` call `build_cfg` makes per procedure, so the
/// result equals the whole-module build's unit (modulo offset).  SCCP is seeded
/// with the key's interprocedural `param_constants` (caller-uniform-literal
/// folds), decoded back to the seed map `build_for_inner` would pass on the
/// fresh path — so a procedure with such seeds memoises instead of bypassing the
/// cache, and rebuilds only when a caller's literal at that position changes (a
/// new key).  The interprocedural taint re-run still happens at aggregation time
/// (`with_interprocedural`).  Uses `db.registry` — byte-identical to the
/// registry both diagnostics consumers build (`build_default` + `load_dialect`).
#[salsa::tracked]
pub fn function_lattice<'db>(db: &'db dyn TclDb, key: FnLatticeKey<'db>) -> Arc<FunctionUnit> {
    let context = key.context(db);
    let upvar: HashMap<String, UpvarInfo> = context.upvar_ctx(db).iter().cloned().collect();
    let proc_params: HashMap<String, Vec<String>> =
        context.proc_params(db).iter().cloned().collect();
    let global_write_procs: HashMap<String, GlobalWriteInfo> =
        context.global_write_ctx(db).iter().cloned().collect();
    let registry = db.registry(key.dialect(db));
    let cfg = build_cfg_function_with_upvars(
        key.qname(db),
        key.body(db),
        true,
        upvar,
        proc_params,
        global_write_procs,
    );
    let param_constants =
        tcl_compiler::compilation_unit::decode_param_constants(key.param_constants(db));
    let known_classes: HashSet<String> = key.known_classes(db).iter().cloned().collect();
    let traced_variables: BTreeSet<String> = key.traced_variables(db).iter().cloned().collect();
    let trace_facts = ModuleTraceFacts {
        traced_variables: &traced_variables,
        has_dynamic_variable_trace: key.has_dynamic_variable_trace(db),
    };
    Arc::new(FunctionUnit::build_with_param_constants_and_classes(
        key.qname(db),
        cfg,
        key.params(db),
        &registry,
        param_constants.as_ref(),
        &known_classes,
        trace_facts,
    ))
}

/// Interned identity of one top-level `proc`'s **offset-0** static body source
/// (SRV-INCREMENTAL Task 3 — incremental per-item IR *lowering*).  A `proc`
/// body is lowered against a clean slate (`lower_proc` pushes an empty const-map
/// frame, so the body inherits no tracked scalars from preceding code), so its
/// lowering is a pure function of `(body_text, namespace, dialect, config)` —
/// no position, no cross-item state.  An edit to one proc's body changes only
/// that proc's `ProcBodyKey`, so salsa reuses every other proc body's lowered
/// IR; an edit that merely *shifts* a body leaves its key unchanged (the caller
/// rebases the offset-0 `Script` back to the body's real offset).  Used only for
/// **context-free bodies** (the per-body gate `lowering::body_cache_eligible`)
/// where the isolated lowering is byte-identical to the in-place `lower_body`;
/// guarded by the corpus differential gates (`file_analysis_corpus` /
/// `compiler_check_corpus`).
#[salsa::interned]
pub struct ProcBodyKey<'db> {
    #[returns(ref)]
    pub body_text: String,
    #[returns(ref)]
    pub namespace: String,
    #[returns(ref)]
    pub dialect: String,
    /// The two dialect-varying [`tcl_lexer::LexerConfig`] fields (see
    /// [`LexerCfgKey`]); the rest are the invariant defaults both consumers use.
    pub expand_syntax: bool,
    pub irules_brace_separator: bool,
}

/// Memoised offset-0 isolated lowering of one top-level `proc` body
/// (SRV-INCREMENTAL Task 3).  Replicates the body-lowering setup `lower_proc`
/// performs for a static literal body (a fresh `Lowerer` at `proc_depth == 1`
/// with an empty const-map frame, lowering at offset 0).  Byte-identical to the
/// body the whole-file lowering produces for that procedure, normalised to
/// offset 0 — for the context-free files the caller gates on.
#[salsa::tracked]
pub fn lower_proc_body<'db>(db: &'db dyn TclDb, key: ProcBodyKey<'db>) -> Arc<Script> {
    let registry = db.registry(key.dialect(db));
    let config = tcl_lexer::LexerConfig {
        expand_syntax: key.expand_syntax(db),
        irules_brace_separator: key.irules_brace_separator(db),
        ..tcl_lexer::LexerConfig::default()
    };
    Arc::new(tcl_compiler::lowering::lower_proc_body_isolated(
        key.body_text(db),
        key.namespace(db),
        &registry,
        config,
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
#[must_use]
pub fn memoised_compilation_unit(
    db: &dyn TclDb,
    source: &str,
    registry: &CommandRegistry,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
    dialect_opt: Option<&str>,
) -> CompilationUnit {
    build_unit_with_keys(db, source, registry, defer_top_level, config, dialect_opt).0
}

/// [`memoised_compilation_unit`] that also returns the per-procedure
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
    registry: &CommandRegistry,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
    dialect_opt: Option<&str>,
) -> (CompilationUnit, HashMap<String, FnLatticeKey<'db>>) {
    let dialect = dialect_opt.unwrap_or("");
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
            CfgContext::new(db, upvar, proc_params, global_write)
        });
        let key = FnLatticeKey::new(
            db,
            req.body.clone(),
            req.qname.to_owned(),
            req.params.to_vec(),
            context,
            req.dialect.to_owned(),
            req.param_constants.to_vec(),
            req.known_classes.to_vec(),
            req.traced_variables.to_vec(),
            req.has_dynamic_variable_trace,
        );
        lattice_keys.insert(req.qname.to_owned(), key);
        (*function_lattice(db, key)).clone()
    };
    // SRV-INCREMENTAL Task 3: lower each *eligible* top-level proc body through the
    // `lower_proc_body` memo so a body-only edit re-lowers only the edited proc's
    // body (every other body's IR is reused). The per-body gate
    // (`lowering::body_cache_eligible`) decides which bodies take it, so a
    // context-carrying sibling no longer disables the cache for the whole file.
    // Byte-identical to the whole-file lowering (corpus differential gates).
    //
    // File-level precondition: a command alias declared *outside* any body
    // (`interp alias`) populates the alias table that `resolve_alias` consults
    // while lowering every body, but the isolated body lowering starts with an
    // empty table — so a file that may establish aliases forgoes the cache
    // entirely (the per-body scan cannot see a top-level alias).
    let cu = if tcl_compiler::lowering::source_may_alias_commands(source) {
        CompilationUnit::build_for_memoized(
            source,
            registry,
            defer_top_level,
            config,
            dialect,
            &mut lattice_memo,
        )
    } else {
        let body_memo = |body_text: &str, namespace: &str| -> Script {
            let key = ProcBodyKey::new(
                db,
                body_text.to_owned(),
                namespace.to_owned(),
                dialect.to_owned(),
                config.expand_syntax,
                config.irules_brace_separator,
            );
            (*lower_proc_body(db, key)).clone()
        };
        CompilationUnit::build_for_memoized_with_body_cache(
            source,
            registry,
            defer_top_level,
            config,
            dialect,
            &mut lattice_memo,
            &body_memo,
        )
    };
    // Memoise the per-procedure interprocedural taint re-run via `taint_cascade`.
    // The whole-module summary is still rebuilt here (it is the memo's input);
    // only unchanged procedures' `propagate_taints` is skipped.
    let unit = cu.with_interprocedural_memoized(
        registry,
        dialect_opt,
        &mut |qname: &str, ia: &InterproceduralAnalysis| {
            let key = *lattice_keys.get(qname)?;
            let summary_key = taint_summary_key(db, ia, qname, dialect);
            Some((*taint_cascade(db, key, summary_key)).clone())
        },
    );
    (unit, lattice_keys)
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
#[salsa::tracked]
pub fn taint_cascade<'db>(
    db: &'db dyn TclDb,
    lattice_key: FnLatticeKey<'db>,
    summary_key: TaintSummaryKey<'db>,
) -> Arc<HashMap<ValueKey, TaintLattice>> {
    let baseline = function_lattice(db, lattice_key);
    let dialect = summary_key.dialect(db);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(dialect);

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

    Arc::new(baseline.interproc_taints(&registry, &ia, dialect_opt))
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
        callee_set.extend(tcl_compiler::taint_interproc::resolved_callees(fu, known));
        if let Some(src) = body_source {
            callee_set.extend(tcl_compiler::taint_interproc::command_subst_callees(
                src, qname, known,
            ));
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
#[salsa::tracked]
pub fn proc_summary_cascade<'db>(
    db: &'db dyn TclDb,
    lattice_key: FnLatticeKey<'db>,
    deps_key: SummaryDepsKey<'db>,
) -> Arc<ReturnTaintSummary> {
    let fu = function_lattice(db, lattice_key);
    let qname = lattice_key.qname(db);
    let params = lattice_key.params(db);
    let dialect = deps_key.dialect(db);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(dialect);

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
        &registry,
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
#[salsa::tracked]
pub fn function_checks<'db>(db: &'db dyn TclDb, key: FnLatticeKey<'db>) -> Arc<Vec<CompilerCheck>> {
    let fu = function_lattice(db, key);
    let dialect = key.dialect(db);
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(dialect);
    // Per-procedure memo — procs have no implicit instance variables.
    Arc::new(tcl_compiler::compiler_checks::function_nontaint_checks(
        &fu,
        &registry,
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
#[salsa::tracked]
pub fn proc_taint_solve<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
) -> Arc<CheckSolve> {
    let dialect = file.dialect(db).clone();
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(&dialect);
    let (cu, lattice_keys) = build_unit_with_keys(
        db,
        file.text(db),
        &registry,
        false,
        cfg.to_config(db),
        dialect_opt,
    );
    let interproc = cu.interproc.as_ref();

    let taints = tcl_compiler::taint_interproc::solve_interprocedural_taints_with(
        &cu,
        &registry,
        dialect_opt,
        &mut |qname, params, fu, known, summaries| match lattice_keys.get(qname) {
            // Memoised path: the proc has an offset-0 baseline key.
            Some(&lattice_key) => {
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
                    &dialect,
                );
                (*proc_summary_cascade(db, lattice_key, deps_key)).clone()
            }
            // Fallback (a proc without a memoised lattice — e.g. an unanalysable
            // body): run the real inference directly, exactly as the bare solve.
            None => tcl_compiler::taint_interproc::infer_proc_summary(
                qname,
                params,
                fu,
                &registry,
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
        match lattice_keys.get(&fu.name) {
            Some(&key) => {
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
                // The built unit's fallback fus (complexity-guarded / top level)
                // carry **absolute** spans already (`base_offset == 0`), so the
                // per-function checks need no rebase.
                for d in tcl_compiler::compiler_checks::function_nontaint_checks(
                    fu,
                    &registry,
                    dialect_opt,
                    None::<&std::collections::HashSet<String>>,
                ) {
                    fn_checks.push(d);
                }
            }
        }
    }

    // The intrep-shimmer family alone, extended to `TclOO` method bodies and
    // synthetic body units (`apply` lambdas, `namespace eval` bodies). The
    // main per-function loop above still iterates the proc-only
    // `analysable_functions`, unlike `compiler_checks::run_all_checks_with_solved_and_patterns`'s
    // direct path (which iterates the wider `all_body_function_units` and so
    // needs no separate top-up — see `analysable_methods_and_body_units`'s
    // own doc comment for why adding it there too would double-count), so
    // this memoised path needs its own top-up loop to reach the same
    // methods/body units. These never get an offset-0 `FnLatticeKey`, so
    // they carry absolute spans already and need no rebase, same as the
    // `None` arm above.
    for fu in cu.analysable_methods_and_body_units() {
        for d in tcl_compiler::compiler_checks::shimmer_family_checks(
            fu,
            &registry,
            dialect_opt,
            cu.method_instance_vars(&fu.name),
        ) {
            fn_checks.push(d);
        }
    }

    let optimisations = solve_optimisations(db, &cu, &lattice_keys, &registry, dialect_opt);
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
    s
}

/// Interned per-procedure optimiser dependency key: the offset-0 body source (for
/// the optimiser's `source[span]` reads), the cross-proc resolution domain (every
/// module proc qname), the opt-relevant summaries of this proc's resolved direct
/// callees (O103 fold / purity inputs), and the module `redefined_procedures` set
/// (the O103 don't-fold-a-redefined-callee gate).  A body edit to an unrelated proc
/// whose summary this proc doesn't read leaves this key unchanged → cache hit.
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
    )
}

/// Memoised offset-0 raw optimisations for one procedure (SRV-INCREMENTAL Task 4).
/// Builds a single-procedure offset-0 [`CompilationUnit`] — the proc's offset-0
/// `function_lattice` unit, its offset-0 IR body, and the reconstructed interproc
/// (domain stubs overlaid with the resolved direct callees' real opt summaries) —
/// and runs [`optimise_unit_raw`] on it.  Returns the **raw** (pre-overlap-select,
/// pre-renumber) optimisations at **offset 0**; the caller rebases by the proc's
/// `body_offset` and runs the whole-module [`finalise_optimisations`] over the
/// assembled set.  A body edit re-runs only the edited proc; an unrelated proc's
/// edit is a cache hit unless this proc reads its summary (a resolved direct call).
#[salsa::tracked]
pub fn function_optimisations<'db>(
    db: &'db dyn TclDb,
    key: FnLatticeKey<'db>,
    deps: OptDepsKey<'db>,
) -> Arc<Vec<Optimisation>> {
    let fu = function_lattice(db, key);
    let qname = key.qname(db).clone();
    let params = key.params(db).clone();
    let body = key.body(db).clone();
    let dialect = key.dialect(db).clone();
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(&dialect);
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
        namespace_scoped: false,
        base_priority: 0,
    };
    let mut ir_procs = HashMap::new();
    ir_procs.insert(qname.clone(), proc);
    let ir_module = tcl_compiler::ir::Module {
        source: body_source.clone(),
        top_level: tcl_compiler::ir::Script::new(),
        procedures: ir_procs,
        methods: HashMap::new(),
        body_units: HashMap::new(),
        redefined_procedures: redefined,
        redefined_methods: HashSet::new(),
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
    };
    let empty_cfg = tcl_compiler::cfg::Function::new("::", "entry");
    let top_fu = FunctionUnit::build("::", empty_cfg.clone(), &[], &registry);
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
        top_level: top_fu,
        procedures: fu_procs,
        methods: HashMap::new(),
        body_units: HashMap::new(),
        interproc: Some(ia),
        connection_scope: None,
    };
    Arc::new(tcl_compiler::optimiser::optimise_unit_raw(
        &cu,
        &registry,
        dialect_opt,
    ))
}

/// Whether `module` carries any whole-module trace fact (execution *or*
/// variable — `Module::traced_commands` / `has_dynamic_trace` /
/// `traced_variables` / `has_dynamic_variable_trace`).
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
}

/// Assemble a document's optimisations from the per-procedure memo (Task 4).
///
/// For a non-iRules module with no command mutations and a lattice key for every
/// analysable procedure, each proc's raw optimisations come from the memoised
/// [`function_optimisations`] (offset 0), rebased by the proc's `body_offset`; the
/// top-level body's raw optimisations are computed on a top-level-only unit (small,
/// not memoised), and the whole-module [`finalise_optimisations`] runs once over the
/// assembled set.  Otherwise (iRules / command mutations / a complexity-guarded
/// proc without a key) it falls back to the whole-module [`optimise_unit`] — always
/// byte-identical, guarded by the `compiler_check` random-edit + corpus fuzzers.
fn solve_optimisations<'db>(
    db: &'db dyn TclDb,
    cu: &CompilationUnit,
    lattice_keys: &HashMap<String, FnLatticeKey<'db>>,
    registry: &CommandRegistry,
    dialect_opt: Option<&str>,
) -> Vec<Optimisation> {
    let mutations =
        tcl_compiler::command_binding::scan_module_command_mutations(&cu.ir_module, registry);
    let every_proc_keyed = cu
        .procedures
        .keys()
        .all(|qname| lattice_keys.contains_key(qname));
    let is_irules = tcl_compiler::taint::is_irules_dialect(dialect_opt);
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
    if is_irules
        || !cu.methods.is_empty()
        || mutations != tcl_compiler::command_binding::ModuleCommandMutations::default()
        || has_arg_sensitive_target
        || !every_proc_keyed
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
    let top_unit = CompilationUnit {
        source: cu.source.clone(),
        ir_module: tcl_compiler::ir::Module {
            source: cu.source.clone(),
            top_level: cu.ir_module.top_level.clone(),
            procedures: HashMap::new(),
            methods: HashMap::new(),
            body_units: HashMap::new(),
            redefined_procedures: redefined.clone(),
            redefined_methods: HashSet::new(),
            namespace_imports: Vec::new(),
            namespace_exports: Vec::new(),
            // Copied from the real whole-module facts (unlike the per-proc
            // offset-0 unit above, this top-level-only unit is built
            // straight from `cu`, so there is no salsa-caching reason to
            // default these — and the `has_trace_facts` fallback above
            // means this path only runs at all when the module has none,
            // but copy the real values rather than asserting that by
            // omission).
            traced_commands: cu.ir_module.traced_commands.clone(),
            has_dynamic_trace: cu.ir_module.has_dynamic_trace,
            traced_variables: cu.ir_module.traced_variables.clone(),
            has_dynamic_variable_trace: cu.ir_module.has_dynamic_variable_trace,
        },
        cfg_module: tcl_compiler::cfg::CfgModule {
            top_level: cu.cfg_module.top_level.clone(),
            procedures: HashMap::new(),
        },
        top_level: cu.top_level.clone(),
        procedures: HashMap::new(),
        methods: HashMap::new(),
        body_units: HashMap::new(),
        interproc: cu.interproc.clone(),
        connection_scope: None,
    };
    for mut opt in tcl_compiler::optimiser::optimise_unit_raw(&top_unit, registry, dialect_opt) {
        if let Some(g) = opt.group {
            opt.group = Some(group_base + g);
        }
        raw.push(opt);
    }

    tcl_compiler::optimiser::finalise_optimisations(&raw, cu, registry, dialect_opt)
}

/// Interned identity of the dialect-varying [`tcl_lexer::LexerConfig`] fields,
/// the salsa key that lets the two diagnostics consumers *share* one built
/// [`CompilationUnit`].  Only `expand_syntax` / `irules_brace_separator` vary
/// between [`LexerConfig::default`] (the analyser tail) and
/// [`LexerConfig::for_dialect`] (the optimiser); the rest are always the default
/// (`strict_quoting = false`, zero base offsets) on both paths.  The two configs
/// **coincide for every dialect except `tcl8.4` / `f5-irules`**, so for the
/// common case both consumers intern the same key and demand the same
/// [`compilation_unit`] — built once per edit instead of twice.
#[salsa::interned]
pub struct LexerCfgKey<'db> {
    pub expand_syntax: bool,
    pub irules_brace_separator: bool,
}

impl LexerCfgKey<'_> {
    /// The full [`tcl_lexer::LexerConfig`] this key represents (the two
    /// interned fields + the invariant defaults both diagnostics paths use).
    fn to_config(self, db: &dyn TclDb) -> tcl_lexer::LexerConfig {
        tcl_lexer::LexerConfig {
            expand_syntax: self.expand_syntax(db),
            irules_brace_separator: self.irules_brace_separator(db),
            ..tcl_lexer::LexerConfig::default()
        }
    }
}

/// Intern a [`LexerCfgKey`] from a concrete [`tcl_lexer::LexerConfig`].
fn lexer_cfg_key(db: &dyn TclDb, config: tcl_lexer::LexerConfig) -> LexerCfgKey<'_> {
    LexerCfgKey::new(db, config.expand_syntax, config.irules_brace_separator)
}

/// The shared, memoised [`CompilationUnit`] for a document under a given lexer
/// config — built via [`memoised_compilation_unit`] (per-procedure lattices on
/// the salsa-native [`function_lattice`] graph).  Tracked + keyed on
/// `(file, cfg)` so the analyser tail ([`file_analysis_incremental`]) and the
/// optimiser/compiler-checks pass ([`compiler_check_diagnostics`]) **share one
/// build per edit** whenever their configs coincide (every dialect bar `tcl8.4`
/// / `f5-irules`); for those two dialects the configs differ, so each consumer
/// builds its own (status quo).  Byte-identical to a direct
/// `memoised_compilation_unit` call.
#[salsa::tracked]
pub fn compilation_unit<'db>(
    db: &'db dyn TclDb,
    file: SourceFile,
    cfg: LexerCfgKey<'db>,
) -> Arc<CompilationUnit> {
    let dialect = file.dialect(db).clone();
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(&dialect);
    Arc::new(memoised_compilation_unit(
        db,
        file.text(db),
        &registry,
        false,
        cfg.to_config(db),
        dialect_opt,
    ))
}

/// Incremental whole-file analysis: the per-item path with each `proc` body's
/// isolated analysis memoised via [`item_body_analysis`], so a body edit
/// recomputes one body + the cheap shell instead of the whole walk; the
/// CFG/SSA diagnostic tail's per-procedure lattices are likewise memoised via
/// the salsa-native [`function_lattice`] query (through
/// [`memoised_compilation_unit`]), so an unchanged procedure's lattice is reused
/// (and rebased) instead of rebuilt.  Byte-identical to [`file_analysis`] (and
/// `analyse`) — proven by the `per_item_corpus` gate over the shared
/// `analyse_per_item_with` orchestration.
#[salsa::tracked]
pub fn file_analysis_incremental(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let disabled_vec = config.disabled_diagnostics(db).clone();
    let non_ascii = config.non_ascii_mode(db);
    let extra_commands: HashSet<String> = config.extra_commands(db).iter().cloned().collect();
    let dialect = file.dialect(db).clone();
    let text = file.text(db).clone();
    let mut analyser = Analyser::with_disabled_diagnostics(disabled_vec.iter().cloned().collect())
        .with_non_ascii_mode(non_ascii)
        .with_extra_commands(extra_commands)
        .with_file_path(file.path(db).clone());

    // Build the CFG/SSA tail's compilation unit with per-procedure lattices
    // memoised by `function_lattice`, and feed it through the analyser's
    // `cu_override` seam, via the shared [`compilation_unit`] query.  The default
    // lexer config mirrors what `emit_cfg_ssa_diagnostics` builds for itself, so
    // the supplied unit is the one it would otherwise build; routing through the
    // tracked query lets `compiler_check_diagnostics` reuse this exact build in
    // the same edit whenever the dialect's config matches the default (every
    // dialect but `tcl8.4` / `f5-irules`).
    let cfg_key = lexer_cfg_key(db, tcl_lexer::LexerConfig::default());
    analyser.set_cu_override(compilation_unit(db, file, cfg_key));

    let mut body_fn = |body: &DeferredBody| -> BodyFragment {
        let key = ItemBodyKey::new(
            db,
            body.body_text.clone(),
            body.namespace.clone(),
            body.scope_name.clone(),
            body.params.clone(),
            body.is_method,
            body.oo_global_resolution,
            body.class_variables.clone(),
            dialect.clone(),
            disabled_vec.clone(),
            non_ascii,
        );
        (*item_body_analysis(db, key)).clone()
    };
    Arc::new(analyser.analyse_per_item_with(&text, &dialect, &mut body_fn))
}

/// The compiler-checks + optimiser diagnostics for one document, unfiltered.
///
/// Returned by [`compiler_check_diagnostics`] for the server to filter
/// (optimiser master switch / per-code disables) and lift into LSP diagnostics.
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
    dialect_opt: Option<&str>,
    generic_patterns: Option<&[String]>,
) -> CompilerDiagnostics {
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
/// never cross-pollute.  Byte-identical to the former direct
/// `lift_compiler_diagnostics` build.
#[salsa::tracked]
pub fn compiler_check_diagnostics(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<CompilerDiagnostics> {
    let dialect = file.dialect(db).clone();
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    let registry = db.registry(&dialect);
    // Share the analyser tail's build via the [`compilation_unit`] query when the
    // dialect's lexer config matches the default (every dialect but `tcl8.4` /
    // `f5-irules`): the optimiser lowers with the dialect config, so a matching
    // config interns the same `LexerCfgKey` and reuses the same per-edit build.
    let cfg_key = lexer_cfg_key(db, tcl_lexer::LexerConfig::for_dialect(&dialect));
    let cu = compilation_unit(db, file, cfg_key);
    // Both halves of `run_all_checks` come from the memoised [`proc_taint_solve`]:
    // the interprocedural taint solve (`solve.taints`)
    // and the per-procedure non-taint checks (`solve.fn_checks`, already rebased),
    // so an unchanged procedure contributes neither a re-solve nor a re-check.
    // The remaining taint-family + iRules module checks (which read the solved
    // taints) are appended over the shared build, then the combined set is sorted
    // into the same deterministic order `run_all_checks` produces.  Byte-identical
    // to the in-line build; guarded by the corpus differential.  Optimiser
    // unchanged.
    let solve = proc_taint_solve(db, file, cfg_key);
    let mut checks = solve.fn_checks.clone();
    let generic_patterns = config.generic_variable_patterns(db).as_deref();
    tcl_compiler::compiler_checks::push_taint_and_module_checks(
        &cu,
        &registry,
        dialect_opt,
        &solve.taints,
        generic_patterns,
        &mut checks,
    );
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
) -> CompilerDiagnostics {
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    let cu = CompilationUnit::build_for_with_config(
        text,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    )
    .with_interprocedural(registry, dialect_opt);
    compiler_diagnostics_from_unit(&cu, registry, dialect_opt, generic_patterns)
}

/// Document outline — wraps `document_symbols_from_analysis`, reusing the
/// tracked [`file_analysis_incremental`] so the outline shares the per-item
/// memoised analysis with the push-diagnostics path in the same edit.
#[salsa::tracked]
pub fn document_symbols(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<DocumentSymbol> {
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::document_symbols::document_symbols_from_analysis(file.text(db), &analysis)
}

/// The document's [`CompilationUnit`] under the default lexer config — a thin
/// wrapper over [`compilation_unit`] that interns the `LexerCfgKey` from the
/// file's dialect, so callers that only have `(db, file)` (semantic tokens,
/// server-side accessors) share the same memoised build as the diagnostics
/// path.
#[salsa::tracked]
pub fn document_compilation_unit(db: &dyn TclDb, file: SourceFile) -> Arc<CompilationUnit> {
    let cfg_key = lexer_cfg_key(db, tcl_lexer::LexerConfig::for_dialect(file.dialect(db)));
    compilation_unit(db, file, cfg_key)
}

/// Semantic tokens — wraps `semantic_tokens::full_with_cu`; reads the durable
/// registry.
///
/// This query demands the document's [`CompilationUnit`] so a `regexp` /
/// `regsub` pattern supplied through a provably-constant string variable
/// highlights its originating `set` literal as a regex (see
/// [`tcl_compiler::regex_source`]), and the whole-file analysis so a
/// `$obj method …` / `[dict get $objs $k] method …` dispatch resolves against
/// user classes and their `oo::configurable` properties (issue #797), not only
/// registry ones. That reworked the earlier "tokens never touch the analysis
/// pipeline" latency shortcut (issue #333) in favour of correctness — but the
/// analysis half originally reused the coarse, non-incremental
/// [`file_analysis`] rather than [`file_analysis_incremental`], which
/// reintroduced a latency/starvation regression (issue #829): every token
/// request paid for a *third* independent whole-file analyser walk (on top of
/// the two the diagnostics path already shares via [`compilation_unit`]), and
/// that walk has no interior salsa cancellation checkpoint, so a concurrent
/// edit's `set_text` blocks until it finishes.  Using
/// [`file_analysis_incremental`] here instead keeps the #797 correctness fix
/// (identical `AnalysisResult` shape, proven byte-identical to `file_analysis`
/// by the `per_item_corpus` gate) while sharing the diagnostics path's
/// per-item memoisation and cancellation checkpoints: a token request that
/// lands after diagnostics have already analysed this revision is a cache
/// hit, and a cold request is preemptible by the next edit instead of running
/// an uninterruptible pass to completion.
#[salsa::tracked]
pub fn semantic_tokens(db: &dyn TclDb, file: SourceFile, config: AnalyserConfig) -> SemanticTokens {
    let registry = db.registry(file.dialect(db));
    let cu = document_compilation_unit(db, file);
    let analysis = file_analysis_incremental(db, file, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
        file.text(db),
        file.dialect(db),
        &registry,
        Some(&cu),
        Some(&analysis),
    )
}

/// The project's workspace-merged class hierarchy: every file's `ClassDef`s
/// unioned into one cross-file MRO index, so a `$obj method` dispatch resolves
/// against a class defined in *another* file.
///
/// Aggregates `file_analysis_incremental(f, config).all_classes` across
/// `project`.  A body-only edit rebuilds this (cheap re-aggregation) but yields
/// an equal [`ClassHierarchy`], so salsa backdates and dependent token queries
/// do not recompute.  On a class-signature change the merged index changes and
/// the affected tokens recompute — the correct cross-file invalidation.
///
/// Uses the incremental, per-item-memoised [`file_analysis_incremental`] (not
/// the coarse [`file_analysis`]) for each project file, the same template
/// [`project_diagnostics`] establishes: a file whose diagnostics the worker has
/// already analysed for this revision is a cache hit here too, rather than a
/// second independent whole-file walk (issue #829).
#[salsa::tracked]
pub fn project_class_index(
    db: &dyn TclDb,
    project: Project,
    config: AnalyserConfig,
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
    for &file in project.files(db) {
        let analysis = file_analysis_incremental(db, file, config);
        for (name, class) in &analysis.all_classes {
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
/// target even when `myproc` is defined in another file (issue #813 follow-up).
///
/// A proc name defined with *conflicting* roles across files is dropped as
/// ambiguous by [`VarNameArgRoles::from_procs`], so the merged index is
/// order-independent — matching the abstention posture of
/// [`project_class_index`].
///
/// Uses [`file_analysis_incremental`] per file, matching [`project_class_index`]
/// and [`project_diagnostics`] — a cache hit against the diagnostics worker's
/// already-computed per-item analysis rather than a second whole-file walk.
#[salsa::tracked]
pub fn project_proc_var_index(
    db: &dyn TclDb,
    project: Project,
    config: AnalyserConfig,
) -> Arc<VarNameArgRoles> {
    let analyses: Vec<Arc<AnalysisResult>> = project
        .files(db)
        .iter()
        .map(|&file| file_analysis_incremental(db, file, config))
        .collect();
    Arc::new(VarNameArgRoles::from_procs(
        analyses.iter().flat_map(|a| a.all_procs.values()),
    ))
}

/// [`semantic_tokens`] resolved against the **workspace-merged** class index, so
/// a `$obj method …` dispatch on a class defined in another project file
/// resolves too.  The server calls this when a [`Project`] is available; the
/// bare [`semantic_tokens`] (local file only) is the fallback.
#[salsa::tracked]
pub fn semantic_tokens_project(
    db: &dyn TclDb,
    file: SourceFile,
    config: AnalyserConfig,
    project: Project,
) -> SemanticTokens {
    let registry = db.registry(file.dialect(db));
    let cu = document_compilation_unit(db, file);
    let classes = project_class_index(db, project, config);
    let proc_roles = project_proc_var_index(db, project, config);
    tcl_lsp_core::semantic_tokens::full_with_cu_and_classes_and_roles(
        file.text(db),
        file.dialect(db),
        &registry,
        Some(&cu),
        Some(&classes),
        Some(&proc_roles),
    )
}

/// Folding ranges — wraps `folding::folding_ranges`; reads the durable registry.
#[salsa::tracked]
pub fn folding_ranges(db: &dyn TclDb, file: SourceFile) -> Vec<FoldingRange> {
    let registry = db.registry(file.dialect(db));
    tcl_lsp_core::folding::folding_ranges(file.text(db), file.dialect(db), &registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(db: &TclDatabase) -> AnalyserConfig {
        AnalyserConfig::new(db, Vec::new(), NonAsciiMode::Default, Vec::new(), None)
    }

    const SRC: &str = "proc greet {name} {\n    puts \"hi $name\"\n}\n# c\nset x 1\n";

    #[test]
    fn body_cache_gate_is_per_body_and_whitespace_aware() {
        use tcl_compiler::lowering::body_cache_eligible;
        // A plain, context-free body is eligible.
        assert!(body_cache_eligible(" set x 1 "));
        assert!(body_cache_eligible(" puts hi "));
        // A body carrying a cross-item construct is not — including the tab-
        // separated forms (Codex #731): the isolated lowerer drops the effect.
        assert!(!body_cache_eligible(" interp\talias {} x {} y "));
        assert!(!body_cache_eligible(" namespace\timport ::ns::* "));
        assert!(!body_cache_eligible(" rename set myset "));
        assert!(!body_cache_eligible(" oo::class create C "));
        // A nested `proc` disqualifies the enclosing body.
        assert!(!body_cache_eligible(" proc inner {} {} "));
        // The gate is per-body: a context-carrying sibling no longer disables the
        // clean body — the clean body stays eligible on its own.
        assert!(body_cache_eligible(" set y 2 "));
    }

    #[test]
    fn compiler_check_o122_tailrec_memo_matches_uncached() {
        // An impure (side-effecting) tail-recursive proc fires O122
        // (recursion→loop). The per-proc optimise memo must reconstruct
        // `proc.body_source` as the *body text* — not the whole-command slice used
        // for span alignment — so the loop-conversion replacement wraps only the
        // body, not the entire `proc …` declaration (Codex #731 review, lib.rs:1743).
        let dialect = "tcl8.6";
        let src = "proc countdown {n} {\n    puts $n\n    if {$n <= 0} { return }\n    countdown [expr {$n - 1}]\n}\n";
        let db = TclDatabase::default();
        let registry = db.registry(dialect);
        let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned(), None);
        let got = compiler_check_diagnostics(&db, file, cfg(&db));
        let want = compiler_check_diagnostics_uncached(src, &registry, dialect, None);
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

    #[test]
    fn document_symbols_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = document_symbols(&db, file, cfg(&db));
        let expected = tcl_lsp_core::document_symbols::document_symbols(SRC, "tcl");
        assert_eq!(got, expected);
    }

    #[test]
    fn semantic_tokens_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned(), None);
        let got = semantic_tokens(&db, file, cfg(&db));
        let reg = db.registry("tcl");
        let expected = tcl_lsp_core::semantic_tokens::full(SRC, "tcl", &reg);
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
    /// not a no-op (issue #829: the fast-path fallback in
    /// `Backend::semantic_tokens_core_data` trades this enrichment away
    /// temporarily, so it must exist for the trade to mean anything).
    #[test]
    fn semantic_tokens_retags_constant_regex_source_true_positive() {
        let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let enriched = semantic_tokens(&db, file, cfg(&db));
        let reg = db.registry("tcl9.0");
        let coarse = tcl_lsp_core::semantic_tokens::full(src, "tcl9.0", &reg);
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
        let coarse = tcl_lsp_core::semantic_tokens::full(src, "tcl9.0", &reg);
        assert_eq!(
            enriched, coarse,
            "a non-constant pattern source must not be retagged by either tier"
        );
    }

    /// Issue #829 root-cause regression test: `semantic_tokens` must depend on
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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

    /// [`project_class_index`] and [`project_proc_var_index`] must likewise
    /// use [`file_analysis_incremental`] per project file (not the coarse
    /// [`file_analysis`]), matching [`project_diagnostics`]'s established
    /// template — otherwise `semantic_tokens_project` pays for a fresh
    /// whole-file walk of every project file on top of what diagnostics
    /// already computed for them.
    #[test]
    fn project_indexes_use_incremental_analysis_not_coarse() {
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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

        let _ = project_class_index(&db, project, cfg);
        let _ = project_proc_var_index(&db, project, cfg);
        let log_snapshot = log.lock().unwrap().clone();
        assert!(
            log_snapshot
                .iter()
                .any(|s| s.contains("file_analysis_incremental")),
            "project indexes must read the incremental analysis: {log_snapshot:?}"
        );
        assert!(
            log_snapshot.iter().all(|s| !s.contains("file_analysis(")),
            "project indexes must never invoke the coarse file_analysis query: \
             {log_snapshot:?}"
        );
    }

    #[test]
    fn pre_warming_makes_project_index_loop_all_cache_hits() {
        // #844 Gap 3: the server-layer parallel warm pre-populates
        // `file_analysis_incremental` for every project file so the enriched
        // `project_class_index` / `project_proc_var_index` loops are pure cache
        // hits — no per-file re-analysis. That is exactly what lets the warm
        // collapse a cold workspace's serial walk into a parallel one: the
        // tracked query does only the cheap cross-file aggregation, not N whole
        // -file analyses. This pins the salsa property the warm relies on.
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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

        // The warm: analyse every project file once, exactly what
        // `spawn_workspace_warm` fans across the blocking pool.  Uses the *same*
        // `(file, config)` keys the project indexes will read.
        for &file in project.files(&db) {
            let _ = file_analysis_incremental(&db, file, cfg);
        }
        // From here the project-index loops must not re-execute the per-file
        // query — every read is served from the warmed cache.
        log.lock().unwrap().clear();
        let _ = project_class_index(&db, project, cfg);
        let _ = project_proc_var_index(&db, project, cfg);
        let after = log.lock().unwrap().clone();
        assert!(
            after
                .iter()
                .all(|s| !s.contains("file_analysis_incremental")),
            "after the warm, the project indexes must hit cache, not re-run \
             file_analysis_incremental: {after:?}"
        );
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
        let cross = semantic_tokens_project(&db, user, cfg(&db), project);
        let local = semantic_tokens(&db, user, cfg(&db));
        assert_ne!(
            cross.data, local.data,
            "cross-file index must resolve the method the local pass leaves unresolved"
        );
        // The merged index sees the class from the other file.
        assert!(
            project_class_index(&db, project, cfg(&db))
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
                !project_class_index(&db, project, cfg(&db))
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
        let got = folding_ranges(&db, file);
        let reg = db.registry("tcl");
        let expected = tcl_lsp_core::folding::folding_ranges(SRC, "tcl", &reg);
        assert_eq!(got, expected);
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

    /// The slice-3 firewall: a body-only edit (same length, so other bodies
    /// keep their offset) recomputes exactly one `item_body_analysis`.
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
                AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
            );
            let registry = db.registry(dialect);
            let want = compiler_check_diagnostics_uncached(src, &registry, dialect, None);
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
                AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
            );
            let want = compiler_check_diagnostics_uncached(src, &registry, dialect, None);
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

    /// Random-edit differential fuzzer for the **whole** memoised checks path
    /// (SRV-INCREMENTAL Task 2b verification gate — the "random-edit fuzzer still
    /// to build" the status table flags).  The cold corpus differential and the
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
            let want = compiler_check_diagnostics_uncached(&src, &registry, dialect, None);
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
            registries: Arc::default(),
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
            AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
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
            AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
        );
        assert_eq!(
            cascades(&log),
            1,
            "unrelated body edit -> exactly ONE taint cascade recomputes"
        );
    }

    /// The per-procedure optimiser memo (`function_optimisations`, Task 4) must
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
            registries: Arc::default(),
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

    /// SRV-INCREMENTAL Task 3: the per-procedure body-lowering memo
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
            registries: Arc::default(),
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
            registries: Arc::default(),
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
            AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
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
            AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None),
        );
        assert_eq!(
            summaries(&log),
            1,
            "unrelated body edit -> exactly ONE summary inference recomputes"
        );
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
            registries: Arc::default(),
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
            registries: Arc::default(),
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned(), None);
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned(), None);
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
    /// the synthesized arity error is produced *after* the analyser's own code
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
        let cfg_on = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let base = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
        assert!(has_w123(base), "baseline W123 expected");
        // With the command declared extra → suppressed.
        let cfg = AnalyserConfig::new(
            &db,
            Vec::new(),
            NonAsciiMode::Default,
            vec!["mylibsend".to_owned()],
            None,
        );
        assert!(!has_w123(cfg), "extraCommands should suppress W123");
    }

    /// Cross-file classes: a command resolving to a
    /// class (the class command) defined in another project file is resolved —
    /// W123 suppressed — and, being a non-proc, never draws an arity error.
    #[test]
    fn project_diagnostics_resolves_cross_file_class() {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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

    /// Regression for a real `proc_arity` bug: a required parameter
    /// positioned *after* a defaulted one does not lower the minimum by
    /// the defaulted parameters ahead of it — Tcl's argument binding is
    /// strictly positional, so supplying a value for the later required
    /// parameter requires also supplying one for every position before
    /// it, including the "optional" one. Confirmed against real `tclsh`
    /// 9.0.4: `proc opt {a {b 5} c} {}` accepts exactly 3 arguments,
    /// never 2 — the old formula (`min` = count of non-default params =
    /// 2) silently accepted a 2-argument call that real Tcl rejects.
    #[test]
    fn cross_file_arity_required_after_default_forces_exact_count() {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
    fn callback_arity_codes(src: &str) -> Vec<(String, String)> {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
        let f = SourceFile::new(&db, src.to_owned(), "tcl9.0".to_owned(), None);
        let proj = Project::new(&db, vec![f]);
        project_diagnostics(&db, f, cfg, proj)
            .iter()
            .map(|d| (d.code.as_str().to_owned(), d.message.clone()))
            .collect()
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
        // appended 2, for 3 total; `cb` needs 4 → E002.  Before this fix,
        // a braced multi-word prefix was silently dropped entirely (never
        // even recorded as an invocation), so this drew nothing at all.
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
    fn callback_arity_atleast_does_not_false_fire_too_few() {
        // FP guard: `trace add variable` appends 3 (AtLeast? no — Exactly(3)); use
        // `trace add execution` (AtLeast(2)) against a 4-param handler.  The
        // open-ended max means "too few" must not fire even though `min`=2 < 4.
        let src = "proc h {a b c d} { return 0 }\ntrace add execution somecmd enter h\n";
        let d = callback_arity_codes(src);
        assert!(
            !d.iter().any(|(c, _)| c == "E002"),
            "an AtLeast(2) callback must not false-fire E002 against a 4-param handler; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_namespace_unknown_zero_param_draws_e003() {
        // `namespace unknown h` invokes `h cmd ?args...?` (AtLeast(1)); a 0-param
        // handler can never accept the appended command name → E003 too-many. A
        // real bug (ground truth: tclsh 9.0 raises "called with too many args").
        let d = callback_arity_codes("proc h {} { return 0 }\nnamespace unknown h\n");
        assert!(
            d.iter().any(|(c, m)| c == "E003" && m.contains("'h'")),
            "a 0-param `namespace unknown` handler must draw E003; got {d:?}"
        );
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
        // Post-conversion: `scale -command` appends the new value (Exactly(1)).
        // A 0-param callback can't accept it → E003; a bareword 1-param callback
        // is silent (TN).
        let bad = callback_arity_codes("proc onChange {} { }\nscale .s -command onChange\n");
        assert!(
            bad.iter()
                .any(|(c, m)| c == "E003" && m.contains("onChange")),
            "a 0-param `scale -command` callback (1 appended) must draw E003; got {bad:?}"
        );
        let ok = callback_arity_codes("proc onChange {v} { }\nscale .s -command onChange\n");
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 1-param scale callback must be silent; got {ok:?}"
        );
        // A braced widget-path scroll callback is never arity-checked (not a
        // literal bareword head) — no false arity error.
        let widget = callback_arity_codes("listbox .lb -yscrollcommand {.sb set}\n");
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
            "proc f {x y} { expr {$x + $y} }\nmath::calculus::integral 0 1 100 f\n",
        );
        assert!(
            d.iter().any(|(c, m)| c == "E002" && m.contains("'f'")),
            "a 2-param func where calculus::integral appends 1 must draw E002; got {d:?}"
        );
        // The correct 1-param shape is silent (TN).
        let ok = callback_arity_codes(
            "proc f {x} { expr {$x * 2} }\nmath::calculus::integral 0 1 100 f\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 1-param calculus func must be silent; got {ok:?}"
        );
    }

    #[test]
    fn callback_arity_unknown_appended_never_fires() {
        // FP guard: `coroinject`/`coroprobe` carry `Unknown` appended arity
        // (depends on the yield point), so the injected command is a reference
        // only — never arity-checked, whatever its param count.
        let d = callback_arity_codes("proc h {} { return 0 }\ncoroinject myCoro h\n");
        assert!(
            !d.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "an Unknown-arity callback must never draw an arity error; got {d:?}"
        );
    }

    #[test]
    fn callback_arity_option_value_exactly_checked() {
        // `smtp -tlspolicy` and `halfpipe -write-command` both append exactly 2;
        // a 1-param callback is over-fed → E003, a 2-param callback is silent.
        let smtp_bad =
            callback_arity_codes("proc pol {code} { }\nsmtp::sendmessage $t -tlspolicy pol\n");
        assert!(
            smtp_bad
                .iter()
                .any(|(c, m)| c == "E003" && m.contains("'pol'")),
            "a 1-param -tlspolicy callback (2 appended) must draw E003; got {smtp_bad:?}"
        );
        let smtp_ok =
            callback_arity_codes("proc pol {code diag} { }\nsmtp::sendmessage $t -tlspolicy pol\n");
        assert!(
            !smtp_ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 2-param -tlspolicy callback must be silent; got {smtp_ok:?}"
        );
        let pipe_bad =
            callback_arity_codes("proc w {chan} { }\ntcl::chan::halfpipe -write-command w\n");
        assert!(
            pipe_bad
                .iter()
                .any(|(c, m)| c == "E003" && m.contains("'w'")),
            "a 1-param -write-command callback (2 appended) must draw E003; got {pipe_bad:?}"
        );
    }

    #[test]
    fn callback_arity_mime_getbody_command_atleast_one() {
        // `mime::getbody -command` appends AtLeast(1) (reason keyword + optional
        // payload).  A 0-param callback can't accept the reason word → E003; the
        // canonical `{reason args}` shape is silent (open-ended max ⇒ no
        // false "too many").
        let bad = callback_arity_codes("proc cb {} { }\nmime::getbody $t -command cb\n");
        assert!(
            bad.iter().any(|(c, m)| c == "E003" && m.contains("'cb'")),
            "a 0-param mime -command callback must draw E003; got {bad:?}"
        );
        let ok = callback_arity_codes("proc cb {reason args} { }\nmime::getbody $t -command cb\n");
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a `{{reason args}}` mime -command callback must be silent; got {ok:?}"
        );
        // FP guard: comm's 14-arg reply callback is virtually always `{args}` —
        // the catch-all absorbs all 14 → no arity error.
        let comm = callback_arity_codes(
            "proc reply {args} { }\ncomm::comm send -command reply $id {list x}\n",
        );
        assert!(
            !comm.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a variadic comm -command reply handler must be silent; got {comm:?}"
        );
    }

    #[test]
    fn callback_arity_struct_graph_walk_command_checked() {
        // `$g walk … -command cb` (object instance method) appends 3 (action
        // graphName node).  A 2-param callback is over-fed → E003; a 3-param one
        // is silent.  Exercises both the named (`struct::graph name`) and handle
        // (`set g [struct::graph]`) instance forms.
        let bad = callback_arity_codes(
            "proc twoP {a b} { }\nstruct::graph myG\nmyG walk root -command twoP\n",
        );
        assert!(
            bad.iter().any(|(c, m)| c == "E003" && m.contains("'twoP'")),
            "a 2-param graph walk -command callback (3 appended) must draw E003; got {bad:?}"
        );
        let ok = callback_arity_codes(
            "proc threeP {a b c} { }\nset g [struct::graph]\n$g walk root -command threeP\n",
        );
        assert!(
            !ok.iter().any(|(c, _)| c == "E002" || c == "E003"),
            "a correct 3-param graph walk callback must be silent; got {ok:?}"
        );
    }

    #[test]
    fn callback_arity_struct_tree_walkproc_checked() {
        // `$t walkproc … cmdprefix` (trailing positional prefix) appends 3 (tree
        // node action).  A 2-param callback → E003.
        let bad =
            callback_arity_codes("proc twoP {a b} { }\nstruct::tree myT\nmyT walkproc root twoP\n");
        assert!(
            bad.iter().any(|(c, m)| c == "E003" && m.contains("'twoP'")),
            "a 2-param tree walkproc callback (3 appended) must draw E003; got {bad:?}"
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
            registries: Arc::default(),
        };
        let src = "namespace eval ::a { proc x {p} { set q $p; return $q } }\n\
                   namespace eval ::b { proc x {p} { set q $p; return $q } }\n\
                   proc top {} { set z 1 }\n";
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
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
            registries: Arc::default(),
        };
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None);
        let src = "proc a {x} { return $x }\nproc b {} { a 1 }\n";
        let count_cu = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .iter()
                .filter(|s| s.contains("compilation_unit"))
                .count()
        };

        // tcl8.6: default == for_dialect, so the two consumers share one build.
        let file86 = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
        let _ = file_analysis_incremental(&db, file86, cfg);
        let _ = compiler_check_diagnostics(&db, file86, cfg);
        assert_eq!(
            count_cu(&log),
            1,
            "tcl8.6: both consumers share exactly one compilation_unit build"
        );

        // tcl8.4: for_dialect disables `{*}` expansion, so the configs differ
        // and each consumer builds its own unit (two executions).
        let file84 = SourceFile::new(&db, src.to_owned(), "tcl8.4".to_owned(), None);
        let _ = file_analysis_incremental(&db, file84, cfg);
        let _ = compiler_check_diagnostics(&db, file84, cfg);
        assert_eq!(
            count_cu(&log),
            2,
            "tcl8.4: differing lexer configs -> a separate build per consumer"
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
