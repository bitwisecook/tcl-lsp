//! Byte-identity guard for the analyser tail on the memoised CU path.
//!
//! [`file_analysis_incremental`] consumes the memoised, offset-aware
//! `CompilationUnit` (offset-0 per-procedure units + `base_offset`, Approach B),
//! while [`file_analysis`] runs `Analyser::analyse` directly (its own
//! real-position unit).  Their diagnostics must be **byte-identical** over the
//! corpus — this is the gate that pins the analyser tail's `fu.abs_span`
//! conversion after the offset-0 flip (`compiler_check_corpus` covers the
//! optimiser / compiler-checks consumers; this covers the analyser tail).
//!
//! `#[ignore]`d for being a slow corpus sweep — run with `--ignored`.

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, file_analysis, file_analysis_incremental,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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

#[test]
#[ignore = "slow corpus sweep (~100s over tmp/); run with --ignored"]
fn file_analysis_incremental_matches_full_over_corpus() {
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

    // Per-file analysis is independent — the original shared one `TclDatabase`
    // across files but each `SourceFile` is a distinct input analysed once for
    // `inc` and once for `full`, so there is no cross-file memo reuse to lose.
    // Give each parallel task its own database (salsa inputs are mutated on
    // creation, so a fresh db per task avoids any shared-state contention).
    let outcomes: Vec<Option<String>> = files
        .par_iter()
        .filter_map(|path| {
            let src = std::fs::read_to_string(path).ok()?;
            if src.is_empty() || src.len() > 400_000 {
                return None;
            }
            let db = TclDatabase::default();
            let cfg = AnalyserConfig::new(
                &db,
                Vec::new(),
                tcl_compiler::analyser::NonAsciiMode::Default,
                Vec::new(),
            );
            let file = SourceFile::new(&db, src.clone(), dialect.to_owned());
            let inc = file_analysis_incremental(&db, file, cfg);
            let full = file_analysis(&db, file, cfg);
            Some((inc.diagnostics != full.diagnostics).then(|| {
                path.strip_prefix(repo_root())
                    .unwrap_or(path)
                    .display()
                    .to_string()
            }))
        })
        .collect();

    let checked = outcomes.len();
    let bad: Vec<String> = outcomes.into_iter().flatten().take(40).collect();

    assert!(
        bad.is_empty(),
        "file_analysis_incremental != file_analysis in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
