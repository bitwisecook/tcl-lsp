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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use tcl_compiler::cfg_builder::build_cfg_function_with_upvars;
use tcl_compiler::cfg_builder::upvar_info::UpvarInfo;
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit, LatticeRequest};
use tcl_compiler::compiler_checks::Diagnostic as CompilerCheck;
use tcl_compiler::ir::Script;
use tcl_compiler::optimiser::Optimisation;

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
/// interprocedural `param_constants` are built fresh and never interned, so the
/// key needs no `param_constants`.
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
}

/// Memoised offset-0 baseline lattice (CFG → SSA → def-use → SCCP → type →
/// rendered → intra-procedural taint) for one procedure, built from its interned
/// offset-0 body + context.  A body-only edit changes only that procedure's
/// `FnLatticeKey`, so salsa reuses every other procedure's lattice; a shifted
/// body interns to the same key (cache hit).  Rebuilds the CFG via the same
/// `build_cfg_function_with_upvars` call `build_cfg` makes per procedure, so the
/// result equals the whole-module build's unit (modulo offset).  The
/// interprocedural taint re-run still happens at aggregation time
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
    Arc::new(FunctionUnit::build(
        key.qname(db),
        cfg,
        key.params(db),
        &registry,
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
pub fn memoised_compilation_unit<'db>(
    db: &'db dyn TclDb,
    source: &str,
    registry: &CommandRegistry,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
    dialect_opt: Option<&str>,
) -> CompilationUnit {
    let dialect = dialect_opt.unwrap_or("");
    // The module CFG context is the same for every procedure in this build;
    // intern it once on the first request and reuse the id (O(procs), not
    // O(procs²)).
    let mut context: Option<CfgContext<'db>> = None;
    CompilationUnit::build_for_memoized(
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
            );
            (*function_lattice(db, key)).clone()
        },
    )
    .with_interprocedural(registry, dialect_opt)
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
    Arc::new(compiler_diagnostics_from_unit(&cu, &registry, dialect_opt))
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
/// tracked [`file_analysis`].
#[salsa::tracked]
pub fn document_symbols(
    db: &dyn salsa::Database,
    file: SourceFile,
    config: AnalyserConfig,
) -> Vec<DocumentSymbol> {
    let analysis = file_analysis(db, file, config);
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
