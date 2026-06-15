//! Slice-2b corpus gate: `analyse_per_item` must equal `analyse` byte-for-byte
//! over the real-world `tmp/` corpus (the per-item walk decomposition must
//! converge to exactly a full rebuild). Corpus-gated (`--ignored`), mirroring
//! `differential_incremental`.

use std::path::{Path, PathBuf};

use tcl_compiler::analyser::Analyser;

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
#[ignore = "corpus gate; run explicitly with --ignored (needs tmp/ trees)"]
fn per_item_matches_analyse_over_corpus() {
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

    let mut checked = 0usize;
    let mut fellback = 0usize;
    let mut mismatches: Vec<String> = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.len() > 400_000 {
            continue;
        }
        let want = Analyser::new().analyse(&src, dialect);
        let got = Analyser::new().analyse_per_item(&src, dialect);
        checked += 1;
        if !tcl_lexer::script_is_complete(&src) || src.contains("tcl-lsp: stub") {
            fellback += 1;
        }
        if got != want {
            let name = path.file_name().unwrap().to_string_lossy();
            let field = if got.all_procs != want.all_procs {
                "all_procs"
            } else if got.all_classes != want.all_classes {
                "all_classes"
            } else if got.global_scope != want.global_scope {
                "global_scope"
            } else if got.diagnostics != want.diagnostics {
                "diagnostics"
            } else if got.command_invocations != want.command_invocations {
                "command_invocations"
            } else if got.all_variables != want.all_variables {
                "all_variables"
            } else {
                "other"
            };
            if mismatches.len() < 25 {
                mismatches.push(format!("{name}: field={field}"));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "analyse_per_item != analyse in {} / {checked} files (fellback~{fellback}):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!(
        "slice-2b gate: {checked} files, analyse_per_item == analyse ({fellback} trivially-fellback)"
    );
}
