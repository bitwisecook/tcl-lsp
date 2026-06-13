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

use std::collections::HashSet;
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};

/// The Tcl LSP query database.
///
/// Cloneable so a worker thread can run queries against a handle while the
/// main thread sets inputs (the rust-analyzer snapshot pattern).
#[salsa::db]
#[derive(Default, Clone)]
pub struct TclDatabase {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for TclDatabase {}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(db: &TclDatabase) -> AnalyserConfig {
        AnalyserConfig::new(db, Vec::new(), NonAsciiMode::Default)
    }

    #[test]
    fn file_analysis_matches_direct_analyse() {
        let db = TclDatabase::default();
        let src = "proc greet {name} {\n    puts \"hi $name\"\n}\n";
        let file = SourceFile::new(&db, src.to_owned(), "tcl".to_owned());
        let got = file_analysis(&db, file, cfg(&db));

        let mut direct = Analyser::new();
        let expected = direct.analyse(src, "tcl");
        assert_eq!(*got, expected);
        assert!(got.all_procs.contains_key("::greet"));
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
