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

//! Differential fuzzer for incremental analysis (permanent guard).
//!
//! The contract: `analyse_incremental` must converge to *exactly* what a
//! from-scratch `analyse` produces. We apply random edit sequences to corpus
//! files and assert `incremental == fresh` (byte-identical `AnalysisResult`) at
//! every step. Any divergence (when incremental did not fall back to a full
//! walk) is a correctness bug. This is the backbone the per-item rewrite will
//! reuse — extend it as new incremental paths land.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::segmenter::segment_commands;

mod common;
use common::{Progress, describe_analysis_divergence};

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

// Tiny deterministic PRNG (xorshift) so failures reproduce from the seed.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn upto(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            // `next() % (n as u64)` is in `[0, n)`, so it always fits in usize.
            #[allow(clippy::cast_possible_truncation)]
            let bounded = (self.next() % n as u64) as usize;
            bounded
        }
    }
}

/// Apply one random edit, keeping `text` valid UTF-8 (edit on char boundaries).
fn random_edit(text: &str, rng: &mut Rng) -> String {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    if bounds.len() < 2 {
        return format!("{text}\nset x 1\n");
    }
    let pick = |rng: &mut Rng| bounds[rng.upto(bounds.len())];
    match rng.upto(3) {
        0 => {
            // insert a snippet
            let at = pick(rng);
            let snips = [
                "\nset y 2\n",
                " ",
                "# c\n",
                "puts $z",
                "}\nproc q {} {",
                "[",
                "$a",
            ];
            let s = snips[rng.upto(snips.len())];
            format!("{}{}{}", &text[..at], s, &text[at..])
        }
        1 => {
            // delete a small range
            let a = pick(rng);
            let b = pick(rng);
            let (lo, hi) = (a.min(b), a.max(b));
            let hi = (lo + (hi - lo).min(12)).min(text.len());
            let hi = bounds
                .iter()
                .copied()
                .find(|&x| x >= hi)
                .unwrap_or(text.len());
            format!("{}{}", &text[..lo], &text[hi..])
        }
        _ => {
            // replace a char-ish range with a token
            let a = pick(rng);
            let b = pick(rng);
            let (lo, hi) = (a.min(b), a.max(b));
            format!("{}{}{}", &text[..lo], "X", &text[hi..])
        }
    }
}

/// Targeted (non-ignored) guard for the `apply`-lambda namespace fix: a `proc`
/// nested in an apply body must produce a byte-identical `AnalysisResult` on the
/// incremental (per-item / deferred) and fresh (full) paths, whether the apply
/// sits inside a `namespace eval`, pins an explicit lambda namespace, or defines
/// several nested procs. This runs in CI (the corpus fuzzer below needs `tmp/`
/// trees that aren't present in a worktree checkout).
#[test]
fn incremental_matches_fresh_for_apply_lambda_bodies() {
    let dialect = "tcl8.6";
    let sources = [
        "namespace eval foo { apply {{} { proc helper {} { return 1 } }} }\n",
        "namespace eval foo {\n  apply {{x} {\n    proc helper {} { return $x }\n    set y 1\n  }} 5\n}\n",
        "apply {{} { proc helper {} {} } ::bar}\n",
        "namespace eval a::b { apply {{} { proc p {} {} ; proc q {} {} }} }\n",
        "apply {{} { proc helper {} { return 1 } }}\n",
    ];
    for src in sources {
        let mut text = src.to_string();
        let mut cur = segment_commands(&text);
        // A no-op step plus small appends: every step must match a fresh walk.
        for edit in ["", "\nset z 9\n", " "] {
            let new_text = format!("{text}{edit}");
            let inc = Analyser::new().analyse_incremental(&text, &cur, &new_text, dialect);
            let fresh = Analyser::new().analyse(&new_text, dialect);
            assert!(
                inc == fresh,
                "incremental != fresh for apply lambda body:\n{}",
                describe_analysis_divergence(src, &inc, &fresh)
            );
            text = new_text;
            cur = segment_commands(&text);
        }
    }
}

