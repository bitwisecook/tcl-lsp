// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Slice-1 corpus gate: `file_decls` must equal the analyser's own declaration
//! sets (`all_procs` / `all_classes` / `command_aliases` / `ensemble_namespaces`)
//! over the real-world `tmp/` corpus.
//!
//! In slice 1 the `item_tree` query is anchored to `Analyser::analyse`, so this
//! holds by construction — but the test is the **permanent guard** that bites
//! when slices 2–3 swap `item_tree` onto a cheap, independent CST extractor.
//! Any divergence between that extractor and `analyse` shows up here as a
//! per-file proc/class/alias/ensemble set mismatch. Corpus-gated (`--ignored`),
//! mirroring `tcl-compiler`'s `differential_incremental`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use tcl_compiler::analyser::{Analyser, ItemTree};

mod common;
use common::Progress;

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
fn file_decls_match_analyse_over_corpus() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.4.20/library",
        "tcl8.5.19/library",
        "tcl8.6.18/library",
        "tcl9.0.4/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("file_decls_gate");
    let mut checked = 0usize;
    let mut mismatches: Vec<String> = Vec::new();
    for (idx, path) in files.iter().enumerate() {
        let file_label = path.file_name().unwrap().to_string_lossy().into_owned();
        if (idx + 1) % 50 == 0 || idx + 1 == files.len() {
            prog.tick(start0 + idx + 1, total, &format!("last={file_label}"));
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.len() > 400_000 {
            continue;
        }
        // GOT: the cheap structure-only extractor (what `item_tree` uses).
        let mut so = Analyser::new().structure_only();
        let so_result = so.analyse(&src, dialect);
        let decls = ItemTree::from_analysis(&so_result, &so.ensemble_namespaces).file_decls();

        // WANT: a full `analyse`'s declaration sets — the authority.
        let mut full = Analyser::new();
        let result = full.analyse(&src, dialect);
        let want_procs: BTreeSet<String> = result.all_procs.keys().cloned().collect();
        let want_classes: BTreeSet<String> = result.all_classes.keys().cloned().collect();
        let want_aliases: BTreeSet<String> = result.command_aliases.keys().cloned().collect();
        let want_ensembles: BTreeSet<String> = full.ensemble_namespaces.iter().cloned().collect();
        checked += 1;

        let name = path.file_name().unwrap().to_string_lossy();
        let mut report = |field: &str, got: &BTreeSet<String>, want: &BTreeSet<String>| {
            if got != want {
                let only_got: Vec<_> = got.difference(want).take(4).cloned().collect();
                let only_want: Vec<_> = want.difference(got).take(4).cloned().collect();
                let m =
                    format!("{name} {field}: only_decls={only_got:?} only_analyse={only_want:?}");
                prog.finding(&m);
                if mismatches.len() < 20 {
                    mismatches.push(m);
                }
            }
        };
        report("procs", &decls.procs, &want_procs);
        report("classes", &decls.classes, &want_classes);
        report("aliases", &decls.aliases, &want_aliases);
        report("ensembles", &decls.ensembles, &want_ensembles);
    }
    prog.finish(&format!("{checked} files, {} mismatches", mismatches.len()));

    assert!(
        mismatches.is_empty(),
        "file_decls != analyse decl sets in {} files / {checked} checked:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!("slice-1 gate: {checked} files, file_decls == analyse decl sets");
}
