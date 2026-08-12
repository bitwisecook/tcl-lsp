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

//! Direct-infrastructure concurrency stress test for the salsa query database
//! — issue #829 robustness suite, "no LSP front end" half.
//!
//! Hammers [`TclDatabase`] exactly the way `tcl-lsp-server` does (one writer
//! thread repeatedly calling `SourceFile::set_text`, many reader threads each
//! cloning a snapshot and running the same salsa queries the LSP handlers run
//! — `semantic_tokens` and `file_analysis_incremental`), with **no LSP
//! transport, no subprocess, no JSON-RPC** in the loop at all. This isolates
//! whether the query graph's own cancellation/concurrency contract holds: a
//! writer's `set_text` must never deadlock behind a reader, a reader must
//! never observe a torn/inconsistent result, and neither side may panic.
//!
//! Every #829 fix in this crate — routing `semantic_tokens` through the
//! cancellable `file_analysis_incremental` instead of the uncancellable
//! `file_analysis`, and the server-side fast-path race — depends on this
//! contract holding under real contention, not just in the single-threaded
//! unit tests. This is the adversarial multi-threaded version of that same
//! claim.
//!
//! Usage:
//! ```text
//! cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
//! # Tunables (all optional):
//! STRESS_PROCS=600 STRESS_READERS=8 STRESS_WRITES=200 \
//!   cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis
//! ```
//!
//! Exit code is non-zero if any reader or the writer panicked, or if a
//! post-stress correctness check against a fresh, uncached analysis fails. A
//! read that comes back `Cancelled` is expected and healthy — the whole point
//! of the incremental path is that it can be interrupted; it is only a
//! failure if a read never returns at all (which would show up as this
//! process hanging past `STRESS_TIMEOUT_SECS`).
//!
//! **On any failure** this prints a line starting `STRESS_FAILURE:` (grep for
//! it) naming a self-contained reproduction bundle directory — the exact
//! source text at the moment of failure plus a `repro.md` with both a re-run
//! command and a ready-to-adapt static (non-concurrent) unit test skeleton.
//! Timing-dependent races may not reproduce on every re-run of the harness
//! itself, but the dumped fixture always works as a deterministic starting
//! point for turning the failure into a permanent regression test.

use std::env;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tcl_lsp_db::{
    AnalyserConfig, SourceFile, TclDatabase, TclDb as _, file_analysis_incremental, semantic_tokens,
};

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The same generator shape `generate_big_tcl` uses in the native `lsp_e2e`
/// suite (`rust/tcl-lsp-server/tests/e2e/semantic_tokens_reference_client.rs`)
/// — kept independent here (this crate has no dependency on `tcl-lsp-server`)
/// so both halves of the stress suite exercise a comparable-weight fixture.
fn generate_big_tcl(n: usize) -> String {
    let mut s = String::with_capacity(n * 200);
    s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
    for i in 0..n {
        let _ = write!(
            s,
            "# proc number {i}\n\
             proc ::bench::step{i} {{a b}} {{\n\
             \x20   set v{i} [expr {{$a + $b}}]\n\
             \x20   set msg \"step {i} = $v{i}\"\n\
             \x20   if {{$v{i} > 10}} {{\n\
             \x20       set v{i} [expr {{$v{i} + 1}}]\n\
             \x20   }}\n\
             \x20   return $v{i}\n\
             }}\n\n"
        );
    }
    // One construct the enriched tier treats differently from the coarse one
    // (see tcl-lsp-db's `semantic_tokens_retags_constant_regex_source_true_positive`),
    // so a reader thread's result shape isn't trivially identical regardless
    // of what the writer is doing concurrently.
    s.push_str(
        "proc ::bench::regex_check {} {\n    set my_re \".*abc\"\n    regexp $my_re $s\n}\n",
    );
    s
}

/// A tiny, cheap edit: append a trailing comment with a monotonic counter, so
/// each write is a distinct, valid revision without perturbing the rest of
/// the file's structure (byte-for-byte offset-invariant for every earlier
/// proc, exercising the per-item firewall the same way a real keystroke
/// would).
fn edited(base: &str, generation: u64) -> String {
    format!("{base}# edit generation {generation}\n")
}

/// The parameters one run was invoked with — captured once so any failure
/// path can drop them into a self-contained reproduction bundle without
/// threading half a dozen individual arguments through every function.
#[derive(Clone)]
struct RunParams {
    procs: usize,
    readers: usize,
    writes: usize,
    timeout: Duration,
    dialect: String,
}

