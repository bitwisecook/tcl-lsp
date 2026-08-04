# Rust LSP — performance: results, optimisations, and how to measure

Authoritative record of the native (`tcl-lsp-server`) LSP performance work:
the Python-vs-Rust baseline, the optimisations shipped, their measured impact,
and how to reproduce every number. Companions:
[`current-architecture.md`](current-architecture.md) (the runtime model) and
[`incremental-analysis.md`](incremental-analysis.md) (the remaining per-item
work).

> **Update (2026):** Python is fully retired on this branch, so the
> Python-vs-Rust comparison below is now a **historical migration
> baseline** — the Python backend and the `bench_lsp_backends.py` harness
> that drove *both* backends no longer exist. The absolute Rust numbers
> and optimisations still stand; only the cross-backend reproduction step
> is retired (see "Reproducing the key numbers").

## The benchmark harness

The `bench_lsp_backends.py` harness drove both backends over JSON-RPC
against real Tcl projects and reported per-feature timings — time-to-
semantic-tokens, heavy-edit re-analysis, time-to-full-diagnostics,
time-to-full-optimisation, and an open+index + all-features pass over a
multi-file corpus (tcllib modules + Tcl stdlib). It was retired with the
Python backend; the native server is built and driven directly:

```
cargo build -p tcl-lsp-server --release
# drive the built server over JSON-RPC (e.g. via .claude/skills/lsp-client)
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

### 7. Semantic-token prioritisation (#829)

`semantic_tokens` fed the same coarse, uncancellable `file_analysis` query
diagnostics had already moved off (§4) — a large edit's diagnostics ran
detached and cancellable, but a concurrent `semanticTokens/full` request
queued behind the same uncancellable whole-file walk and could be starved
for the walk's full duration. It now shares `file_analysis_incremental`
with diagnostics (`project_class_index` / `project_proc_var_index` also
switched, matching the template `project_diagnostics` already used), so a
token request and a diagnostics run reuse the same per-item memoised
analysis instead of each re-walking the file.

Sharing the cancellable query fixes staleness but not first-response
latency on a very large file: the enriched analysis can still take longer
than an editor's request timeout on a cold document. `semantic_tokens_core_data`
now races the enriched result against a `SEMANTIC_TOKENS_FAST_PATH_BUDGET`
(40 ms) timer (`tokio::select! { biased; … }`). Whichever finishes first
wins: on a fast/warm file the enriched result serves directly; on a
cold/large file the 40 ms timer wins and the request serves the cheap
coarse tier (`core_semantic_tokens::full` — segmenter + registry only, no
`CompilationUnit`) while the enriched computation keeps running detached.
When it finishes, `SemanticTokensRefreshCtx::deliver_if_changed` diffs the
result against the last tokens served for that URI and sends
`workspace/semanticTokens/refresh` only if it actually differs, so the
editor re-requests and gets the enriched tier without the server ever
blocking the fast path on it. `semantic_tokens_range` mirrors the same
tiering on cache miss (serves from the coarse tier via
`range_with_cu_and_analysis(None, None, …)` instead of rebuilding a full
`CompilationUnit` inline) **and now converges the same way** (#844 Gap 4):
it takes the CU / analysis reads as `JoinHandle`s
(`db_compilation_unit_handle` / `db_file_analysis_handle`) so the ones the
budget drops are handed to a detached continuation that recomputes the
viewport-filtered range against the enriched unit/analysis and fires the
same coalesced refresh once it genuinely differs from the coarse range that
was served. Because a range response is never written to
`last_semantic_tokens`, the continuation diffs against the exact coarse
`Vec<u32>` it returned rather than the cache. A static cold viewport no
longer stays coarse until the next scroll/edit.

**Liveness invariant, not just latency.** The detached continuation that
waits for the enriched result past the budget holds an active salsa
read-handle for however long that computation takes — potentially seconds
on a large cold file. Salsa blocks a concurrent `set_text` until every
active read-handle releases, and `set_text` runs under the server's global
`db` mutex, so a stalled read here would stall every other read too. This
stays live only because the query is the cancellable, per-item
`file_analysis_incremental` (a cancellation checkpoint at each proc/method
body): an edit's `set_text` flips the cancel flag, the detached read
unwinds at its next item boundary, and the write proceeds promptly. Routing
`db_semantic_tokens` back through the coarse `file_analysis` would let one
cold background token computation serialise every subsequent edit behind a
whole-file walk — worse than the starvation this fix closes.

**Refresh-fan-out coalescing.** `workspace/semanticTokens/refresh` carries
no URI — it asks the client to re-pull tokens for every open document — so
many large cold documents finishing their enriched computation near
simultaneously (e.g. right after `initialized` restores several tabs) each
firing their own refresh is pure waste, not a correctness issue (VS Code
already coalesces them; other clients may not). `SemanticTokensRefreshCtx`
collapses this with a `SEMANTIC_TOKENS_REFRESH_DEBOUNCE` (50 ms, matching
`DIAGNOSTICS_DEBOUNCE`): the first continuation to flip a shared
`refresh_pending` flag owns one debounced fire; any continuation arriving
within the window sees the flag already set and rides along. Lossless
because the fire is workspace-scoped regardless of which document
triggered it.

A companion fix in the same investigation: workspace-informed diagnostic
refinement (W120/W123, driven by `workspace_index` / `package_resolver`
from `scan_workspace_folders`) is unconditional, not gated by the opt-in
`xcDiagnostics` toggle — so `reschedule_all_open_documents` (previously
`reschedule_xc_open_documents`, xc-only) now runs for **every** open
document after `initialized`'s workspace scan, and on workspace-folder /
watched-file changes. An editor that restores tabs before the scan
completes (racing `initialized`) no longer keeps a stale false-positive
W120 until the next edit.

### 8. Progressive, parallelised diagnostics pipeline (#844)

§7 made *semantic tokens* progressive (coarse now, enriched-via-refresh when
ready); #844 generalises that shape to the rest of the pipeline. The
principle: parallelise as much as possible, deliver good value early and full
value when it is ready.

**Parallel deep pass (Gap 2).** `run_diagnostics_analyser_path` ran three
independent whole-file passes back to back — the per-file analyser walk
(`file_analysis_incremental`, ~82% of the deep-pass wall-clock), the
compiler/optimiser checks (`compiler_check_diagnostics`), and the cross-file
resolution (`project_diagnostics`) — even though only the downstream
refine + lift consume all three. `run_deep_diagnostics` now runs them under one
`tokio::join!`, collapsing the deep pass towards its longest single pass. The
only thing given up is fail-fast on a base-analysis cancellation (the other
passes may do a little wasted work before observing the same cancellation) — a
trade the principle explicitly accepts.

**Progressive diagnostics (Gap 1).** Diagnostics were single-publish: the
client saw nothing until the deepest pass finished (~1.3 s on an 8.5k-line
file). The deep pass is now raced against a `DIAGNOSTICS_FAST_TIER_BUDGET`
(40 ms). If it overruns, a workspace-independent **fast tier** — the per-file
analyser diagnostics plus the pure source-style lints — is published first, so
the user gets the bulk of the diagnostics without waiting on the
compiler/optimiser + cross-file + refinement passes; the deep pass then lands
and replaces it for the same version. Correctness rests on three properties:

- *Flicker-safe partition.* The fast tier excludes exactly the codes a
  workspace / cross-file pass can *retract* — W120 and W123, classified once on
  `DiagCode::refined_by_workspace` (the single source of truth; the LSP
  `is_fast_tier` only asks it). The deep pass otherwise only *adds* diagnostics
  (compiler/optimiser, synthesised cross-file arity), so the fast tier is a
  strict subset of the deep tier and no fast-tier diagnostic is ever
  contradicted. Publishing an un-refined W120 would resurface exactly the
  startup false positive §7's `reschedule_all_open_documents` eliminated.
- *Currency + ordering.* The fast tier is published strictly before the deep
  tier within one run (the deep future cannot publish until `deep.await` polls
  it, which only runs after the fast publish), and both are currency-guarded on
  the run's revision under the `documents` lock, so a superseding edit can never
  invert them. The per-document coalescing scheduler runs one worker at a time,
  so no cross-run interleaving is possible either.
- *Push-only, deep-only pull.* The fast tier is `deliver_fast_tier_if_current`
  — push-only, and never primes the pull cache. The pull path
  (`textDocument/diagnostic`) always serves or computes the *complete* deep set;
  a pull-capable client skips the fast tier entirely (its "early" signal would
  be a refresh, but a re-pull recomputes the full deep set synchronously,
  defeating the purpose).

The debounce-skip has two halves. `DIAGNOSTICS_FAST_TIER_MIN_LINES` (500) is
the timing-independent floor: a trivial document's deep-pass wall-clock is
dominated by one-time warm-up (registry construction, the first salsa query),
which can overrun a 40 ms budget on a cold server even for a one-line file — so
size, not just the elapsed budget, gates the fast tier, keeping small files a
single publish regardless of machine speed. The budget race is the second half:
it suppresses the fast tier for *large but warm* files whose memoised deep pass
lands inside the budget.

**Parallel workspace warm (Gap 3).** `project_class_index` /
`project_proc_var_index` walk `project.files(db)` serially, so a cold
workspace's first enriched `semantic_tokens_project` serially analyses every
file. `spawn_workspace_warm` (kicked off detached after a workspace scan sets
the `Project`) fans `file_analysis_incremental` for every project file across
the blocking pool (semaphore-bounded to at most
`WORKSPACE_WARM_MAX_CONCURRENCY`), so the tracked query's loop is all cache
hits — the enrichment side's analogue of the deep pass's `join!`. It is a pure
salsa-cache optimisation (a concurrent real read dedups against it), never
blocks a request, and never stalls an edit: at most the permit count of
snapshots is live at once (each warm clones its snapshot only after acquiring a
permit), and each read is the cancellable per-item query, so a concurrent
`set_text` unwinds them at a per-item boundary rather than waiting them out.

### 9. Shared document snapshots (#1184)

Every request handler works from a snapshot: `Backend::read_document` clones
the `DocumentState` out of the store and drops the lock, so handlers can cross
`.await` points and hand work to `spawn_blocking` without holding the mutex or
serialising against each other. The snapshot was a **deep copy**: `text` was a
`String` and `line_index` owned a `Box<[u32]>`, so each of the ~55
`read_document` call sites copied the whole document plus four bytes per line,
and many handlers then cloned `doc.text` *again* to move it into a worker.

Both large fields are now shared handles — `text: Arc<str>`, and
`LineIndex`'s backing store is an `Arc<[u32]>` — so a snapshot is two
reference-count bumps. With `R` concurrent requests against a `B`-byte,
`L`-line document, snapshot memory goes from `O(R × (B + 4L))` to
`O(B + 4L + R)`. On the two corpus documents #1181 measures:

| document | bytes | lines | old: copied per snapshot | new: copied per snapshot |
|---|---:|---:|---:|---:|
| `tcllib/modules/practcl/practcl.tcl` | 263,816 | 8,463 | ~291 KiB | nothing |
| `tcllib/modules/fumagic/filetypes.tcl` | 1,320,164 | 85,040 | ~1.58 MiB | nothing |

("Nothing" for the two large fields — two reference-count bumps. A snapshot
still copies the short `dialect` and `language_id` strings, which are bounded by
the dialect vocabulary rather than by document size.)

A handler that snapshotted `filetypes.tcl` and then cloned `doc.text` for a
worker transiently copied ~2.84 MiB; it now copies nothing. The two hottest
consumers took the handle through rather than re-materialising a `String`:
`analysis_for` (~30 call sites — and its common path is a cache hit that never
reads the text at all) and `DiagJob`, the per-run diagnostics snapshot.

**The ownership contract.** `DocumentState` is both the mutable owner of an
open document and the immutable snapshot handed to readers, so sharing is only
safe under three rules, all documented on the type:

1. **Build-and-swap, never mutate in place.** `apply_content_change_indexed`
   splices into a fresh buffer and `LineIndex::apply_edit` builds a fresh
   `Vec`; each is then installed as a *new* handle. An outstanding snapshot
   observes the revision it was taken at for its whole lifetime, which is what
   lets an in-flight request finish against an older revision while the editor
   keeps typing.
2. **`text` and `line_index` are one snapshot, not two fields.** They are
   installed together in the single place that replaces them, so no reader can
   pair text from revision `N` with an index from revision `N-1` and resolve a
   position against the wrong offsets.
3. **Sharing is never mutable.** Nothing writes through an `Arc` a reader may
   hold; the one component that legitimately rewrites a buffer (the fix-all
   code action) takes its own owned copy.

Closed on-disk files are unchanged: `read_document`'s disk fallback mints a
fresh snapshot per read and does **not** install it in the open-document store,
so a cross-file read cannot retain every file it ever touched.

Pinned by `concurrent_snapshots_share_one_text_and_index_allocation` (32
overlapping readers share one allocation),
`an_edit_never_mutates_a_snapshot_an_in_flight_request_holds` (revision
currency, text *and* index together),
`reading_a_closed_on_disk_file_does_not_retain_it`, and, in `tcl-lexer`,
`clone_shares_one_backing_allocation` /
`apply_edit_leaves_outstanding_clones_on_the_old_revision`.

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
| Python vs Rust table | historical — the dual-backend `bench_lsp_backends.py` harness was retired with Python; the table is preserved as the migration baseline |
| analyser walk vs tail split; lattice costs | `cargo run --release -p tcl-compiler --example incr_experiments` |
| per-edit tail split (analyser / checks / taint solve) | `cargo run --release -p tcl-lsp-db --example tail_profile` (`FILE=` picks the document) |
| incremental-path fallback distribution | `cargo run --release -p tcl-compiler --example per_item_fallbacks` (`ROOT=` picks the corpus) |
| document-snapshot sharing (#1184) | `cargo test -p tcl-lsp-server --lib -- concurrent_snapshots` and `cargo test -p tcl-lexer --lib line_index` |
| heavy-edit interactive latency | edit + trivial-request timing via `.claude/skills/lsp-client` |
| native lsp_e2e suite | `make test-rust` (runs `rust/tcl-lsp-server/tests/*_e2e.rs` via `cargo test`) |
