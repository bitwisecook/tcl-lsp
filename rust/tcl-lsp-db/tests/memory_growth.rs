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
//! This workspace forbids `unsafe` code (`unsafe_code = "forbid"` — see
//! `Cargo.toml`), which rules out a hand-rolled counting `GlobalAlloc`. The
//! primary measurement is therefore the same **safe**, deterministic
//! introspection `examples/edit_memory.rs` already reports for humans:
//! `<dyn salsa::Database>::memory_usage` (the `salsa_unstable` feature,
//! already a `tcl-lsp-db` dev-dependency), which sums the retained bytes of
//! every interned/tracked struct slot and every memoised query result.
//!
//! ## What this test does and does not cover
//!
//! `memory_usage` accounts for memory the **query database owns**. It is
//! structurally blind to heap that nothing owns — which is exactly what
//! #1035's own root cause was: `Box::leak`ed `&'static` strings belong to
//! no salsa ingredient, so a reverted fix would leak ~500 KiB per edit and
//! this figure would not move at all. That class is pinned directly, by
//! pointer identity, in `tcl-registry`'s
//! `commands::tcl::mathop_generated::tests::specs_are_built_once_and_never_releaked`
//! and its `mathfunc_generated` twin: two `specs()` calls must hand back
//! the *same* allocations, which equal-but-freshly-leaked strings would
//! fail. Those two tests are the #1035 regression guard; this one covers
//! the complementary, database-owned growth they cannot see, plus a coarse
//! resident-set companion (Linux only) that sees both.
//!
//! ## Shape of the assertion
//!
//! Runs a small, CI-fast number of synthetic edits (see `edits`, each a
//! distinct, constant-length inserted statement — never a repeat, so
//! nothing can hide behind salsa's identical-input early cutoff) split into
//! four equal quartiles. The first quartile is warm-up (lazily-populated
//! caches reaching steady state); the assertion is that the **late window**
//! — the second half of the session — adds only a small fraction of what
//! warm-up did. A constant linear leak keeps every quartile equal, so it
//! fails that comparison however small the per-edit amount; a genuine
//! plateau passes it with orders of magnitude to spare.

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

/// Resident set size in KiB from `/proc/self/statm`, or `None` off Linux (or
/// if the read fails).
///
/// A companion, not the primary measure: it needs no allocator
/// introspection, so unlike [`total_salsa_retained_bytes`] it *does* see
/// heap the query database does not own — the `Box::leak` class #1035 was.
/// It is coarse, so the bound it carries is deliberately loose.
fn rss_kib() -> Option<u64> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages * 4)
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
        0,
        Vec::new(),
        Vec::new(),
    )
}

/// Baseline edits driven through the database, on the narrowest machine.
/// Small enough to stay CI-fast while still giving four distinct quartiles
/// wide enough to average out per-edit noise.
const BASE_EDITS: u32 = 80;

/// Salsa's interned-table shard count — `(available_parallelism() * 4)`
/// rounded up to a power of two.  The reference width this file's baseline
/// was measured at is 16 shards (a 4-core machine).
fn interned_shards() -> u32 {
    let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    u32::try_from(cpus.saturating_mul(4).next_power_of_two()).unwrap_or(u32::MAX)
}

/// Total edits driven through the database, scaled by the interned-table
/// shard count.
///
/// The plateau this test asserts is reached when salsa's interned collector
/// starts recycling slots instead of allocating new ones, and a slot only
/// becomes reclaimable once its **own** shard's LRU tail holds a stale entry.
/// So a wider machine needs proportionally more revisions to reach the same
/// steady state, and a fixed count is a machine-dependent test.  Measured, all
/// else equal:
///
/// | shards | q1 | q2 | q3 | q4 | late window |
/// |-------:|---:|---:|---:|---:|------------:|
/// |     16 | 7680 | 3232 |  792 | 1016 |  1808 (plateaued) |
/// |    32+ | 10632 | 8440 | 6024 | 4672 | 10696 (still falling) |
///
/// Both rows are the *same* healthy behaviour seen at different points on the
/// curve; only the second had not yet arrived.  Scaling the session length
/// keeps the assertion measuring the steady state rather than the warm-up.
fn edits() -> u32 {
    (interned_shards() * (BASE_EDITS / 16)).max(BASE_EDITS)
}

/// One quarter of [`edits`], the checkpoint interval.
fn quartile() -> u32 {
    edits() / 4
}

/// Resident-set budget for the late half of a *baseline-length* session, in
/// KiB.  Deliberately loose (measured late-window growth over 10 runs was
/// 0-32 KiB) because RSS is coarse and platform-dependent; it still catches
/// any leak above roughly 200 KiB per edit.
const BASE_LATE_RSS_BUDGET_KIB: u64 = 8 * 1024;

