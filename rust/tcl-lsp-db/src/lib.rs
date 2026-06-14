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
use std::sync::{Arc, Mutex, OnceLock};

use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};

use tcl_compiler::analyser::per_item::{analyse_proc_body_isolated, BodyFragment, DeferredBody};
use tcl_compiler::analyser::{
    Analyser, AnalysisResult, FileDecls, ItemSig, ItemTree, NonAsciiMode,
};
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_lsp_core::document_symbols::DocumentSymbol;
use tcl_lsp_core::folding::FoldingRange;
use tcl_lsp_core::semantic_tokens::SemanticTokens;
use tcl_registry::dialects::DialectSet;
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
pub fn item_body_analysis<'db>(
    db: &'db dyn salsa::Database,
    key: ItemBodyKey<'db>,
) -> Arc<BodyFragment> {
    // The isolated analysis works at offset 0 and ignores `body_tok` / scope
    // path (the aggregator supplies the real position when grafting), so a
    // placeholder token is fine.
    let body = DeferredBody {
        body_text: key.body_text(db).clone(),
        body_tok: tcl_lexer::Token::new(tcl_lexer::TokenType::Str, tcl_lexer::Span::new(0, 0)),
        scope_path: Vec::new(),
        is_method: false,
        namespace: key.namespace(db).clone(),
        scope_name: key.scope_name(db).clone(),
        params: key.params(db).clone(),
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

/// Process-wide content-addressed cache of per-procedure lattices (slice 4).
///
/// Keyed by the procedure's position-independent content key (body source +
/// module-context digest + params + interprocedural `param_constants` + dialect
/// — see `CompilationUnit::build_for_memoized`); the value carries the build
/// offset so a shifted-but-unchanged body is rebased to its new position.  The
/// map is **idempotent** (a key fully determines its `FunctionUnit`), so reading
/// and populating it inside the otherwise-pure [`file_analysis_incremental`]
/// query is sound — exactly like the durable `registries` cache — and the
/// resulting `AnalysisResult` is byte-identical whether entries hit or miss.
fn lattice_cache() -> &'static Mutex<HashMap<String, (u32, FunctionUnit)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (u32, FunctionUnit)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Bound on [`lattice_cache`] entries; cleared wholesale when exceeded (a miss
/// just rebuilds the byte-identical unit, so eviction is always safe).
const LATTICE_CACHE_CAP: usize = 16_384;

/// Build a `CompilationUnit` (with interprocedural summary applied) whose
/// per-procedure lattices are memoised in the process-wide [`lattice_cache`].
///
/// Shared by the analyser's CFG/SSA diagnostic tail and the optimiser's
/// compiler-checks pass so an unchanged procedure's lattice is built once and
/// reused (rebased to its new offset) across edits *and* across both
/// consumers' passes.  Byte-identical to
/// [`CompilationUnit::build_for_with_config`] `+ with_interprocedural`.
///
/// **`cache_ns` must uniquely encode the `config`** (the two consumers lower
/// with different [`tcl_lexer::LexerConfig`]s, which can change a `{*}`/`}{`
/// body's IR): callers pass distinct namespaces so entries never cross-pollute.
#[must_use]
pub fn memoised_compilation_unit(
    source: &str,
    registry: &CommandRegistry,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
    cache_ns: &str,
    dialect_opt: Option<&str>,
) -> CompilationUnit {
    CompilationUnit::build_for_memoized(
        source,
        registry,
        defer_top_level,
        config,
        cache_ns,
        &mut |key: &str, build: &mut dyn FnMut() -> (u32, FunctionUnit)| {
            if let Some(entry) = lattice_cache()
                .lock()
                .expect("lattice cache poisoned")
                .get(key)
            {
                return entry.clone();
            }
            let entry = build();
            let mut map = lattice_cache().lock().expect("lattice cache poisoned");
            if map.len() >= LATTICE_CACHE_CAP {
                map.clear();
            }
            map.insert(key.to_owned(), entry.clone());
            entry
        },
    )
    .with_interprocedural(registry, dialect_opt)
}

/// Incremental whole-file analysis: the per-item path with each `proc` body's
/// isolated analysis memoised via [`item_body_analysis`], so a body edit
/// recomputes one body + the cheap shell instead of the whole walk; the
/// CFG/SSA diagnostic tail's per-procedure lattices are likewise memoised via
/// [`lattice_cache`] + `CompilationUnit::build_for_memoized` (slice 4), so an
/// unchanged procedure's lattice is reused (and rebased) instead of rebuilt.
/// Byte-identical to [`file_analysis`] (and `analyse`) — proven by the
/// `per_item_corpus` gate over the shared `analyse_per_item_with` orchestration.
#[salsa::tracked]
pub fn file_analysis_incremental(
    db: &dyn salsa::Database,
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
    // memoised, and feed it through the analyser's `cu_override` seam.  The
    // registry + default lexer config mirror what `emit_cfg_ssa_diagnostics`
    // builds for itself, so the supplied unit is the one it would otherwise
    // build.  Cache namespace `"a:"` (analyser, default lexer config) is
    // distinct from the optimiser's so the two never cross-pollute.
    let mut registry = CommandRegistry::build_default();
    let dialect_opt = (!dialect.is_empty()).then_some(dialect.as_str());
    if let Some(d) = DialectSet::parse(&dialect) {
        registry.load_dialect(d);
    }
    let cu = memoised_compilation_unit(
        &text,
        &registry,
        false,
        tcl_lexer::LexerConfig::default(),
        &format!("a:{dialect}"),
        dialect_opt,
    );
    analyser.set_cu_override(Arc::new(cu));

    let mut body_fn = |body: &DeferredBody| -> BodyFragment {
        let key = ItemBodyKey::new(
            db,
            body.body_text.clone(),
            body.namespace.clone(),
            body.scope_name.clone(),
            body.params.clone(),
            dialect.clone(),
            disabled_vec.clone(),
            non_ascii,
        );
        (*item_body_analysis(db, key)).clone()
    };
    Arc::new(analyser.analyse_per_item_with(&text, &dialect, &mut body_fn))
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
        assert!(file_analysis(&db, file, config)
            .all_procs
            .contains_key("::a"));

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
}
