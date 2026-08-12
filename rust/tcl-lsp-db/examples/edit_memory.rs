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

//! Resident-memory growth across a *typing session* (issue #1035).
//!
//! `tail_profile` alternates between two fixed texts, so the content-keyed
//! interned structs only ever see two distinct bodies and nothing accumulates —
//! which is why it measured the incremental path as healthy while real editing
//! grew without bound.  This drives the buffer the way an editor does instead:
//! every keystroke produces a **fresh, never-seen-before** text (of constant
//! length, so growth can never be blamed on a bigger input), and every keystroke
//! runs the production diagnostic queries.
//!
//! The `mode` argument bisects the growth onto a subsystem, which is how #1035
//! was localised: `set` (input write only) stayed flat, while `nosalsa` — the
//! analyser called with no query database at all — leaked just as steadily,
//! placing the bug in the compiler rather than the incremental layer.
//!
//! Usage: `cargo run --release --example edit_memory -- [edits] [mode] [file]`
//! where `mode` is `both` (default), `fa`, `cc`, `set`, or `nosalsa`.

use salsa::Setter as _;
use tcl_compiler::analyser::{Analyser, NonAsciiMode};
use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, compiler_check_diagnostics, file_analysis_incremental,
};

/// Resident set size in KiB, read from `/proc/self/statm` (pages) — no
/// allocator introspection, so it also catches arena/`mmap` growth.
fn rss_kib() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = statm
        .split_whitespace()
        .nth(1)
        .expect("resident field")
        .parse()
        .expect("resident pages");
    pages * 4
}

fn main() {
    let mut args = std::env::args().skip(1);
    let edits: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(200);
    // Which work each keystroke drives, so the growth can be bisected onto a
    // subsystem: `both` (production), `fa` / `cc` (one query each), `set` (input
    // write only — no analysis), `nosalsa` (the analyser called directly, with
    // no query database in the picture at all).
    let mode = args.next().unwrap_or_else(|| "both".to_owned());
    let rel = args
        .next()
        .unwrap_or_else(|| "samples/tcl/09_long_code.tcl".to_owned());
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&rel);
    let src = std::fs::read_to_string(&path).expect("read source file");
    let dialect = "tcl8.6";

    // Type inside a procedure *body*, so the body-keyed interned structs
    // (`ProcBodyKey` / `ItemBodyKey` / `FnLatticeKey` / `OptDepsKey`) see a new
    // key on every keystroke — exactly what editing does.
    //
    // Ask the analyser for `body_span` rather than scanning for a brace: in
    // `proc name {args} {body}` the first `{` after `proc` opens the *parameter
    // list*, so a naive scan types into the parameters instead. That still
    // parses, so nothing fails loudly — it just measures the wrong thing, and a
    // parameter edit additionally re-keys every procedure's lattice through the
    // module-wide `proc_params` context rather than isolating one body.
    //
    // `body_span.start()` is the offset *of* the opening `{`, hence the `+ 1`:
    // inserting at `start()` itself lands between the parameter list and the
    // brace, which leaves every body unedited (merely shifted). The body-keyed
    // interned structs are offset-invariant, so that session re-keys nothing —
    // it measures a buffer that changes with no incremental work behind it.
    let edit_pos = Analyser::new()
        .analyse(&src, dialect)
        .all_procs
        .values()
        .map(|p| p.body_span)
        .filter(|s| s.end() > s.start())
        .max_by_key(|s| s.end() - s.start())
        .map_or(src.len() / 2, |s| s.start() as usize + 1);

    let mut db = TclDatabase::default();
    let file = SourceFile::new(&db, src.clone(), dialect.to_owned(), None);
    let config = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
     0,);

    println!(
        "== {rel} ({} lines, {} bytes) — {edits} keystrokes, mode={mode} ==",
        src.lines().count(),
        src.len()
    );

    // Cold build first, so the baseline excludes one-time registry/parse cost.
    let _ = file_analysis_incremental(&db, file, config);
    let _ = compiler_check_diagnostics(&db, file, config);
    let base = rss_kib();
    println!("  after cold build:        {base:>8} KiB RSS");

    for i in 1..=edits {
        // A distinct, **constant-length** body on every edit: the buffer never
        // repeats (an editor never replays an old text) but it also never grows,
        // so per-edit growth cannot be blamed on a bigger input. `{i:08}` keeps
        // the inserted statement byte-identical in length across the whole run.
        let mut text = src.clone();
        text.insert_str(edit_pos, &format!("\n    set probe_v {i:08}"));

        if mode == "nosalsa" {
            // No database at all: just the analyser, on a fresh text each time.
            std::hint::black_box(Analyser::new().analyse(&text, dialect));
        } else {
            file.set_text(&mut db).to(text);
            if mode == "both" || mode == "fa" {
                let _ = file_analysis_incremental(&db, file, config);
            }
            if mode == "both" || mode == "cc" {
                let _ = compiler_check_diagnostics(&db, file, config);
            }
        }

        if i % 20 == 0 || i == 1 {
            let now = rss_kib();
            println!(
                "  after {i:>4} edits:        {now:>8} KiB RSS   (+{} KiB, {:.1} KiB/edit)",
                now.saturating_sub(base),
                f64::from(u32::try_from(now.saturating_sub(base)).unwrap_or(u32::MAX))
                    / f64::from(i),
            );
        }
    }

    report_ingredients(&db, edits);

    // Is the query graph holding it?  Drop the database and re-measure: salsa's
    // interned garbage collector recycles slots (the counts above plateau), so a
    // resident set that does not fall here is not being held by anything salsa
    // still owns.
    let before_drop = rss_kib();
    drop(db);
    println!("\n== where the resident memory lives ==");
    println!("  peak (db alive):         {before_drop:>8} KiB RSS");
    println!("  after drop(db):          {:>8} KiB RSS", rss_kib());
}

