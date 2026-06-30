//! Byte-identity guard for the `function_lattice` memo path.
//!
//! The memoised [`compiler_check_diagnostics`] (offset-0 per-procedure lattice +
//! rebase, via `build_for_memoized`) must be **byte-identical** to a fresh,
//! whole-module [`compiler_check_diagnostics_uncached`] build
//! (`build_for_with_config`) — they are the two halves of the salsa-native
//! lattice graph.
//!
//! This previously diverged for ~30% of the `tmp/` corpus, but the divergence
//! was *nondeterminism*, not an offset-0-vs-whole-module analysis difference:
//! several diagnostic producers folded over `HashMap`s in iteration order
//! (phi-span resolution, `run_all_checks` output order, the optimiser's group-id
//! allocation, W313's "first offending path variable", and the order-sensitive
//! `type_join` fold over a phi's predecessors).  Each is now deterministic, so
//! the memo and whole-module builds agree byte-for-byte.  See
//! `docs/design/rust/incremental-analysis.md` ("Memo byte-identity").

use std::path::{Path, PathBuf};

use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    compiler_check_diagnostics_uncached,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A default `AnalyserConfig` (no overrides) for the memo-vs-uncached
/// differential — both sides then use the built-in generic-name patterns.
fn default_config(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
    )
}

fn gather(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            gather(&p, out, cap);
        } else if p.extension().is_some_and(|x| x == "tcl") {
            out.push(p);
            if out.len() >= cap {
                return;
            }
        }
    }
}

/// Focused regression for the `proc_summary_cascade` memo: `graphops.tcl`'s
/// `::struct::graph::op::distance` tripped the debug fixpoint guard when the
/// memoised `proc_summary_cascade` reconstructed only the *reachable* summaries
/// — a resolved callee that was absent (rather than present-and-clean) made
/// `propagate_taints` fall through to its conservative bare-argument join and
/// over-taint.  Fast (one file), runs in debug so the guard is live.
#[test]
fn compiler_check_memo_matches_uncached_graphops() {
    let dialect = "tcl8.6";
    let path = repo_root().join("tmp/tcllib-2.0/modules/struct/graphops.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned());
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, &registry, dialect, None);
    assert_eq!(
        got.checks, want.checks,
        "graphops checks diverge (memo vs uncached)"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "graphops optimisations diverge (memo vs uncached)"
    );
}

/// Focused regression for the per-function check memo:
/// `init.tcl` has `foreach` loops whose constant-true condition emits an O100
/// with a **`None`** span → the `(0, 0)` "unknown" sentinel.  The per-proc memo
/// returns offset-0 spans rebased by `body_offset`, but that sentinel must be
/// left alone (the whole-module build rebases the `Option<Span>` before lowering,
/// so `None` stays `(0, 0)`).  A blanket `+ body_offset` regressed this; fast
/// (one file) so it pins the fix without the corpus sweep.
#[test]
fn compiler_check_memo_matches_uncached_init() {
    let dialect = "tcl8.6";
    let path = repo_root().join("tmp/tcl8.6.16/library/init.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned());
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, &registry, dialect, None);
    assert_eq!(
        got.checks, want.checks,
        "init.tcl checks diverge (memo vs uncached)"
    );
}

#[test]
#[ignore = "slow corpus sweep (~100s over tmp/); run with --ignored"]
fn compiler_check_memo_matches_uncached_over_corpus() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.4.20/library",
        "tcl8.5.19/library",
        "tcl8.6.16/library",
        "tcl9.0.3/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    let db = TclDatabase::default();
    let mut checked = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.is_empty() || src.len() > 400_000 {
            continue;
        }
        let file = SourceFile::new(&db, src.clone(), dialect.to_owned());
        let got = compiler_check_diagnostics(&db, file, default_config(&db));
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(&src, &registry, dialect, None);
        checked += 1;
        if (got.checks != want.checks || got.optimisations != want.optimisations) && bad.len() < 40
        {
            bad.push(
                path.strip_prefix(repo_root())
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        bad.is_empty(),
        "compiler_check_diagnostics (memo) != uncached in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
