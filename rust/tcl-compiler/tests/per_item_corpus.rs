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

//! Slice-2b corpus gate: `analyse_per_item` must equal `analyse` byte-for-byte
//! over the real-world `tmp/` corpus (the per-item walk decomposition must
//! converge to exactly a full rebuild). Corpus-gated (`--ignored`), mirroring
//! `differential_incremental`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use tcl_compiler::analyser::Analyser;

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

#[test]
#[ignore = "corpus gate; run explicitly with --ignored (needs tmp/ trees)"]
fn per_item_matches_analyse_over_corpus() {
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
    // Chunk support: process files[skip .. skip+limit] (env `TCL_FUZZ_SKIP` /
    // `TCL_FUZZ_LIMIT`), so a killed sweep resumes and a batch can be bounded.
    let (files, start0, total) = Progress::slice(&files);

    // Durable, flushed progress + findings: each divergence is described in full
    // and written to `target/fuzz-progress/per_item_gate.log` the instant it is
    // found, so a `SIGKILL` mid-sweep still leaves every finding on disk.
    let prog = Mutex::new(Progress::new("per_item_gate"));
    let done = AtomicUsize::new(start0);

    // Each file is analysed by independent `Analyser::new()` instances, so the
    // sweep parallelises cleanly across files (the bottleneck is the ~2 deep
    // analyses per file in a debug build). Collect a per-file outcome and
    // aggregate afterwards so the gate is order-independent.
    let outcomes: Vec<(bool, Option<String>)> = files
        .par_iter()
        .map(|path| {
            let Ok(src) = std::fs::read_to_string(path) else {
                return (false, None);
            };
            if src.len() > 400_000 {
                return (false, None);
            }
            let want = Analyser::new().analyse(&src, dialect);
            let got = Analyser::new().analyse_per_item(&src, dialect);
            let fellback = !tcl_lexer::script_is_complete(&src) || src.contains("tcl-lsp: stub");
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mismatch = (got != want).then(|| describe_analysis_divergence(&name, &got, &want));
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            {
                let mut p = prog.lock().unwrap();
                if let Some(m) = &mismatch {
                    p.finding(m);
                }
                if n % 25 == 0 || n == total {
                    p.tick(n, total, &format!("last={name}"));
                }
            }
            (fellback, mismatch)
        })
        .collect();

    let checked = outcomes.len();
    let fellback = outcomes.iter().filter(|(f, _)| *f).count();
    let mismatches: Vec<String> = outcomes
        .into_iter()
        .filter_map(|(_, m)| m)
        .take(25)
        .collect();
    prog.into_inner().unwrap().finish(&format!(
        "{checked} files, {} mismatches, {fellback} trivially-fellback",
        mismatches.len()
    ));

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

/// CI-runnable slice of the byte-identity gate (issue #1123): the corpus
/// gates in this file are `#[ignore]`d (they need the fetched `tmp/` trees),
/// which let the contract rot unnoticed for months.  This one walks the
/// repo's **own** `samples/**/*.tcl` — checked in, deterministic, always
/// present — so every ordinary `cargo test` run re-proves `analyse_per_item
/// == analyse` over real files, not just the unit fixtures.
#[test]
fn per_item_matches_analyse_over_repo_samples() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    gather(&repo_root().join("samples"), &mut files, 500);
    files.sort();
    assert!(
        files.len() >= 50,
        "expected the checked-in samples corpus; found {} files",
        files.len()
    );
    let mut mismatches = Vec::new();
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let want = Analyser::new().analyse(&src, dialect);
        let got = Analyser::new().analyse_per_item(&src, dialect);
        if got != want {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            mismatches.push(describe_analysis_divergence(&name, &got, &want));
        }
    }
    assert!(
        mismatches.is_empty(),
        "analyse_per_item != analyse in {} / {} sample files:\n{}",
        mismatches.len(),
        files.len(),
        mismatches.join("\n")
    );
}

// Tiny deterministic xorshift PRNG so failures reproduce from the seed.
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
            let at = pick(rng);
            let snips = [
                "\nset y 2\n",
                " ",
                "# c\n",
                "puts $z",
                "}\nproc q {} {",
                "[",
                "$a",
                "\nvariable ns::v\n",
                "\nset ::g 1\n",
                "\nglobal g\n",
            ];
            let s = snips[rng.upto(snips.len())];
            format!("{}{}{}", &text[..at], s, &text[at..])
        }
        1 => {
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
            let a = pick(rng);
            let b = pick(rng);
            let (lo, hi) = (a.min(b), a.max(b));
            format!("{}{}{}", &text[..lo], "X", &text[hi..])
        }
    }
}

