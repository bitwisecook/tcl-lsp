# Incremental analysis worker model

> **Status:** design (approved direction; not yet implemented). Part of the
> integrated incremental-computation initiative (rope + red-green tree +
> dependency graph). This doc covers *where the incremental state lives and how
> it is driven*; the data structures themselves are specified alongside the
> rope (`shared/rope.py`) and the green-tree / dependency-graph design.

## Why

Today every analysis is **stateless and off-process**. A lazy
`ProcessPoolExecutor` (`server/state.py:_get_process_pool`) is driven from two
`run_in_executor` sites in `server/diagnostics_pipeline.py` — basic/full
analysis and deep diagnostics (`_deep_coro` → `_run_deep_diagnostics`) — each
with an `asyncio.to_thread` fallback. Only `source` + config flags cross the
boundary; the worker **rebuilds the `CompilationUnit` from scratch** and pickles
results back. Every keystroke starts cold.

That model exists because analysis is currently expensive and whole-document, so
it must run off the asyncio event loop (a CPU-bound pass would otherwise freeze
hover/completion/semantic-tokens). But it fights the destination: an incremental
**dependency graph wants to persist across edits**, and a stateless pool task
that rebuilds everything cold cannot hold it. It also forces three pain points —
the per-edit cache must be pickled both ways, proc identity needs a
process-stable hash (Python `hash()` is salted per process), and the loop cannot
read the worker's graph for hover/refs.

## async vs threads vs processes (the runtime question)

On the current runtime — **CPython 3.11, standard GIL build** (free-threading is
3.13+, per-interpreter-GIL subinterpreters are 3.12+) — only processes give true
CPU parallelism. The analysis is CPU-bound pure Python.

| Mechanism | CPU parallel? | Shares memory with loop? | Verdict |
|---|---|---|---|
| **async only** | no | n/a | insufficient — a sync pass blocks the loop until it yields |
| **thread** | no (GIL) | **yes** | best for *incremental* edits: zero marshalling, direct reads; GIL released ~every 5ms so the loop stays responsive when per-edit work is small |
| **process** | **yes** | no (pickle) | needed only for the *cold/huge* build (one multi-second chunk) and cross-document fan-out |

**Key realisation:** the subprocess model is a consequence of analysis being
*cold and expensive*. The whole point of the incremental graph is to make each
edit's recompute *small* (only dirty nodes). Once per-edit work is milliseconds,
a worker **thread** holding a shared-memory graph is the natural fit and deletes
all three pain points at once. Processes are retained strictly for the cold path.

## The model

**Per-document persistent worker thread + shared in-process state; the process
pool builds the cold initial graph on open.**

1. **Shared state = `DocumentState` (extended).** It already provides exactly the
   needed contract: an atomic `_StateSnapshot` swap under an `RLock`, so readers
   see consistent state while a writer updates. Extend the snapshot to hold the
   persistent incremental graph (rope + green tree + dependency graph + per-node
   caches). The document's worker thread is the **single writer**; the event loop
   is the reader.