/// Write a self-contained reproduction bundle for a failure: the exact source
/// text at the point of failure (a real, directly-loadable `.tcl` file) plus
/// a `repro.md` describing how to reconstruct it — both a re-run of this
/// harness with the same parameters, and a ready-to-adapt static
/// (single-threaded, no timing dependency) unit test. Prints a
/// `STRESS_FAILURE:` line naming the bundle directory; every failure path in
/// this file goes through this so a person or an agent picking up a report
/// never has to re-run the (inherently timing-dependent) stress harness just
/// to see the state that triggered it.
fn dump_repro_bundle(tag: &str, text: &str, params: &RunParams, failure: &str) {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let dir = std::env::temp_dir().join(format!(
        "tcl-lsp-stress-failure-{tag}-{}-{millis}",
        std::process::id(),
    ));
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "stress_concurrent_analysis: could not create repro bundle dir {}: {e}",
            dir.display()
        );
        return;
    }
    let fixture_path = dir.join("fixture.tcl");
    let _ = std::fs::write(&fixture_path, text);
    let repro_md = format!(
        "# stress_concurrent_analysis failure: {tag}\n\
         \n\
         ## What happened\n\
         \n\
         {failure}\n\
         \n\
         ## Run parameters\n\
         \n\
         - STRESS_PROCS={procs}\n\
         - STRESS_READERS={readers}\n\
         - STRESS_WRITES={writes}\n\
         - STRESS_TIMEOUT_SECS={timeout_secs}\n\
         - dialect={dialect}\n\
         \n\
         ## Re-run the stress harness with the same parameters\n\
         \n\
         Timing-dependent races may not reproduce on every run:\n\
         \n\
         ```sh\n\
         STRESS_PROCS={procs} STRESS_READERS={readers} STRESS_WRITES={writes} \\\n\
         \x20 STRESS_TIMEOUT_SECS={timeout_secs} \\\n\
         \x20 cargo run --release -p tcl-lsp-db --example stress_concurrent_analysis\n\
         ```\n\
         \n\
         ## Static, non-concurrent regression test\n\
         \n\
         `fixture.tcl` is the actual source text at the moment of failure, so it also\n\
         works as a deterministic starting point independent of timing — e.g. adapt\n\
         into a `#[test]` in `rust/tcl-lsp-db/src/lib.rs`:\n\
         \n\
         ```rust\n\
         let db = TclDatabase::default();\n\
         let cfg = AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default, Vec::new(), None, None, 0);\n\
         let text = std::fs::read_to_string(\"fixture.tcl\").unwrap();\n\
         let file = SourceFile::new(&db, text, \"{dialect}\".to_owned());\n\
         let tokens = semantic_tokens(&db, file, cfg);\n\
         // Compare `tokens` against tcl_lsp_core::semantic_tokens::full(..) or a\n\
         // fresh Analyser::new().analyse(..) to reconstruct the exact assertion.\n\
         ```\n",
        procs = params.procs,
        readers = params.readers,
        writes = params.writes,
        timeout_secs = params.timeout.as_secs(),
        dialect = params.dialect,
    );
    let _ = std::fs::write(dir.join("repro.md"), repro_md);
    eprintln!("STRESS_FAILURE: reproduction bundle at {}", dir.display());
}

/// Per-reader-thread outcome counters, shared via `Arc` with the main thread.
#[derive(Clone)]
struct ReaderCounters {
    ok: Arc<AtomicU64>,
    cancelled: Arc<AtomicU64>,
    panicked: Arc<AtomicBool>,
}

impl ReaderCounters {
    fn new() -> Self {
        Self {
            ok: Arc::new(AtomicU64::new(0)),
            cancelled: Arc::new(AtomicU64::new(0)),
            panicked: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// One reader thread's body: loop cloning a fresh snapshot and running a
/// salsa query against it until `stop` is set, exactly the "clone, query,
/// drop" pattern `tcl-lsp-server`'s request handlers use. On a genuine panic
/// (never expected — a `Cancelled` is caught separately and is healthy),
/// dumps the exact source text the failing query ran against before
/// returning, so the failure is reproducible without re-running under load.
fn reader_loop(
    reader_id: usize,
    db_handle: &Mutex<TclDatabase>,
    file: SourceFile,
    cfg: AnalyserConfig,
    counters: &ReaderCounters,
    stop: &AtomicBool,
    params: &RunParams,
) {
    let mut local_iters: u64 = 0;
    while !stop.load(Ordering::Relaxed) {
        let snapshot = db_handle.lock().expect("db mutex poisoned").clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            salsa::Cancelled::catch(|| {
                // Alternate between the two salsa queries every #829 fix
                // routes through the same cancellable incremental analysis,
                // mirroring the mix of `semanticTokens/full` and diagnostics
                // requests a real editing session sends concurrently.
                if local_iters.is_multiple_of(2) {
                    let _ = semantic_tokens(&snapshot, file, cfg);
                } else {
                    let _ = file_analysis_incremental(&snapshot, file, cfg);
                }
            })
        }));
        match result {
            Ok(Ok(())) => {
                counters.ok.fetch_add(1, Ordering::Relaxed);
            }
            Ok(Err(_cancelled)) => {
                counters.cancelled.fetch_add(1, Ordering::Relaxed);
            }
            Err(panic) => {
                counters.panicked.store(true, Ordering::Relaxed);
                let text = file.text(&snapshot).clone();
                eprintln!(
                    "reader {reader_id} PANICKED at local iteration {local_iters} \
                     (semantic_tokens/file_analysis_incremental): {panic:?}"
                );
                dump_repro_bundle(
                    &format!("reader{reader_id}-panic"),
                    &text,
                    params,
                    &format!(
                        "Reader thread {reader_id} panicked on iteration {local_iters} while \
                         running {} against the text in `fixture.tcl`. This must never happen — \
                         a cancellation should surface as `salsa::Cancelled`, not a panic.",
                        if local_iters.is_multiple_of(2) {
                            "`semantic_tokens`"
                        } else {
                            "`file_analysis_incremental`"
                        }
                    ),
                );
                break;
            }
        }
        local_iters += 1;
    }
}