/// Edit-based per-item gate: `analyse_per_item` must equal `analyse`
/// byte-for-byte after *every* random edit (not just on the pristine corpus).
/// This is the `incremental == fresh` contract for the per-item walk — the
/// permanent backstop for the widened enclosing-context fast path, which the
/// existing `differential_incremental` fuzzer does not cover (it exercises the
/// older `analyse_commands`-based `analyse_incremental`).  `analyse_per_item`
/// falls back to a full rebuild whenever it can't prove equivalence, so the
/// equality must hold unconditionally; a fast-path divergence is a real bug.
#[test]
#[ignore = "corpus fuzz; run explicitly with --ignored (needs tmp/ trees)"]
fn per_item_matches_analyse_under_edits() {
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
    // Chunk / resume (env `TCL_FUZZ_SKIP` / `TCL_FUZZ_LIMIT`).
    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("per_item_edit_fuzz");

    // Random edits can produce malformed transient inputs that make the
    // *reference* `analyse` itself panic (e.g. a non-char-boundary slice deep in
    // the expr lexer) — a pre-existing analyser-robustness concern orthogonal to
    // the incremental contract.  Silence the default panic hook so those
    // expected, caught panics don't spam the test log.
    std::panic::set_hook(Box::new(|_| {}));
    let run = |src: &str| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Analyser::new().analyse(src, dialect)
        }))
        .ok()
    };
    let run_pi = |src: &str| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Analyser::new().analyse_per_item(src, dialect)
        }))
        .ok()
    };

    let mut checked = 0usize;
    let mut steps = 0usize;
    let mut mismatches: Vec<String> = Vec::new();
    'files: for (idx, path) in files.iter().enumerate() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Ok(orig) = std::fs::read_to_string(path) else {
            prog.tick(start0 + idx + 1, total, &format!("skip(unreadable) {name}"));
            continue;
        };
        if orig.is_empty() || orig.len() > 200_000 {
            prog.tick(start0 + idx + 1, total, &format!("skip(size) {name}"));
            continue;
        }
        checked += 1;
        // A few independent edit sequences per file (different seeds).
        for seed in [0x9E37_79B9_7F4A_7C15u64, 0xD1B5_4A32_D192_ED03] {
            let mut rng = Rng(seed ^ (orig.len() as u64).wrapping_mul(0x1000_0001B3));
            let mut text = orig.clone();
            for _ in 0..16 {
                text = random_edit(&text, &mut rng);
                if text.len() > 400_000 {
                    break;
                }
                let want = run(&text);
                let got = run_pi(&text);
                steps += 1;
                // If the reference panics on this malformed transient, there is
                // nothing to compare — but `analyse_per_item` must not diverge
                // (it either matches, or also panics by falling back to the same
                // `analyse`).  A one-sided panic, or differing results, is a bug.
                let bad = match (&want, &got) {
                    (Some(w), Some(g)) => w != g,
                    (None, None) => false,
                    _ => true,
                };
                if bad {
                    // Full detail, flushed the instant it is found (survives a kill).
                    let detail = match (&want, &got) {
                        (Some(w), Some(g)) => {
                            describe_analysis_divergence(&format!("{name} (seed {seed:#x})"), g, w)
                        }
                        (Some(_), None) => {
                            format!("{name} (seed {seed:#x}): per_item panicked, analyse ok")
                        }
                        (None, Some(_)) => {
                            format!("{name} (seed {seed:#x}): analyse panicked, per_item ok")
                        }
                        (None, None) => unreachable!(),
                    };
                    prog.finding(&detail);
                    mismatches.push(detail);
                    if mismatches.len() >= 25 {
                        break 'files;
                    }
                    break; // next seed/file
                }
            }
        }
        prog.tick(start0 + idx + 1, total, &format!("{name} steps={steps}"));
    }
    let _ = std::panic::take_hook();
    prog.finish(&format!(
        "{checked} files, {steps} steps, {} findings",
        mismatches.len()
    ));

    assert!(
        mismatches.is_empty(),
        "analyse_per_item != analyse after edit in {} cases ({checked} files, {steps} steps):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!("per-item edit gate: {checked} files, {steps} edited states, all byte-identical");
}
