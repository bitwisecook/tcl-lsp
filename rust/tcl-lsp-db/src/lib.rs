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
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit, LatticeRequest};
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
    Analyser, AnalysisResult, FileDecls, ItemSig, ItemTree, NonAsciiMode,
};
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_lsp_core::document_symbols::DocumentSymbol;
use tcl_lsp_core::folding::FoldingRange;
use tcl_lsp_core::semantic_tokens::SemanticTokens;
use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;

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
}

/// Whole-file analysis — the `AnalysisResult` every feature provider already
/// consumes, behind an `Arc` so reads bump a refcount rather than deep-clone.
///
/// Wraps [`Analyser::analyse`] unchanged.  Per-item firewall granularity is a
/// later step in this phase; this is the coarse foundation query.
#[salsa::tracked]
pub fn file_analysis(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Arc<AnalysisResult> {
    let disabled: HashSet<String> = config.disabled_diagnostics(db).iter().cloned().collect();
    let mut analyser = Analyser::with_disabled_diagnostics(disabled)
        .with_non_ascii_mode(config.non_ascii_mode(db));
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
/// (`max == usize::MAX` ⇒ a trailing `args` makes it unbounded).  Required params
/// (no default, excluding a trailing `args`) set the minimum; the rest are
/// optional.
fn proc_arity(params: &[tcl_compiler::signature_scan::types::ParamDef]) -> (usize, usize) {
    let has_args = params.last().is_some_and(|p| p.name == "args");
    let counted = if has_args {
        &params[..params.len() - 1]
    } else {
        params
    };
    let min = counted.iter().filter(|p| !p.has_default).count();
    let max = if has_args { usize::MAX } else { counted.len() };
    (min, max)
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
    argc: usize,
    candidates: &[(usize, usize)],
) -> Option<tcl_compiler::analyser::types::Diagnostic> {
    use tcl_compiler::analyser::types::{Diagnostic, Severity};
    let min = candidates.iter().map(|&(lo, _)| lo).min()?;
    let max = candidates.iter().map(|&(_, hi)| hi).max()?;
    let (code, message) = if argc < min {
        (
            DiagCode::E002,
            format!("Too few arguments for '{name}': expected at least {min}, got {argc}"),
        )
    } else if max != usize::MAX && argc > max {
        (
            DiagCode::E003,
            format!("Too many arguments for '{name}': expected at most {max}, got {argc}"),
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
            && let Some(diag) = cross_file_arity_diagnostic(name, *span, *argc, candidates)
            && !is_disabled(diag.code.as_str())
        {
            out.push(diag);
        }
    }
    out
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
    let arities = project_command_arities(db, project);
    let disabled = config.disabled_diagnostics(db);
    // `file_analysis_incremental` (not the coarse `file_analysis`) so this reuses
    // the per-item firewall result the diagnostics worker already computed for
    // `file` — a cache hit, not a second whole-file analysis.
    let analysis = file_analysis_incremental(db, file, config);
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

/// Interned module-wide CFG context (`upvar_procs` + `proc_params` from
/// `prepare_cfg_context`), the context a procedure body's CFG is built under.
/// Interned once per build and shared by every [`FnLatticeKey`] so a procedure's
/// key stays small and the per-build interning cost is `O(procs)`, not
/// `O(procs²)`.  The entry vecs are sorted by name before interning so an equal
/// context (regardless of hash-map iteration order) yields the same id.
#[salsa::interned]
pub struct CfgContext<'db> {
    #[returns(ref)]
    pub upvar_ctx: Vec<(String, UpvarInfo)>,
    #[returns(ref)]
    pub proc_params: Vec<(String, Vec<String>)>,
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
    let registry = db.registry(key.dialect(db));
    let cfg = build_cfg_function_with_upvars(key.qname(db), key.body(db), true, upvar, proc_params);
    let param_constants =
        tcl_compiler::compilation_unit::decode_param_constants(key.param_constants(db));
    let known_classes: HashSet<String> = key.known_classes(db).iter().cloned().collect();
    Arc::new(FunctionUnit::build_with_param_constants_and_classes(
        key.qname(db),
        cfg,
        key.params(db),
        &registry,
        param_constants.as_ref(),
        &known_classes,
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
    let cu = CompilationUnit::build_for_memoized(
        source,
        registry,
        defer_top_level,
        config,
        dialect,
        &mut |req: &LatticeRequest<'_>| -> FunctionUnit {
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
                CfgContext::new(db, upvar, proc_params)
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
            );
            lattice_keys.insert(req.qname.to_owned(), key);
            (*function_lattice(db, key)).clone()
        },
    );
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
fn summary_deps_key<'db>(
    db: &'db dyn TclDb,
    qname: &str,
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
        for callee in &calls {
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
    Arc::new(tcl_compiler::compiler_checks::function_nontaint_checks(
        &fu,
        &registry,
        dialect_opt,
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
                let deps_key = summary_deps_key(db, qname, interproc, summaries, known, &dialect);
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
    // **offset-0** spans (it runs on the offset-0 `function_lattice` unit), so we
    // add the procedure's `body_offset` here — the same rebase the whole-module
    // build's `rebase_function_unit` applies.  A proc without a lattice key (e.g.
    // the top level, or a complexity-guarded body) falls back to the direct
    // per-function computation on the *already-rebased* built unit (no offset add).
    let mut fn_checks: Vec<CompilerCheck> = Vec::new();
    for fu in cu.analysable_functions() {
        match lattice_keys.get(&fu.name) {
            Some(&key) => {
                let body_offset = cu
                    .ir_module
                    .procedures
                    .get(&fu.name)
                    .map_or(0, |p| p.span.start());
                for d in function_checks(db, key).iter() {
                    let mut d = d.clone();
                    // Rebase real spans by the procedure's offset, but leave the
                    // `(0, 0)` "unknown span" sentinel alone: a spanless check (an
                    // O100 constant branch whose `cb.span` is `None`) renders to
                    // `(0, 0)` in *both* paths — the whole-module build rebases the
                    // `Option<Span>` (so `None` stays `None`) *before* the
                    // `None → (0,0)` lowering, so the offset must not be added here.
                    if d.span.start() != 0 || d.span.end() != 0 {
                        d.span = tcl_lexer::Span::new(
                            d.span.start() + body_offset,
                            d.span.end() + body_offset,
                        );
                    }
                    fn_checks.push(d);
                }
            }
            None => {
                // The built unit's fallback fus (complexity-guarded / top level)
                // carry **absolute** spans already (`base_offset == 0`), so the
                // per-function checks need no rebase.
                for d in tcl_compiler::compiler_checks::function_nontaint_checks(
                    fu,
                    &registry,
                    dialect_opt,
                ) {
                    fn_checks.push(d);
                }
            }
        }
    }

    Arc::new(CheckSolve { taints, fn_checks })
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
    let dialect = file.dialect(db).clone();
    let text = file.text(db).clone();
    let mut analyser = Analyser::with_disabled_diagnostics(disabled_vec.iter().cloned().collect())
        .with_non_ascii_mode(non_ascii);

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
) -> CompilerDiagnostics {
    CompilerDiagnostics {
        checks: tcl_compiler::compiler_checks::run_all_checks(cu, registry, dialect_opt),
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
pub fn compiler_check_diagnostics(db: &dyn TclDb, file: SourceFile) -> Arc<CompilerDiagnostics> {
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
    tcl_compiler::compiler_checks::push_taint_and_module_checks(
        &cu,
        &registry,
        dialect_opt,
        &solve.taints,
        &mut checks,
    );
    tcl_compiler::compiler_checks::sort_diagnostics(&mut checks);
    Arc::new(CompilerDiagnostics {
        checks,
        optimisations: tcl_compiler::optimiser::optimise_unit(&cu, &registry, dialect_opt),
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
) -> CompilerDiagnostics {
    let dialect_opt = (!dialect.is_empty()).then_some(dialect);
    let cu = CompilationUnit::build_for_with_config(
        text,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    )
    .with_interprocedural(registry, dialect_opt);
    compiler_diagnostics_from_unit(&cu, registry, dialect_opt)
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

/// Semantic tokens — wraps `semantic_tokens::full`; reads the durable registry.
#[salsa::tracked]
pub fn semantic_tokens(db: &dyn TclDb, file: SourceFile) -> SemanticTokens {
    let registry = db.registry(file.dialect(db));
    tcl_lsp_core::semantic_tokens::full(file.text(db), file.dialect(db), &registry)
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
        AnalyserConfig::new(db, Vec::new(), NonAsciiMode::Default)
    }

    const SRC: &str = "proc greet {name} {\n    puts \"hi $name\"\n}\n# c\nset x 1\n";

    #[test]
    fn file_analysis_matches_direct_analyse() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned());
        let got = file_analysis(&db, file, cfg(&db));

        let mut direct = Analyser::new();
        let expected = direct.analyse(SRC, "tcl");
        assert_eq!(*got, expected);
        assert!(got.all_procs.contains_key("::greet"));
    }

    #[test]
    fn document_symbols_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned());
        let got = document_symbols(&db, file, cfg(&db));
        let expected = tcl_lsp_core::document_symbols::document_symbols(SRC, "tcl");
        assert_eq!(got, expected);
    }

    #[test]
    fn semantic_tokens_match_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned());
        let got = semantic_tokens(&db, file);
        let reg = db.registry("tcl");
        let expected = tcl_lsp_core::semantic_tokens::full(SRC, "tcl", &reg);
        assert_eq!(got, expected);
        assert!(!got.data.is_empty());
    }

    #[test]
    fn folding_matches_direct() {
        let db = TclDatabase::default();
        let file = SourceFile::new(&db, SRC.to_owned(), "tcl".to_owned());
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
        let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned());
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
        let file = SourceFile::new(&db, src.to_owned(), "tcl".to_owned());
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
        let file = SourceFile::new(&db, "proc greet {name} {}\n".to_owned(), "tcl".to_owned());
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
        let file = SourceFile::new(&db, "proc a {} {}\n".to_owned(), "tcl".to_owned());
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
            let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned());
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
            "tcl8.6".to_owned(),
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let file = SourceFile::new(
            &db,
            "oo::class create K {\n  method a {} { set x 11111 }\n  method b {} { set y 22222 }\n}\n"
                .to_owned(),
            "tcl8.6".to_owned(),
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
            let file = SourceFile::new(&db, src.to_owned(), dialect.to_owned());
            let got = compiler_check_diagnostics(&db, file);
            let registry = db.registry(dialect);
            let want = compiler_check_diagnostics_uncached(src, &registry, dialect);
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
        let file = SourceFile::new(&db, versions[0].to_owned(), dialect.to_owned());
        for src in versions {
            file.set_text(&mut db).to(src.to_owned());
            let got = compiler_check_diagnostics(&db, file);
            let want = compiler_check_diagnostics_uncached(src, &registry, dialect);
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
        );
        let _ = compiler_check_diagnostics(&db, file);
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
        let _ = compiler_check_diagnostics(&db, file);
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
            registries: Arc::default(),
        };
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
        );
        let _ = compiler_check_diagnostics(&db, file);
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
        let _ = compiler_check_diagnostics(&db, file);
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
        );
        let b = SourceFile::new(
            &db,
            "proc b {} { set y 2 }\n".to_owned(),
            "tcl8.6".to_owned(),
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
        );
        let b = SourceFile::new(&db, "proc other {} {}\n".to_owned(), "tcl8.6".to_owned());
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let a = SourceFile::new(&db, "helper foo bar\n".to_owned(), "tcl8.6".to_owned());
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
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
    #[test]
    #[allow(clippy::cast_possible_truncation)] // index modulo a tiny array
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
            let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned());
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned());
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned());
        let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned());
        let project = Project::new(&db, vec![a, b]);

        let mut rng = 0x1234_5678_9abc_def0_u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for _ in 0..60 {
            let b_text = b_variants[(next() % b_variants.len() as u64) as usize];
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
    #[allow(clippy::cast_possible_truncation)] // index modulo a tiny array
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
            let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
            let a = SourceFile::new(&db, a_text.to_owned(), "tcl8.6".to_owned());
            let b = SourceFile::new(&db, b_text.to_owned(), "tcl8.6".to_owned());
            let project = Project::new(&db, vec![a, b]);
            diag_keys(&project_diagnostics(&db, a, cfg, project))
        };

        let mut db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let a = SourceFile::new(&db, a_variants[0].to_owned(), "tcl8.6".to_owned());
        let b = SourceFile::new(&db, b_variants[0].to_owned(), "tcl8.6".to_owned());
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
            let a_text = a_variants[(next() % a_variants.len() as u64) as usize];
            let b_text = b_variants[(next() % b_variants.len() as u64) as usize];
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        // B defines `proc helper {x y}` — arity exactly 2.
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
        );
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags
                .iter()
                .any(|d| d.code.as_str() == code && d.message.contains("helper"))
        };

        // 3 args to a 2-param proc → E003 (too many), and the W123 is suppressed.
        let a3 = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned());
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
        let a2 = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned());
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        // B: a class `Widget` AND a proc whose tail is also `Widget` (arity 1).
        let b = SourceFile::new(
            &db,
            "oo::class create Widget { method draw {} {} }\n\
             namespace eval ns { proc Widget {x} {} }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
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
        let a = SourceFile::new(&db, "Widget new extra\n".to_owned(), "tcl8.6".to_owned());
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
        );
        let a = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned());
        let proj = Project::new(&db, vec![a, b]);
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags
                .iter()
                .any(|d| d.code.as_str() == code && d.message.contains("helper"))
        };

        // E003 enabled (default): wrong arity surfaces as E003, W123 suppressed.
        let cfg_on = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let d_on = project_diagnostics(&db, a, cfg_on, proj);
        assert!(has(&d_on, "E003"), "baseline: E003 present when enabled");
        assert!(!has(&d_on, "W123"), "baseline: W123 suppressed (resolved)");

        // E003 disabled (W123 left enabled): no E003, W123 still suppressed — the
        // call genuinely resolves cross-file regardless of the arity-code toggle.
        let cfg_off = AnalyserConfig::new(&db, vec!["E003".to_owned()], NonAsciiMode::Default);
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
        );
        let has = |diags: &[tcl_compiler::analyser::types::Diagnostic], code: &str| {
            diags.iter().any(|d| d.code.as_str() == code)
        };
        // W123 disabled, E003 left enabled.
        let cfg = AnalyserConfig::new(&db, vec!["W123".to_owned()], NonAsciiMode::Default);

        // Wrong arity (3 args to a 2-param proc) → E003 still fires; no W123.
        let bad = SourceFile::new(&db, "helper a b c\n".to_owned(), "tcl8.6".to_owned());
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
        let ok = SourceFile::new(&db, "helper a b\n".to_owned(), "tcl8.6".to_owned());
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

    /// Cross-file classes: a command resolving to a
    /// class (the class command) defined in another project file is resolved —
    /// W123 suppressed — and, being a non-proc, never draws an arity error.
    #[test]
    fn project_diagnostics_resolves_cross_file_class() {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        // B defines a TclOO class `Widget`.
        let b = SourceFile::new(
            &db,
            "oo::class create Widget { method draw {} {} }\n".to_owned(),
            "tcl8.6".to_owned(),
        );
        // A invokes the `Widget` class command (defined cross-file).
        let a = SourceFile::new(&db, "Widget new\n".to_owned(), "tcl8.6".to_owned());
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let b = SourceFile::new(
            &db,
            "proc two {a b} {}\nproc opt {a {b 1}} {}\nproc variadic {a args} {}\nproc none {} {}\n"
                .to_owned(),
            "tcl8.6".to_owned(),
        );
        // The cross-file arity code drawn by calling `src` (file A over {A, B}):
        // Some("E002") too few, Some("E003") too many, None if it fits.
        let arity_code = |src: &str| -> Option<String> {
            let a = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned());
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

    /// Conservative arity: a `{*}`-expanded call has an
    /// unknown runtime arg count (`argc == None`), so it must never draw an arity
    /// error even though its literal word count looks wrong — while still resolving
    /// (no W123).
    #[test]
    fn cross_file_arity_skips_expanded_call() {
        let db = TclDatabase::default();
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let b = SourceFile::new(&db, "proc two {a b} {}\n".to_owned(), "tcl8.6".to_owned());
        // `two {*}$lst` — one literal word, `{*}`-expanded → runtime arity unknown.
        let a = SourceFile::new(
            &db,
            "set lst {1 2 3}\ntwo {*}$lst\n".to_owned(),
            "tcl8.6".to_owned(),
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let b = SourceFile::new(
            &db,
            "proc helper {x y} { return $x }\n".to_owned(),
            "tcl8.6".to_owned(),
        );
        // `helper` called with 3 args inside a `[…]` substitution → too many.
        let a = SourceFile::new(
            &db,
            "set x [helper a b c]\n".to_owned(),
            "tcl8.6".to_owned(),
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned());
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
            "tcl8.6".to_owned(),
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        // Four independent procedures; we edit `b`'s body and leave a, c, d alone.
        let file = SourceFile::new(
            &db,
            "proc a {} { set x 11111 }\n\
             proc b {} { set y 22222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
        );
        let _ = file_analysis_incremental(&db, file, cfg);
        let _ = compiler_check_diagnostics(&db, file);
        log.lock().unwrap().clear();

        // Length-changing body edit to ONE procedure (`b`); a/c/d shift but their
        // offset-0 lattice keys are unchanged (cache hits).
        file.set_text(&mut db).to("proc a {} { set x 11111 }\n\
             proc b {} { set y 222222222 }\n\
             proc c {} { set z 33333 }\n\
             proc d {} { set w 44444 }\n"
            .to_owned());
        let _ = file_analysis_incremental(&db, file, cfg);
        let _ = compiler_check_diagnostics(&db, file);
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        // `other` first so editing it shifts the two below; `target` takes a
        // param `caller` always passes the literal `42` for -> param_constants.
        let file = SourceFile::new(
            &db,
            "proc other {} { set z 1 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
                .to_owned(),
            "tcl8.6".to_owned(),
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
        let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        let src = "proc a {x} { return $x }\nproc b {} { a 1 }\n";
        let count_cu = |log: &Arc<Mutex<Vec<String>>>| {
            std::mem::take(&mut *log.lock().unwrap())
                .iter()
                .filter(|s| s.contains("compilation_unit"))
                .count()
        };

        // tcl8.6: default == for_dialect, so the two consumers share one build.
        let file86 = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned());
        let _ = file_analysis_incremental(&db, file86, cfg);
        let _ = compiler_check_diagnostics(&db, file86);
        assert_eq!(
            count_cu(&log),
            1,
            "tcl8.6: both consumers share exactly one compilation_unit build"
        );

        // tcl8.4: for_dialect disables `{*}` expansion, so the configs differ
        // and each consumer builds its own unit (two executions).
        let file84 = SourceFile::new(&db, src.to_owned(), "tcl8.4".to_owned());
        let _ = file_analysis_incremental(&db, file84, cfg);
        let _ = compiler_check_diagnostics(&db, file84);
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
        let _ = compiler_check_diagnostics(&db, file86);
        assert_eq!(
            count_cu(&log),
            1,
            "after an edit, tcl8.6 again shares one build across both consumers"
        );
    }
}
