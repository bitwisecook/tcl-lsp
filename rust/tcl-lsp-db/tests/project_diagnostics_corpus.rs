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

//! Corpus-scale multi-file `incremental == fresh` differential for the cross-file
//! diagnostics path (SRV-INCREMENTAL Task 6 verification gate).
//!
//! The in-crate fuzzers (`project_diagnostics_incremental_matches_fresh_under_edits`
//! and `…_both_files_edited`) drive a **two-file** synthetic project through 60
//! edits.  This harness broadens that to **corpus scale**: a project of many real
//! `.tcl` files from `tmp/` plus a synthetic caller/library pair whose cross-file
//! edges (W123 suppression, `E002`/`E003` cross-file arity, and the per-symbol
//! `command_arity` early-cutoff) actually fire, driven through a fuzzed edit
//! sequence.  After every edit the caller's (and a sampled real file's)
//! `project_diagnostics` must equal a from-scratch whole-project rebuild — the
//! gate that catches any untracked read or stale cross-file dependency edge that a
//! small synthetic project would miss (the heuristic-edge risk the track flags).
//!
//! `#[ignore]`d for being a slow corpus sweep — run with `--ignored`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use salsa::Setter as _;
use tcl_compiler::analyser::NonAsciiMode;
use tcl_compiler::analyser::types::Diagnostic;
use tcl_lsp_db::{AnalyserConfig, Project, SourceFile, TclDatabase, project_diagnostics};

mod common;
use common::Progress;

/// Synthetic library / caller function count.
const N_FN: usize = 6;

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
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    // Deterministic order so the fuzz sequence is reproducible run-to-run.
    entries.sort();
    for p in entries {
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

/// `(code, start, end, message)` per diagnostic, in order — the same comparison
/// key the in-crate fuzzers use (the analyser emits deterministically).
fn keys(diags: &[Diagnostic]) -> Vec<(String, u32, u32, String)> {
    diags
        .iter()
        .map(|d| {
            (
                d.code.to_string(),
                d.span.start(),
                d.span.end(),
                d.message.clone(),
            )
        })
        .collect()
}

/// Synthetic library defining `__fuzzlib_fn0..N` (unique tails that no real corpus
/// file declares), with `param_counts[i]` required parameters each.  Editing this
/// drives the cross-file resolution / arity domain.
fn lib_text(param_counts: &[usize]) -> String {
    let mut s = String::new();
    for (i, &n) in param_counts.iter().enumerate() {
        let params: Vec<String> = (0..n).map(|j| format!("p{j}")).collect();
        let _ = writeln!(s, "proc __fuzzlib_fn{i} {{{}}} {{}}", params.join(" "));
    }
    s
}

/// Synthetic caller invoking `__fuzzlib_fn<idx>` with `argc` args — a cross-file
/// call that resolves against the library and is arity-checked against it.
fn caller_text(calls: &[(usize, usize)]) -> String {
    let mut s = String::new();
    for &(idx, argc) in calls {
        let args: Vec<String> = (0..argc).map(|j| j.to_string()).collect();
        let _ = writeln!(s, "__fuzzlib_fn{idx} {}", args.join(" "));
    }
    s
}

/// A whole-project database over the corpus: the synthetic library and
/// caller sit at fixed indices `[0]`/`[1]`, the real files after them.
fn build_project(
    dialect: &str,
    real: &[(String, String)],
    lib: &str,
    caller: &str,
) -> (TclDatabase, AnalyserConfig, Vec<SourceFile>, Project) {
    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
     0,);
    let mut files = vec![
        SourceFile::new(&db, lib.to_owned(), dialect.to_owned(), None),
        SourceFile::new(&db, caller.to_owned(), dialect.to_owned(), None),
    ];
    for (src, dia) in real {
        files.push(SourceFile::new(&db, src.clone(), dia.clone(), None));
    }
    let project = Project::new(&db, files.clone());
    (db, cfg, files, project)
}