/// The writer: repeatedly mutate the shared input, requiring exclusive
/// (`&mut TclDatabase`) access — the same write barrier
/// `Backend::db_set_source` goes through on every `didChange`. If any reader
/// were holding a clone across a query in a way that violates salsa's
/// cancellation contract, `set_text` would hang rather than return promptly,
/// and this loop would blow past `timeout` — the exact deadlock issue #829's
/// fix must prevent, so a timeout dumps the last successfully-written
/// revision as a reproduction bundle before returning. Returns the number of
/// writes actually completed.
fn writer_loop(
    db_handle: &Mutex<TclDatabase>,
    file: SourceFile,
    base_text: &str,
    params: &RunParams,
    started: Instant,
) -> usize {
    for generation in 0..params.writes {
        use salsa::Setter as _;
        let text = edited(base_text, generation as u64);
        let mut guard = db_handle.lock().expect("db mutex poisoned");
        file.set_text(&mut *guard).to(text);
        drop(guard);
        // A short pause so readers get a realistic chance to observe (and be
        // cancelled by) each revision, rather than the writer racing so far
        // ahead that every read is cancelled before doing any real work.
        std::thread::sleep(Duration::from_millis(2));
        if started.elapsed() > params.timeout {
            let elapsed = started.elapsed();
            eprintln!(
                "stress_concurrent_analysis: TIMEOUT after {generation} of {} writes \
                 ({elapsed:?} elapsed) — the writer is not completing promptly, which is the \
                 failure mode this suite exists to catch (issue #829: a reader must never block \
                 a concurrent edit indefinitely)",
                params.writes,
            );
            dump_repro_bundle(
                "writer-timeout",
                &edited(base_text, generation.saturating_sub(1) as u64),
                params,
                &format!(
                    "`set_text` for generation {generation} did not return within {:?} — a \
                     reader is (or several readers are) blocking the write, the deadlock this \
                     suite exists to catch. The dumped fixture is the last revision the writer \
                     successfully committed (generation {}) before the write that hung.",
                    params.timeout,
                    generation.saturating_sub(1),
                ),
            );
            return generation;
        }
    }
    params.writes
}

/// Post-stress correctness check: the database's *current* (last-written)
/// revision must still analyse correctly and be at least as rich as a
/// completely fresh, uncached direct analyse — proving the barrage of
/// concurrent cancellations never left the database in a state that produces
/// a wrong (not just slow) answer. Returns `true` on success.
fn final_correctness_check(
    db_handle: &Mutex<TclDatabase>,
    file: SourceFile,
    cfg: AnalyserConfig,
    base_text: &str,
    params: &RunParams,
    last_generation: u64,
) -> bool {
    let db_final = db_handle.lock().expect("db mutex poisoned").clone();
    let final_text = edited(base_text, last_generation);
    let final_result = salsa::Cancelled::catch(|| semantic_tokens(&db_final, file, cfg));
    let mut fresh_analyser = tcl_compiler::analyser::Analyser::new();
    let fresh = fresh_analyser.analyse(&final_text, &params.dialect);
    let registry = db_final.registry(&params.dialect);
    let fresh_tokens = tcl_lsp_core::semantic_tokens::full_with_cu_and_analysis(
        &final_text,
        &params.dialect,
        registry,
        None,
        Some(&fresh),
    );

    match final_result {
        Ok(tokens) => {
            // The salsa result may legitimately include CU-derived enrichment
            // (regex retag / object dispatch) the `None`-cu fresh baseline
            // above cannot reproduce, so this only checks that it is at least
            // as rich (never fewer tokens) — a proxy for "not
            // corrupted/truncated", not byte-for-byte equality.
            if tokens.data.len() < fresh_tokens.data.len() {
                eprintln!(
                    "stress_concurrent_analysis: FINAL CHECK FAILED — salsa result has fewer \
                     tokens ({}) than a fresh analyse ({}); concurrent access may have left the \
                     database in a truncated state",
                    tokens.data.len(),
                    fresh_tokens.data.len(),
                );
                dump_repro_bundle(
                    "final-check-truncated",
                    &final_text,
                    params,
                    &format!(
                        "The database's final revision (generation {last_generation}) answered \
                         `semantic_tokens` with {} tokens, fewer than the {} a fresh, uncached \
                         analyse of the same text produces — the concurrent stress run left the \
                         database in a truncated/corrupted state rather than merely a slow one.",
                        tokens.data.len(),
                        fresh_tokens.data.len(),
                    ),
                );
                false
            } else {
                true
            }
        }
        Err(_cancelled) => {
            eprintln!(
                "stress_concurrent_analysis: FINAL CHECK FAILED — the database never settled \
                 enough to answer a query for its own final revision"
            );
            dump_repro_bundle(
                "final-check-never-settled",
                &final_text,
                params,
                &format!(
                    "A `semantic_tokens` query for the database's own final revision \
                     (generation {last_generation}), run *after* every reader/writer thread had \
                     already stopped, still came back `Cancelled` — the database never reached a \
                     quiescent, queryable state."
                ),
            );
            false
        }
    }
}