/// Per-interned-struct slot counts and per-query memo counts, to decide whether
/// the query graph is the one growing.  A count that tracks the edit number is a
/// key salsa's interned garbage collector never reclaimed; counts that plateau
/// while RSS climbs mean the memory is *not* salsa's, and the next step is to
/// bisect with `mode` rather than to keep tuning the query keys.
///
/// (The `bytes` column is stack size only — these ingredients define no
/// `heap_size`, so the `Arc` payloads behind them are not counted. Read it as a
/// relative signal, never as a memory total.)
fn report_ingredients(db: &TclDatabase, edits: u32) {
    let usage = <dyn salsa::Database>::memory_usage(db);

    let mut structs: Vec<_> = usage
        .structs
        .iter()
        .filter(|s| s.count() > 0)
        .map(|s| {
            (
                s.debug_name(),
                s.count(),
                s.size_of_fields(),
                s.heap_size_of_fields(),
            )
        })
        .collect();
    structs.sort_by_key(|&(_, count, ..)| std::cmp::Reverse(count));

    println!("\n== interned / input slots after {edits} edits ==");
    println!("  {:<28} {:>8}  {:>12}", "ingredient", "slots", "bytes");
    for (name, count, fields, heap) in structs.iter().take(12) {
        println!(
            "  {name:<28} {count:>8}  {:>12}",
            fields + heap.unwrap_or(0)
        );
    }

    let mut queries: Vec<_> = usage
        .queries
        .values()
        .filter(|q| q.count() > 0)
        .map(|q| {
            (
                q.debug_name(),
                q.count(),
                q.size_of_fields(),
                q.heap_size_of_fields(),
            )
        })
        .collect();
    queries.sort_by_key(|&(_, count, ..)| std::cmp::Reverse(count));

    println!("\n== memoised query results after {edits} edits ==");
    println!("  {:<28} {:>8}  {:>12}", "query output", "memos", "bytes");
    for (name, count, fields, heap) in queries.iter().take(12) {
        println!(
            "  {name:<28} {count:>8}  {:>12}",
            fields + heap.unwrap_or(0)
        );
    }
}