2. **Bounded worker pool + per-document single-writer lock** (decided). Warm
   edits already run on a shared thread pool (`asyncio.to_thread`,
   `diagnostics_pipeline.py:606`); we keep a *bounded* pool rather than spawning
   one OS thread per document. The single-writer invariant is enforced **per
   uri by a lock/queue** — at most one in-flight job per document, which the
   `diagnostic_scheduler` already provides by superseding stale versions — not
   by thread identity. This avoids head-of-line blocking (a job for A never
   queues behind B's in-flight analysis) with a *bounded* thread count; on the
   GIL build extra threads would add only latency fairness, which the bounded
   pool + per-doc serialization already gives.

3. **Coalescing edit queue = reuse `diagnostic_scheduler`.** It already
   supersedes stale versions per uri and `cancel(uri)`s on close. Repoint it to
   feed the document's worker thread instead of the pool. Only the latest
   version is analysed.

4. **Cold build → process pool, seeding the graph for a true warm start**
   (decided; **already in place** for the current artifacts). Every document's
   *initial* build runs in the bounded `ProcessPoolExecutor` (true parallelism +
   crash isolation for a potentially multi-second chunk, e.g. `filetypes.tcl`
   ≈ 24s). This split exists today (`is_fresh` branch). The returned `dict`
   already carries the full **`compilation_unit` + `analysis` + `chunks` +
   `chunk_caches` + `buffer`**, and `apply_subprocess_result` seeds them *plus*
   `_proc_cache` (`_do_update_proc_cache`) into `DocumentState` — so the *first*
   warm edit after `did_open` is already incremental, not a cold rebuild. The
   only outstanding seed work is the **future destination structures** (rope /
   green tree / dependency graph) once they replace the CU as the live artifact.

   **Measured (2026-05-26)** — serialize→unpickle round-trip of the result dict
   (this dict already crosses the pool boundary today):

   | file | lines | cold build | round-trip | % of cold | wire |
   |---|---|---|---|---|---|
   | `tcltk-man2html-utils.tcl` (typical) | 1.7k | 2.0s | 0.22s | 11% | 1.6 MiB |
   | `filetypes.tcl` (pathological, generated) | 85k | 30s | 5.9s | 20% | 24 MiB |

   Conclusions: (1) round-trip is always **« the cold build it saves**, so
   returning the graph for a warm first edit is a net win; (2) cost is dominated
   by `compilation_unit` + `chunk_caches` (the IR/green-tree-like structures;
   `analysis`/`chunks`/`buffer` are cheap), pickling at ~7 MiB/s — so only
   *pathological* generated files would want a compact / structurally-shared
   serialization, not typical sources; (3) **fixed:** the cold-build pool
   timeout was a fixed 15s — blown by the 30s `filetypes.tcl` build, which then
   re-ran the whole build in-thread — and now scales with document size
   (~30s/MiB over a 15s floor).

5. **Reads stay on the loop**, served directly from the document's latest
   snapshot — no thread hop, no IPC. An in-flight update simply means a reader
   sees the prior consistent snapshot until the worker swaps the next one.

## Concurrency & correctness

- **Single writer per document** (per-uri lock/queue; at most one in-flight job
  per doc) ⇒ no write-write races. The guarantee comes from per-uri
  serialization, not from a dedicated thread.
- **Atomic snapshot swap** ⇒ readers never observe torn state (today's pattern,
  `document_state.py:_swap_snapshot`).
- **Coalescing drops analyses, not text.** The scheduler may supersede
  intermediate versions; the worker always diffs from the *last-applied* rope to
  the *newest* text, never replaying skipped versions — so coalescing stays
  correct for stateful incremental updates.
- **Hash stability across the seed boundary.** The cold build runs in a
  subprocess, so any identity in the serialized graph must be process-stable
  (content-addressed / explicit IDs, not salted `hash()`), or in-process
  incremental updates will disagree with the pool-built seed.
- **Cancellation is cooperative.** A job already running on a worker thread
  cannot be preempted; superseding bounds wasted work to one in-flight recompute
  (small, since incremental). A cold build in a process can be abandoned by
  dropping its result.
- Per-edit GIL hold is bounded by *dirty-node* work (small); cold builds are off
  the thread (in a process), so a worker never holds the GIL for seconds.
- **Crash policy:** wrap each worker job in `try/except`. On an in-thread
  failure, log and fall back to a full rebuild via the pool — never let the
  worker die. The heavy cold path keeps process isolation regardless.

## Lifecycle

| LSP event | Action |
|---|---|
| `did_open` | submit the **cold build to the process pool**; on completion seed the in-process graph from the **serialized** result + publish. Until it returns, reads return a **pending/empty** result (never torn state) |
| `did_change` | enqueue under the doc's **per-uri single-writer lock** (coalesces/supersedes); worker diffs *last-applied → newest* text, applies incrementally, swaps the snapshot, publishes |
| read (hover/defn/refs/tokens) | served on the loop from the latest snapshot (pending until the cold build seeds it) |
| `did_close` | `diagnostic_scheduler.cancel(uri)`; release the doc's writer slot; drop the document state |
| `shutdown` | cancel all; drain the worker pool; shut down the process pool |

## Migration (incremental, gated — not a rewrite)

- **W1 — persistence + per-doc single-writer lock. (shipped)** The cold→pool /
  warm→thread split already existed (`is_fresh` branch). W1 added a **per-uri
  single-writer lock** around the analysis+publish path (`diagnostics_pipeline._publish_diagnostics`)
  so a document has at most one in-flight job, with an early supersession check to
  drop superseded edits before the expensive work. `CompilationUnit`/chunk/proc
  caches persist on `DocumentState` across edits. Behaviour-identical.
- **W2 — true incremental graph updates. (shipped)** Warm compilation was already
  incremental (dirty-chunk IR + per-proc `FunctionUnit` reuse + analyser
  snapshot/restore, gated by `tests/test_incremental_update.py`). W2 closed the
  remaining gap: the **deep-diagnostics** worker re-ran the body-local *shimmer*
  pass over the whole document each edit. The Phase 6 memoizer
  (`server/features/incremental_diagnostics.py`) is now wired into the live deep
  path — `proc_diag_infos` + `split_clean_dirty` target the pool's shimmer pass
  (`shimmer_target_procs`) at only the dirty procs; `merge_memoized_deep` reuses
  re-offset cached shimmer for clean procs and passes non-body-local codes
  through. The per-proc cache lives on `DocumentState`; the body_hash folds in
  the stub + CFG-context fingerprints (position excluded) for soundness. Gated by
  a new end-to-end test class in `tests/test_incremental_diagnostics.py`.
  The rope is also reused across warm edits end-to-end: `update_source_quick`
  splices the edit into the prior rope (`DocumentBuffer.from_edit`, O(log n)),
  and `_carry_or_build_buffer` carries that buffer through the full-analysis
  snapshot instead of rebuilding the O(n) position index — gated by from_edit
  equivalence tests + a reuse test in `tests/test_document_buffer.py`.
- **W3 — direct reads. (already in place)** Every read handler in `server.py`
  resolves `analysis = state.analysis if state else None` and injects it into
  `get_hover` / `get_definition` / `get_references` / …, which only recompute
  `analyse(source)` as a None-fallback. So reads already consume the cached
  in-process analysis — hover even returns early when it is still pending rather
  than re-parsing on the request thread. Hover's `_infer_var_type` /
  `_infer_var_taint` previously each recomputed `analyse_source(source)` (two
  full passes per variable hover); they now read the per-function
  `FunctionAnalysis` (`.types`/`.taints`) from the cached `compilation_unit`
  (`ModuleAnalysis` and `FunctionUnit.analysis` share the `FunctionAnalysis`
  type), with `analyse_source` kept only as the no-CU fallback. Gated by
  CU-vs-recompute equivalence tests in `tests/test_hover.py`.

## Deferred — persistent proc-to-proc dependency graph

A "callers only" invalidation graph is **deliberately not built.** The expensive
work is already incremental: `_proc_cache` reuses each unchanged proc's
`FunctionUnit` (cfg/ssa/types) keyed by body + stub + CFG-context fingerprint,
and W2 memoizes its body-local diagnostics. What a dep graph would add is
skipping the analyser's *sequential* re-pass over clean chunks after the edit —
but the analyser accumulates scope/definition state in document order, so sound
"callers only" invalidation is subtle and risks violating the byte-for-byte
`incremental ≡ full` contract for bounded gain (the analyser pass is cheap
relative to compilation, which is already cached). Revisit only if profiling
shows the post-edit analyser re-pass is a real bottleneck.

**Also shipped:** the cold-build pool timeout now scales with document size
(was a fixed 15s that the 30s `filetypes.tcl` build blew, triggering an
in-thread re-run). **Already in place** (not a remaining task): the `did_open`
cold build seeds the in-process CU + analysis + chunk caches + `_proc_cache`
via `apply_subprocess_result`, so the first warm edit is already incremental.

Gates: the byte-for-byte `incremental ≡ full` contract
(`tests/test_incremental_update.py`), the full suite, and an LSP keystroke-latency
benchmark on `filetypes.tcl`. Fallback-to-full (via the pool) on any in-thread
miss or failure keeps correctness the invariant.

## Decisions (confirmed)

- Worker model: **hybrid** — a bounded in-process worker pool (per-doc
  single-writer lock) for incremental edits; process pool for the cold build and
  cross-document fan-out.
- Granularity: **bounded pool + per-document single-writer lock** (revised from
  one-thread-per-document — on the GIL it gives the same fairness with a bounded
  thread count, matching today's `to_thread`/pool structure).
- Cold-build seed: offload `did_open` to the pool, returning a **serialized
  graph** for a true warm start (pending pickle-cost + timeout-scaling measurement).
- Crash policy: per-job `try/except` + fall back to a pool rebuild.

## Related

- `docs/design/async-diagnostics-tiering.md` — the current basic/deep tiering.
- `docs/design/compiler/green-token-tree.md` — the green tree the worker will persist.
- `shared/rope.py` — the persistent rope the worker holds.
- `server/workspace/document_state.py` — the snapshot-swap container being
  extended into the shared graph host.
