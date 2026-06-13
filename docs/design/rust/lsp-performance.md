# Rust LSP — performance: results, optimisations, and how to measure

Authoritative record of the native (`tcl-lsp-server`) LSP performance work:
the Python-vs-Rust baseline, the optimisations shipped, their measured impact,
and how to reproduce every number. Companions:
[`current-architecture.md`](current-architecture.md) (the runtime model) and
[`incremental-analysis.md`](incremental-analysis.md) (the remaining per-item
work).

## The benchmark harness

`scripts/dev/bench_lsp_backends.py` drives both backends over JSON-RPC against
real Tcl projects and reports per-feature timings. It measures
time-to-semantic-tokens, heavy-edit re-analysis, time-to-full-diagnostics,
time-to-full-optimisation, and an open+index + all-features pass over a
multi-file corpus (tcllib modules + Tcl stdlib).

```
cargo build -p tcl-lsp-server --release
uv run python scripts/dev/bench_lsp_backends.py --json /tmp/bench.json
```

Large single files used: `tmp/tcllib-2.0/modules/practcl/practcl.tcl` (8463
lines), `tmp/tcl9.0.3/library/http/http.tcl`, `tmp/tcllib-2.0/modules/tepam/`.

## Python vs Rust (representative)

| Metric | Python | Rust | Speedup |
|---|--:|--:|--:|
| documentSymbol, 119-file corpus | 8,229 ms | **43 ms** | **190×** |
| open + index, 119 files | 54,156 ms | 277 ms | 196× |
| time→semantic tokens (practcl) | 790 ms | 23 ms | 34× |
| time→full diagnostics (practcl) | 18,901 ms | ~1,300 ms | ~15× |
| time→full optimisation (tepam) | 32,616 ms | 150 ms | 217× |
| hover (sample) | 1,500 ms | 13 ms | 114× |
| formatting (all) | 23,893 ms | 155 ms | 154× |

Rust wins 15–250× across the board. The two dimensions that needed work —
heavy-edit latency and documentSymbol — are covered below.

## Optimisations shipped (this work)

### 1. Crash fix — semantic-tokens OOB panic
`push_comment_tokens` advanced a byte cursor to each comment line's end while the
`chars()` iterator advanced one char, drifting past the buffer and slicing out
of bounds — crashing the server on any real file with several comments (tcllib
`json.tcl`, `practcl.tcl`). Rewritten with a `char_indices` cursor that can't
desync; regression test added. (The ASCII test fixtures were too short to drift,
so the suite missed it.)

### 2. salsa query database + cache elimination
The server kept five hand-evicted caches (`analyses`, `hover_cache`,
`semantic_tokens_cache`, `workspace_index`, `dialect_registries`), each
invalidated by hand in `did_open`/`did_change`/`did_close`. Four are now salsa
queries / a durable registry in `tcl-lsp-db`; invalidation is a single
`SourceFile` input bump. See [`current-architecture.md`](current-architecture.md)
§ *LSP server runtime*. This removed a class of stale-cache bugs (e.g. the
registry's load-order-dependent event-count flip) and the scattered eviction.

### 3. documentSymbol — reuse the cached analysis
`document_symbols` re-ran the full `Analyser::analyse` on every request
(~47 ms/file). It now serves from the memoised `document_symbols` query
(reusing `file_analysis`). **Warm documentSymbol across 80 files: ~3.8 s → 43 ms.**

### 4. Heavy-edit latency — async + debounced diagnostics
`did_change` *awaited* a full diagnostic re-analysis, **blocking the entire LSP
message loop**: after one edit, even a trivial no-analysis request stalled
~1080 ms. Diagnostics now run on a spawned, 50 ms-debounced task
(`schedule_diagnostics` → `run_diagnostics_core`); the loop returns immediately.
Three sub-bugs were fixed getting it robust: a `diag_inputs` self-deadlock
(struct-literal `MutexGuard` temporaries held across a re-lock), a salsa
global-write-exclusivity worker stall under edit bursts (the diagnostics path now
uses a direct `Analyser::analyse`, holding no salsa read-handle), and burst
coalescing via the debounce generation check.

**Post-edit interactive latency (practcl): ~1080 ms → 1–20 ms.** Validated by an
edit-storm e2e stress test passing under full-suite load.

### 5. Shared CompilationUnit (E7)
`lift_compiler_diagnostics` built one `CompilationUnit` for `run_all_checks` and
`optimise_with_dialect` built another. `optimiser::optimise_unit(cu, …)` now runs
the passes over the shared unit. **practcl edit→diagnostics: ~1.4 s → ~1.3 s.**

### 6. describeIruleEvent + event_satisfies (correctness)
`tcl-lsp.describeIruleEvent` returned a constant `validCommandCount` and read
`deprecated` from the wrong source; now mirrors the Python `_build_event_set`
(bit-exact out-of-event filtering) with a single canonical `event_satisfies` in
`tcl-registry`. (The exact cross-backend count remains a documented gap — the two
registries' command coverage differs; it is a port-in-progress, not a logic bug.)

## Where the remaining cost is

The ~1.3 s diagnostic latency on an 8.5k-line file is dominated by the
**whole-file `Analyser::analyse` walk** (~82%, measured); parsing is ~3 ms and the
optimiser/checks are ~18%. Reducing it requires **per-item incremental analysis**
(recompute only the edited proc), whose design, experiments (E1–E8), and
differential fuzzer are in [`incremental-analysis.md`](incremental-analysis.md).
That is the staged, multi-slice continuation; the interactive-latency blocker is
already resolved by the async-diagnostics work above.

## Reproducing the key numbers

| Number | Command |
|---|---|
| Python vs Rust table | `uv run python scripts/dev/bench_lsp_backends.py` |
| analyser walk vs tail split; lattice costs | `cargo run --release -p tcl-compiler --example incr_experiments` |
| heavy-edit interactive latency | edit + trivial-request timing via `.claude/skills/lsp-client` |
| e2e parity (514/1-known-red) | `make test-lsp-e2e-rust` |
