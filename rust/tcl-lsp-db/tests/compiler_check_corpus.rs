//! **KNOWN-FAILING guard** for a pre-existing `function_lattice` memo divergence.
//!
//! The memoised [`compiler_check_diagnostics`] (offset-0 per-procedure lattice +
//! rebase, via `build_for_memoized`) must be **byte-identical** to a fresh,
//! whole-module [`compiler_check_diagnostics_uncached`] build (`build_for_with_config`)
//! — they are the two halves of the salsa-native lattice graph (#604).  They are
//! **not**: over the `tmp/` corpus ~30% of files diverge (per-file, not a
//! cross-file cache collision — fresh-db-per-file diverges too), surfacing as
//! `S101` (shimmer) check spans of *different width* (so the offset-0 build's
//! analysis genuinely differs from the whole-module build's, beyond offset).
//! See `docs/design/rust/incremental-analysis.md` ("Pre-existing memo
//! byte-identity bug").  Un-ignore once `build_for_memoized` is made byte-identical
//! to `build_for_with_config`.

use std::path::{Path, PathBuf};

use tcl_lsp_db::{
    SourceFile, TclDatabase, TclDb, compiler_check_diagnostics, compiler_check_diagnostics_uncached,
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
#[ignore = "KNOWN-FAILING: pre-existing function_lattice memo divergence (~30% of corpus, S101); see incremental-analysis.md"]
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
        let got = compiler_check_diagnostics(&db, file);
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(&src, &registry, dialect);
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
