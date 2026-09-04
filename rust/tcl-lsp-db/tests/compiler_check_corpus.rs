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

use salsa::Setter as _;
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb, compiler_check_diagnostics,
    compiler_check_diagnostics_uncached,
};

mod common;
use common::Progress;

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
        None,
        0,
        Vec::new(),
        Vec::new(),
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
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
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
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    assert_eq!(
        got.checks, want.checks,
        "init.tcl checks diverge (memo vs uncached)"
    );
}

/// Focused regression for issue #1117 — the exact file the `cc_diff` repro
/// names.  `generalClasses.tcl` is ~140 KB of `TclOO`: almost all of its code
/// lives in *method* bodies, which are not procedures and so never reach
/// `proc_taint_solve`'s `analysable_functions` loop.  That query's top-up over
/// `analysable_methods_and_body_units` used to run only `shimmer_family_checks`
/// instead of the whole `function_nontaint_checks` family, so the memoised path
/// reported 20 fewer `O100`/`O105`/`O106` hints than the direct build.  The
/// procedural `tmp/tcl*/library` + `tcllib` sweep below cannot see this: those
/// trees define practically no `TclOO` methods.
///
/// Corpus-gated (`experiments/corpus/fetch_corpus.sh`), so it skips when the
/// checkout is absent, exactly like the `tmp/` fixtures above.
#[test]
fn compiler_check_memo_matches_uncached_tcloo_corpus() {
    let dialect = "tcl8.6";
    let path = repo_root().join("experiments/corpus/georgtree/SpiceGenTcl/src/generalClasses.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skip: {} not present", path.display());
        return;
    };
    let db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let got = compiler_check_diagnostics(&db, file, default_config(&db));
    let registry = db.registry(dialect);
    let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
    assert_eq!(
        got.checks.len(),
        want.checks.len(),
        "generalClasses.tcl check count diverges (memo {} vs uncached {}) — #1117",
        got.checks.len(),
        want.checks.len()
    );
    assert_eq!(
        got.checks, want.checks,
        "generalClasses.tcl checks diverge (memo vs uncached) — #1117"
    );
    assert_eq!(
        got.optimisations, want.optimisations,
        "generalClasses.tcl optimisations diverge (memo vs uncached)"
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
        "tcl9.0.4/library",
        "tcllib-2.0/modules",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 1500);
    }

    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("compiler_check_gate");
    let db = TclDatabase::default();
    let mut checked = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for (idx, path) in files.iter().enumerate() {
        let name = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        if src.is_empty() || src.len() > 400_000 {
            continue;
        }
        let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
        let got = compiler_check_diagnostics(&db, file, default_config(&db));
        let registry = db.registry(dialect);
        let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
        checked += 1;
        if got.checks != want.checks || got.optimisations != want.optimisations {
            let detail = format!(
                "{name}: checks {}->{} opts {}->{}",
                want.checks.len(),
                got.checks.len(),
                want.optimisations.len(),
                got.optimisations.len()
            );
            prog.finding(&detail);
            if bad.len() < 40 {
                bad.push(detail);
            }
        }
        if (idx + 1) % 25 == 0 || idx + 1 == files.len() {
            prog.tick(start0 + idx + 1, total, &format!("last={name}"));
        }
    }
    prog.finish(&format!("{checked} files, {} mismatches", bad.len()));

    assert!(
        bad.is_empty(),
        "compiler_check_diagnostics (memo) != uncached in {}+ / {checked} files:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// Corpus-scale **random-edit** differential for the memoised checks path
/// (SRV-INCREMENTAL Task 2b gate — the random-edit fuzzer over real source the
/// status table flags as "still to build").  The sweep above compares memo vs
/// uncached on a *fresh* db per file; this drives each file through a fuzzed
/// **edit sequence** on one **warm** db — prepending blank lines (an offset
/// shift the per-proc memo must reuse), appending a fuzzed synthetic proc (a
/// re-analysed body), and toggling that proc's shape — asserting the warm
/// memoised `compiler_check_diagnostics` stays byte-identical to a from-scratch
/// `compiler_check_diagnostics_uncached` after every edit.  A stale per-proc
/// lattice / summary cache surviving an edit would diverge here where the
/// fresh-db sweep cannot see it.
#[test]
#[ignore = "slow corpus random-edit sweep (~minutes over tmp/); run with --ignored"]
#[allow(clippy::cast_possible_truncation)] // index modulo tiny arrays
fn compiler_check_memo_matches_uncached_under_corpus_edits() {
    let dialect = "tcl8.6";
    let mut files = Vec::new();
    for v in [
        "tcl8.6.16/library",
        "tcllib-2.0/modules",
        "tcl9.0.4/library",
    ] {
        gather(&repo_root().join("tmp").join(v), &mut files, 120);
    }
    files.sort();

    // Fuzzed trailing-proc shapes (exercise SCCP const-branch, loop optimisations,
    // a taint passthrough+sink, and a plain body).
    let tail_procs = [
        "",
        "\nproc __fuzz_tail {n} { if {1} { set y 1 }\n return $y }\n",
        "\nproc __fuzz_tail {n} { set acc 0\n for {set i 0} {$i < $n} {incr i} { set acc [expr {$acc + $i}] }\n return $acc }\n",
        "\nproc __fuzz_tail {} { set u [gets stdin]; exec $u }\n",
    ];

    let mut rng = 0x5eed_1234_abcd_0001_u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    let (files, start0, total) = Progress::slice(&files);
    let mut prog = Progress::new("compiler_check_edit_fuzz");
    let mut bad: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (idx, path) in files.iter().enumerate() {
        let name = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .display()
            .to_string();
        let Ok(base) = std::fs::read_to_string(path) else {
            continue;
        };
        if base.is_empty() || base.len() > 200_000 {
            continue;
        }
        let mut db = TclDatabase::default();
        let registry = db.registry(dialect);
        let file = SourceFile::new(&db, base.clone(), dialect.to_owned(), None);

        for _ in 0..8 {
            let blanks = (next() as usize) % 4; // offset shift above the real procs
            let tail = tail_procs[(next() as usize) % tail_procs.len()];
            let src = format!("{}{base}{tail}", "\n".repeat(blanks));
            file.set_text(&mut db).to(src.clone());

            let got = compiler_check_diagnostics(&db, file, default_config(&db));
            let want = compiler_check_diagnostics_uncached(&src, registry, dialect, None, None);
            checked += 1;
            if got.checks != want.checks || got.optimisations != want.optimisations {
                let detail = format!(
                    "{name} (blanks={blanks} tail_len={}): checks {}->{} opts {}->{}",
                    tail.len(),
                    want.checks.len(),
                    got.checks.len(),
                    want.optimisations.len(),
                    got.optimisations.len()
                );
                prog.finding(&detail);
                if bad.len() < 40 {
                    bad.push(detail);
                }
                break; // one report per file is enough
            }
        }
        prog.tick(
            start0 + idx + 1,
            total,
            &format!("last={name} steps={checked}"),
        );
    }
    prog.finish(&format!("{checked} steps, {} findings", bad.len()));

    assert!(
        bad.is_empty(),
        "memo != uncached under edits in {}+ files ({checked} edit-steps checked):\n{}",
        bad.len(),
        bad.join("\n")
    );
}