fn main() {
    let params = RunParams {
        procs: env_usize("STRESS_PROCS", 600),
        readers: env_usize("STRESS_READERS", 8),
        writes: env_usize("STRESS_WRITES", 200),
        timeout: Duration::from_secs(env_usize("STRESS_TIMEOUT_SECS", 120) as u64),
        dialect: "tcl9.0".to_owned(),
    };

    println!(
        "stress_concurrent_analysis: procs={} readers={} writes={} timeout={:?}",
        params.procs, params.readers, params.writes, params.timeout
    );

    let base_text = generate_big_tcl(params.procs);

    let db = TclDatabase::default();
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
     0,);
    let file = SourceFile::new(&db, base_text.clone(), params.dialect.clone(), None);

    // Readers each hold their own `TclDatabase` handle, refreshed from the
    // shared `db_handle` before every query — the exact "clone a fresh,
    // short-lived snapshot" pattern `tcl-lsp-server` uses for every request
    // (see `DiagInputs::db`'s doc comment). A `Mutex<TclDatabase>` stands in
    // for the server's `Backend.db: Arc<Mutex<TclDatabase>>`.
    //
    // `db` is *moved* into the mutex, not cloned: an early version of this
    // harness left the original `db` binding alive (an unused, never-queried
    // clone sitting in scope for the rest of `main`) and every subsequent
    // `set_text` call below hung forever. That confirms in the strongest
    // possible way (a live deadlock, not just a doc comment) the invariant
    // `DiagInputs::db` documents: `set_text` requires *every* outstanding
    // clone of the storage to be gone, not merely idle or mid-query-free — so
    // any code path that keeps a `TclDatabase` clone alive without an active
    // reason to (a stray extra binding, a struct field that outlives its
    // request) permanently wedges every future write.
    let db_handle = Arc::new(Mutex::new(db));

    let counters = ReaderCounters::new();
    let stop = Arc::new(AtomicBool::new(false));

    let started = Instant::now();
    let mut handles = Vec::new();
    for reader_id in 0..params.readers {
        let db_handle = Arc::clone(&db_handle);
        let counters = counters.clone();
        let stop = Arc::clone(&stop);
        let reader_params = params.clone();
        handles.push(std::thread::spawn(move || {
            reader_loop(
                reader_id,
                &db_handle,
                file,
                cfg,
                &counters,
                &stop,
                &reader_params,
            );
        }));
    }

    let writes_completed = writer_loop(&db_handle, file, &base_text, &params, started);
    let timed_out = writes_completed < params.writes;

    stop.store(true, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }
    let elapsed = started.elapsed();

    let ok = counters.ok.load(Ordering::Relaxed);
    let cancelled = counters.cancelled.load(Ordering::Relaxed);
    let panicked = counters.panicked.load(Ordering::Relaxed);

    println!(
        "stress_concurrent_analysis: {elapsed:?} elapsed, {writes_completed} writes, \
         {ok} reads completed, {cancelled} reads cancelled (expected under contention), \
         panics={panicked}"
    );

    if timed_out {
        std::process::exit(1);
    }

    let correct = final_correctness_check(
        &db_handle,
        file,
        cfg,
        &base_text,
        &params,
        (writes_completed.max(1) - 1) as u64,
    );

    if panicked || !correct {
        std::process::exit(1);
    }
    println!("stress_concurrent_analysis: PASS");
}
