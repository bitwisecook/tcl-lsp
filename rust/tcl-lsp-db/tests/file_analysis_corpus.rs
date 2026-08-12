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
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, file_analysis, file_analysis_incremental,
};

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

// Regression (Codex review on #739, P2): a command alias declared *outside* any
// body (`interp alias {} = {} expr`) populates the lowerer's alias table that
// `resolve_alias` consults while lowering every body — but the isolated body-cache
// lowering starts with an empty table, so a cached body resolves `=` as an unknown
// command instead of `expr`. The file-level `source_may_alias_commands` guard makes
// such a file forgo the cache; this pins that the memoised path matches a fresh
// build (the hazard itself — that the cache *would* diverge — is proved at the IR
// level in `tcl_compiler::lowering::body_cache_eligible_tests`).
#[test]
fn alias_declared_outside_body_matches_full() {
    let src = "interp alias {} = {} expr\nproc f {x} { return [= {$x + 1}] }\n";
    assert!(
        tcl_compiler::lowering::source_may_alias_commands(src),
        "file with a top-level `interp alias` must be flagged so it forgoes the body cache"
    );
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
    );
    let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let inc = file_analysis_incremental(&db, file, cfg);
    let full = file_analysis(&db, file, cfg);
    assert_eq!(
        inc.diagnostics, full.diagnostics,
        "incremental (memoised) analysis must match a fresh build when an alias is in scope"
    );
}

// Same hazard as `alias_declared_outside_body_matches_full`, but for a static
// `rename` (taint-limitations investigation #1): `rename puts myputs` at the
// top level populates the same lowerer alias table `interp alias` does, so a
// file that renames a command outside any body must equally forgo the
// isolated body cache.
#[test]
fn rename_declared_outside_body_matches_full() {
    let src = "rename puts myputs\nproc f {x} { myputs $x }\n";
    assert!(
        tcl_compiler::lowering::source_may_alias_commands(src),
        "file with a top-level `rename` must be flagged so it forgoes the body cache"
    );
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
    );
    let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let inc = file_analysis_incremental(&db, file, cfg);
    let full = file_analysis(&db, file, cfg);
    assert_eq!(
        inc.diagnostics, full.diagnostics,
        "incremental (memoised) analysis must match a fresh build when a rename is in scope"
    );
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
        "tcl9.0.4/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    // Per-file analysis is independent — the original shared one `TclDatabase`
    // across files but each `SourceFile` is a distinct input analysed once for
    // `inc` and once for `full`, so there is no cross-file memo reuse to lose.
    // Give each parallel task its own database (salsa inputs are mutated on
    // creation, so a fresh db per task avoids any shared-state contention).
    let (files, start0, total) = Progress::slice(&files);
    let prog = Mutex::new(Progress::new("file_analysis_gate"));
    let done = AtomicUsize::new(start0);
    let outcomes: Vec<Option<String>> = files
        .par_iter()
        .map(|path| {
            let name = path
                .strip_prefix(repo_root())
                .unwrap_or(path)
                .display()
                .to_string();
            let bad = (|| {
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
                    None,
                    None,
                    0,
                );
                let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
                let inc = file_analysis_incremental(&db, file, cfg);
                let full = file_analysis(&db, file, cfg);
                (inc.diagnostics != full.diagnostics).then(|| {
                    format!(
                        "{name}: diagnostics {}->{}",
                        full.diagnostics.len(),
                        inc.diagnostics.len()
                    )
                })
            })();
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            {
                let mut p = prog.lock().unwrap();
                if let Some(b) = &bad {
                    p.finding(b);
                }
                if n % 25 == 0 || n == total {
                    p.tick(n, total, &format!("last={name}"));
                }
            }
            bad
        })
        .collect();

    let checked = outcomes.len();
    let bad: Vec<String> = outcomes.into_iter().flatten().take(40).collect();
    prog.into_inner()
        .unwrap()
        .finish(&format!("{checked} files, {} mismatches", bad.len()));

    assert!(
        bad.is_empty(),
        "file_analysis_incremental != file_analysis in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
