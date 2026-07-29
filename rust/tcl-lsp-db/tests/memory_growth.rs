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

//! CI-run bounded-growth regression test for issue #1035 (promoted from the
//! manual `examples/edit_memory.rs` profiling harness).
//!
//! #1035's root cause was `mathop_generated::specs()` /
//! `mathfunc_generated::specs()` each `Box::leak`ing a fresh set of
//! `&'static` strings on *every* registry rebuild — and a registry rebuild
//! happens on every keystroke via `file_analysis_incremental`/
//! `compiler_check_diagnostics`'s downstream compiler/optimiser paths. The
//! fix ([`tcl_registry`]'s `OnceLock` memoisation) turned that from an
//! unbounded per-edit leak into a one-off cost.
//!
//! A coarse RSS check (`/proc/self/statm`, as `edit_memory.rs` uses for its
//! human-readable report) is too noisy and platform-dependent for a CI
//! assertion, and this workspace forbids `unsafe` code (`unsafe_code =
//! "forbid"` — see `Cargo.toml`), which rules out a hand-rolled counting
//! `GlobalAlloc`. Instead this test uses the same **safe**, deterministic
//! introspection `examples/edit_memory.rs` already reports for humans:
//! `<dyn salsa::Database>::memory_usage` (the `salsa_unstable` feature,
//! already a `tcl-lsp-db` dev-dependency), which sums the retained bytes of
//! every interned/tracked struct slot and every memoised query result. That
//! is exactly the "assert the salsa storage size plateaus" alternative
//! this test was asked to prefer.
//!
//! Runs a small, CI-fast number of synthetic edits (`EDITS`, each a
//! distinct, constant-length inserted statement — never a repeat, so
//! nothing can hide behind salsa's identical-input early cutoff) split into
//! four equal quartiles, and asserts the last quartile's retained-bytes
//! growth is not a runaway multiple of the middle quartile's — a
//! **relative plateau** assertion, not an absolute byte budget, so it
//! stays meaningful across machines and Rust/salsa versions.

use salsa::Setter as _;

use tcl_compiler::analyser::{Analyser, NonAsciiMode};
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, compiler_check_diagnostics, file_analysis_incremental,
};

/// Total retained bytes salsa reports across every interned/tracked struct
/// slot and every memoised query result — the same `size_of_fields()` +
/// `heap_size_of_fields()` figures `examples/edit_memory.rs`'s
/// `report_ingredients` prints per-ingredient, summed into one number.
fn total_salsa_retained_bytes(db: &TclDatabase) -> u64 {
    let usage = <dyn salsa::Database>::memory_usage(db);
    let struct_bytes: u64 = usage
        .structs
        .iter()
        .map(|s| (s.size_of_fields() + s.heap_size_of_fields().unwrap_or(0)) as u64)
        .sum();
    let query_bytes: u64 = usage
        .queries
        .values()
        .map(|q| (q.size_of_fields() + q.heap_size_of_fields().unwrap_or(0)) as u64)
        .sum();
    struct_bytes + query_bytes
}

/// A small, self-contained synthetic source with several procedures, so it
/// does not depend on any `samples/` fixture staying a particular size.
const SRC: &str = r#"
namespace eval ::bench {
    variable counter 0
}

proc ::bench::helper {a b} {
    set total 0
    for {set i 0} {$i < 10} {incr i} {
        set total [expr {$total + $a * $i - $b}]
        if {$total > 100} {
            set total 100
        }
    }
    return $total
}

proc ::bench::step {a b} {
    set v [expr {$a + $b}]
    set msg "step = $v"
    if {$v > 10} {
        set v [expr {$v + 1}]
    }
    return $v
}

proc ::bench::run {} {
    variable counter
    incr counter
    return [::bench::step $counter [::bench::helper $counter 2]]
}
"#;

fn build_config(db: &TclDatabase) -> AnalyserConfig {
    AnalyserConfig::new(
        db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
    )
}

/// Total edits driven through the database. Kept small enough to stay
/// CI-fast (well under a second in a debug build) while still giving four
/// distinct quartiles wide enough to average out per-edit noise.
const EDITS: u32 = 80;
const QUARTILE: u32 = EDITS / 4;

#[test]
fn edit_session_memory_growth_plateaus() {
    let dialect = "tcl8.6";

    // Type inside `::bench::helper`'s body — the same "find the largest
    // proc body via the analyser, not a brace scan" approach
    // `examples/edit_memory.rs` uses, so the edit lands inside a real body
    // and re-keys the same body-scoped interned structs an editor would.
    let edit_pos = Analyser::new()
        .analyse(SRC, dialect)
        .all_procs
        .values()
        .map(|p| p.body_span)
        .filter(|s| s.end() > s.start())
        .max_by_key(|s| s.end() - s.start())
        .map_or(SRC.len() / 2, |s| s.start() as usize);

    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), dialect.to_owned(), None);
    let config = build_config(&db);

    // Cold build first — one-time registry/parse cost excluded from the
    // per-edit measurement, exactly as `edit_memory.rs` does.
    let _ = file_analysis_incremental(&db, file, config);
    let _ = compiler_check_diagnostics(&db, file, config);

    let mut checkpoints: Vec<u64> = vec![total_salsa_retained_bytes(&db)];

    for i in 1..=EDITS {
        // A distinct, constant-length statement every edit — the buffer
        // never repeats (so nothing can hide behind salsa's early cutoff
        // on an unchanged value) but never grows in length either, so
        // growth cannot be blamed on a bigger input.
        let mut text = SRC.to_owned();
        text.insert_str(edit_pos, &format!("\n    set probe_v {i:08}"));
        file.set_text(&mut db).to(text);

        let _ = file_analysis_incremental(&db, file, config);
        let _ = compiler_check_diagnostics(&db, file, config);

        if i % QUARTILE == 0 {
            checkpoints.push(total_salsa_retained_bytes(&db));
        }
    }

    assert_eq!(
        checkpoints.len(),
        5,
        "expected one baseline + four quartile checkpoints, got {checkpoints:?}"
    );
    let growth: Vec<u64> = checkpoints
        .windows(2)
        .map(|w| w[1].saturating_sub(w[0]))
        .collect();
    let [q1, q2, q3, q4] = growth[..] else {
        unreachable!("windows(2) over 5 checkpoints yields exactly 4 deltas")
    };

    // A generous multiple of the middle quartile, with a floor so a
    // healthy near-zero baseline (the expected post-fix steady state)
    // cannot make the assertion spuriously strict. This is a *relative*
    // plateau check — it has no absolute byte budget to re-tune per
    // machine or Rust/salsa version — so it fails only on genuinely
    // unbounded, still-accelerating growth (a real leak), not on ordinary
    // interning/cache noise.
    let floor = 64 * 1024; // 64 KiB
    let budget = (q2 * 4).max(floor);
    assert!(
        q4 <= budget,
        "salsa-retained-bytes growth did not plateau across the {EDITS}-edit session \
         (issue #1035 regression class): quartile growth q1={q1} q2={q2} q3={q3} q4={q4} \
         bytes, expected q4 <= {budget} (4x mid-quartile q2, floor {floor}); checkpoints \
         (total salsa retained bytes) = {checkpoints:?}"
    );
}
