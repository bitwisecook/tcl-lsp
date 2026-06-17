//! Backlog #2 validation gate: `optimise_unit_per_function` (each function
//! optimised in an isolated single-function `CompilationUnit` view, then merged
//! and overlap-arbitrated) must be **byte-identical** to the whole-unit
//! `optimise_unit` over the real-world `tmp/` corpus.
//!
//! This pins the "per-function isolation is byte-identical" property the
//! salsa-native per-procedure optimiser memo relies on (the optimiser's only
//! cross-function `PassContext` state — `propagated_*` — is inert in production,
//! `next_group` is canonicalised, and O127's overlap snapshot only matches
//! within one function's own source region).  Corpus-gated (`--ignored`).
//!
//! File size is capped lower than the other corpus gates because the validation
//! path clones the whole `CompilationUnit` once per function (`O(file^2)`); the
//! salsa memo it validates does not clone.

use std::path::{Path, PathBuf};

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::optimiser::{optimise_unit, optimise_unit_per_function};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn registry(dialect: &str) -> tcl_registry::CommandRegistry {
    let mut r = tcl_registry::CommandRegistry::build_default();
    if let Some(d) = tcl_registry::prelude::DialectSet::parse(dialect) {
        r.load_dialect(d);
    }
    r
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
#[ignore = "corpus gate; run explicitly with --ignored (needs tmp/ trees)"]
fn optimise_per_function_matches_whole_unit_over_corpus() {
    let dialect = "tcl8.6";
    let reg = registry(dialect);
    let cfg = tcl_lexer::LexerConfig::for_dialect(dialect);
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

    let mut checked = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Cap: the validation clones the unit per function (O(file^2)).
        if src.is_empty() || src.len() > 60_000 {
            continue;
        }
        let cu = CompilationUnit::build_for_with_config(&src, &reg, false, cfg)
            .with_interprocedural(&reg, Some(dialect));
        let whole = optimise_unit(&cu, &reg, Some(dialect));
        let per_fn = optimise_unit_per_function(&cu, &reg, Some(dialect));
        checked += 1;
        if whole != per_fn && bad.len() < 40 {
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
        "optimise_unit_per_function != optimise_unit in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