#[test]
#[ignore = "slow corpus sweep (~minutes over tmp/); run with --ignored"]
#[allow(clippy::cast_possible_truncation)] // index modulo a tiny array
fn project_diagnostics_incremental_matches_fresh_over_corpus() {
    let dialect = "tcl8.6";
    let mut paths = Vec::new();
    for v in [
        "tcl8.6.16/library",
        "tcllib-2.0/modules",
        "tcl9.0.4/library",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut paths, 200);
    }
    let real: Vec<(String, String)> = paths
        .iter()
        .filter_map(|p| {
            let src = std::fs::read_to_string(p).ok()?;
            (!src.is_empty() && src.len() <= 200_000).then_some((src, dialect.to_owned()))
        })
        .collect();
    assert!(
        real.len() >= 20,
        "expected a real corpus under tmp/ (found {}); fetch with the session hook",
        real.len()
    );

    // xorshift64 — deterministic, no external rng dependency.
    let mut rng = 0x0bad_c0de_1234_5678_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    // Mutable fuzz state: the library's per-fn param counts and the caller's calls.
    let mut params: Vec<usize> = (0..N_FN).map(|i| i % 3).collect();
    let mut calls: Vec<(usize, usize)> = (0..N_FN).map(|i| (i, i % 3)).collect();

    let mut prog = Progress::new("project_diagnostics_fuzz");
    let (mut db, cfg, files, project) =
        build_project(dialect, &real, &lib_text(&params), &caller_text(&calls));
    let lib_file = files[0];
    let caller_file = files[1];
    // A real corpus file sampled for incremental==fresh on the side (catches stale
    // edges in real files whose tails might collide with builtins / each other).
    let sample_real = files[2];

    for iter in 0..120 {
        // Mutate either the library (a signature edit — the cross-file domain) or
        // the caller (call sites / arg counts) — the two-sided fuzz.
        if next().is_multiple_of(2) {
            let i = (next() as usize) % N_FN;
            params[i] = (next() as usize) % 4; // 0..3 required params
            lib_file.set_text(&mut db).to(lib_text(&params));
        } else {
            let i = (next() as usize) % calls.len().max(1);
            let idx = (next() as usize) % N_FN;
            let argc = (next() as usize) % 5; // 0..4 args
            if !calls.is_empty() {
                calls[i] = (idx, argc);
            }
            caller_file.set_text(&mut db).to(caller_text(&calls));
        }

        let inc_caller = keys(&project_diagnostics(&db, caller_file, cfg, project));
        let inc_real = keys(&project_diagnostics(&db, sample_real, cfg, project));

        // Fresh: a brand-new whole-project db with the current texts.
        let (fresh_db, fresh_cfg, fresh_files, fresh_project) =
            build_project(dialect, &real, &lib_text(&params), &caller_text(&calls));
        let fresh_caller = keys(&project_diagnostics(
            &fresh_db,
            fresh_files[1],
            fresh_cfg,
            fresh_project,
        ));
        let fresh_real = keys(&project_diagnostics(
            &fresh_db,
            fresh_files[2],
            fresh_cfg,
            fresh_project,
        ));

        if inc_caller != fresh_caller {
            prog.finding(&format!(
                "iter {iter}: caller incremental != fresh params={params:?} calls={calls:?} inc={inc_caller:?} fresh={fresh_caller:?}"
            ));
        }
        if inc_real != fresh_real {
            prog.finding(&format!(
                "iter {iter}: sampled real-file incremental != fresh"
            ));
        }
        assert_eq!(
            inc_caller, fresh_caller,
            "iter {iter}: caller incremental != fresh\n  params={params:?}\n  calls={calls:?}"
        );
        assert_eq!(
            inc_real, fresh_real,
            "iter {iter}: sampled real-file incremental != fresh"
        );
        prog.tick(iter + 1, 120, "");
    }
    prog.finish("120 iterations");
}
