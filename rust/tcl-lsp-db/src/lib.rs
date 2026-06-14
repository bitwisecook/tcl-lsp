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

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
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
}