/// Same parity guard for a **dynamically-named** `proc` — the tcllib pki
/// idiom `proc [namespace current]::_x {...}` (tcllib 2.0
/// `modules/pki/pki.tcl`:316) and its `$`-spelled cousins.  The `Scope` name
/// these produce feeds `advance_command_resolution_namespace`, and the
/// deferred (per-item) path builds its scope from `DeferredBody::scope_name`,
/// so the two must derive it identically or the incremental walk homes the
/// body's definitions in a different namespace from the full walk.
#[test]
fn incremental_matches_fresh_for_dynamically_named_procs() {
    let dialect = "tcl8.6";
    let sources = [
        "namespace eval ::pki {\n  proc [namespace current]::_x {} { proc helper {} {} }\n}\n",
        "namespace eval ::pki {\n  proc _outer {} {\n    proc [namespace current]::_x {} { proc helper {} { set v 1 } }\n  }\n}\n",
        "set ns ::pki\nproc ${ns}::_x {} { proc helper {} {} }\n",
        "namespace eval ::pki { proc _$suffix {} { proc helper {} {} } }\n",
    ];
    for src in sources {
        let mut text = src.to_string();
        let mut cur = segment_commands(&text);
        for edit in ["", "\nset z 9\n", " "] {
            let new_text = format!("{text}{edit}");
            let inc = Analyser::new().analyse_incremental(&text, &cur, &new_text, dialect);
            let fresh = Analyser::new().analyse(&new_text, dialect);
            assert!(
                inc == fresh,
                "incremental != fresh for a dynamically-named proc:\n{}",
                describe_analysis_divergence(src, &inc, &fresh)
            );
            text = new_text;
            cur = segment_commands(&text);
        }
    }
}

#[test]
#[ignore = "corpus fuzz; run explicitly with --ignored (needs tmp/ trees)"]
fn incremental_matches_fresh_over_corpus() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in ["tcl8.6.18/library", "tcllib-2.0/modules"] {
        gather(&repo_root().join("tmp").join(v), &mut files, 200);
    }
    // Chunk / resume support (env `TCL_FUZZ_SKIP` / `TCL_FUZZ_LIMIT`).
    let (files, start0, total) = Progress::slice(&files);
    // Durable, flushed progress + full-detail findings (survive a `SIGKILL`).
    let prog = Mutex::new(Progress::new("differential_incremental"));
    let done = AtomicUsize::new(start0);

    // Files are independent (each round chain uses fresh `Analyser::new()`
    // instances), so parallelise across files; the per-file seed/round walk
    // stays sequential because each edit feeds the next.
    let outcomes: Vec<(usize, Vec<String>)> = files
        .par_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let (mut checked, mut mismatches) = (0usize, Vec::<String>::new());
            match std::fs::read_to_string(path) {
                Ok(orig) if orig.len() <= 60_000 => {
                    for seed in [1u64, 7, 42] {
                        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).max(1));
                        let mut text = orig.clone();
                        let mut cmds = segment_commands(&text);
                        for round in 0..40 {
                            let new_text = random_edit(&text, &mut rng);
                            let inc = Analyser::new()
                                .analyse_incremental(&text, &cmds, &new_text, dialect);
                            let fresh = Analyser::new().analyse(&new_text, dialect);
                            checked += 1;
                            if inc != fresh {
                                let detail = describe_analysis_divergence(
                                    &format!("{name} seed={seed} round={round}"),
                                    &inc,
                                    &fresh,
                                );
                                prog.lock().unwrap().finding(&detail);
                                mismatches.push(detail);
                            }
                            text = new_text;
                            cmds = segment_commands(&text);
                        }
                    }
                }
                _ => {}
            }
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 20 == 0 || n == total {
                prog.lock()
                    .unwrap()
                    .tick(n, total, &format!("last={name} steps={checked}"));
            }
            (checked, mismatches)
        })
        .collect();

    let checked: usize = outcomes.iter().map(|(c, _)| *c).sum();
    let mismatches: Vec<String> = outcomes.into_iter().flat_map(|(_, m)| m).take(10).collect();
    prog.into_inner()
        .unwrap()
        .finish(&format!("{checked} steps, {} findings", mismatches.len()));
    assert!(
        mismatches.is_empty(),
        "incremental != fresh in {} / {checked} steps:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!("E5 fuzz: {checked} incremental==fresh steps clean");
}