/// [`BASE_LATE_RSS_BUDGET_KIB`] scaled by the session length, so a wider
/// machine — which drives proportionally more edits, see [`edits`] — is held
/// to the same *per-edit* bound rather than a stricter absolute one.
fn late_rss_budget_kib() -> u64 {
    BASE_LATE_RSS_BUDGET_KIB * u64::from(edits()) / u64::from(BASE_EDITS)
}

#[test]
fn edit_session_memory_growth_plateaus() {
    let dialect = "tcl8.6";

    // Type inside `::bench::helper`'s body — the same "find the largest
    // proc body via the analyser, not a brace scan" approach
    // `examples/edit_memory.rs` uses, so the edit lands inside a real body
    // and re-keys the same body-scoped interned structs an editor would.
    //
    // `body_span.start()` is the offset *of* the opening `{`, so the insert
    // point is one past it. Inserting at `start()` itself lands between the
    // parameter list and the brace, which leaves every proc body unedited
    // (merely shifted) — and the body-keyed interned structs are
    // offset-invariant, so such a session re-keys nothing and measures nothing.
    let edit_pos = Analyser::new()
        .analyse(SRC, dialect)
        .all_procs
        .values()
        .map(|p| p.body_span)
        .filter(|s| s.end() > s.start())
        .max_by_key(|s| s.end() - s.start())
        .map_or(SRC.len() / 2, |s| s.start() as usize + 1);

    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, SRC.to_owned(), dialect.to_owned(), None);
    let config = build_config(&db);

    // Cold build first — one-time registry/parse cost excluded from the
    // per-edit measurement, exactly as `edit_memory.rs` does.
    let _ = file_analysis_incremental(&db, file, config);
    let _ = compiler_check_diagnostics(&db, file, config);

    let mut rss: Vec<Option<u64>> = vec![rss_kib()];
    let mut checkpoints: Vec<u64> = vec![total_salsa_retained_bytes(&db)];

    let edits = edits();
    let quartile = quartile();
    for i in 1..=edits {
        // A distinct, constant-length statement every edit — the buffer
        // never repeats (so nothing can hide behind salsa's early cutoff
        // on an unchanged value) but never grows in length either, so
        // growth cannot be blamed on a bigger input.
        let mut text = SRC.to_owned();
        text.insert_str(edit_pos, &format!("\n    set probe_v {i:08}"));
        file.set_text(&mut db).to(text);

        let _ = file_analysis_incremental(&db, file, config);
        let _ = compiler_check_diagnostics(&db, file, config);

        if i % quartile == 0 {
            checkpoints.push(total_salsa_retained_bytes(&db));
            rss.push(rss_kib());
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

    // Warm-up (q1) populates the lazily-built caches; the steady state is
    // what every later edit should cost. A constant *linear* leak keeps all
    // four quartiles equal, so comparing the late window against warm-up
    // rejects it however small the per-edit amount — the previous
    // `q4 <= q2 * 4` form did not, because such a leak inflates q2 exactly
    // as much as q4.
    //
    // Measured on this workspace over 10 consecutive runs, byte-identical
    // every time: q1 = 904 bytes, q2 = q3 = q4 = 0. The bound below
    // therefore keeps ~4 KiB of headroom over a zero steady state while
    // catching any leak above roughly 100 bytes per edit — three orders of
    // magnitude under #1035's own ~500 KiB per edit.
    let late_window = q3 + q4;
    let late_edits = u64::from(quartile) * 2;
    // A quarter of warm-up, with a small absolute floor so a healthy zero
    // steady state cannot make the assertion spuriously strict.
    let late_budget = (q1 / 4).max(4 * 1024);
    assert!(
        late_window <= late_budget,
        "salsa-retained-bytes growth did not plateau across the {edits}-edit session \
         (issue #1035 regression class): quartile growth q1={q1} q2={q2} q3={q3} q4={q4} \
         bytes; the late window (q3+q4 = {late_window} bytes over {late_edits} edits, \
         {} bytes/edit) must be at most {late_budget} — a quarter of the q1 warm-up, \
         floor 4 KiB; checkpoints (total salsa retained bytes) = {checkpoints:?}",
        late_window / late_edits.max(1)
    );

    // Companion: the same plateau in resident set size, which — unlike the
    // salsa figure — also sees heap nothing owns, so it is the half of the
    // measurement that could catch a reverted #1035 directly. Coarse and
    // platform-dependent, hence Linux-gated with a deliberately loose
    // bound: measured late-window growth over 10 runs was 0-32 KiB, and
    // 8 MiB still catches any leak above ~200 KiB per edit.
    if let (Some(mid), Some(end)) = (rss[3], rss[4]) {
        let late_rss = end.saturating_sub(mid);
        let budget = late_rss_budget_kib();
        assert!(
            late_rss <= budget,
            "resident set kept growing through the late half of the session: \
             {late_rss} KiB over {late_edits} edits, budget {budget} KiB; \
             per-quartile RSS (KiB) = {rss:?}"
        );
    }
}
