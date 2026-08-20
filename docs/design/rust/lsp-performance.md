# Rust LSP — latency design and how to measure it

How the native server (`tcl-lsp-server`) meets its latency contract: the
mechanisms that keep the message loop free, the two-tier delivery of semantic
tokens and diagnostics, the invariants those tiers rest on, and the harnesses
that measure them. Companions: [`current-architecture.md`](current-architecture.md)
(the runtime model) and [`incremental-analysis.md`](incremental-analysis.md)
(the per-item analysis the queries sit on).

The contract, from [`engineering-guide.md`](engineering-guide.md): time to
first semantic tokens is the headline metric, interactive latency after an edit
comes second, memory a distant third.

## The query database replaces hand-maintained caches

The server holds one salsa database (`tcl-lsp-db`) instead of per-feature
caches with hand-written eviction. `did_open` / `did_change` set the
`SourceFile` input — the single invalidation point — and every derived value
(analysis, symbols, tokens, folding ranges, the project indexes) invalidates by
dependency. Two consequences matter for latency:

- **No stale-cache class of bug**, and no eviction code on the edit path.
- **Reads are cancellable.** A read clones the db handle onto a
  `spawn_blocking` worker and catches `salsa::Cancelled`, so a superseded
  request unwinds instead of finishing work nobody wants.

`lift_compiler_diagnostics` builds **one** `CompilationUnit` and runs both
`run_all_checks` and `optimiser::optimise_unit` over it; the unit itself is a
tracked query (`compilation_unit`, keyed on an interned lexer config) so the
analyser tail and the compiler/optimiser checks share a single build per edit
whenever their lexer configs coincide — every dialect but `tcl8.4` and
`f5-irules`.

## Diagnostics never block the message loop

`did_open` / `did_change` do not await analysis. They call
`schedule_diagnostics`, which spawns the work after a `DIAGNOSTICS_DEBOUNCE`
(50 ms) and returns immediately; a burst of edits collapses to one run via a
per-URI generation check. Post-edit interactive latency on an 8.5k-line file is
1–20 ms rather than the ~1 s the awaited run cost.

**The scheduler's config cache is epoch-stamped.** Resolving `DiagInputs` — a
dozen config mutexes, the per-folder overlay, the dialect's command registry —
is the expensive half of scheduling, so an ordinary keystroke reuses what the
slot already holds. That reuse is only sound while the configuration it was read
from is still current, which is what `Backend::diag_inputs_epoch` and
`DiagSlot::inputs_epoch` enforce: any path that writes state `diag_inputs` reads
calls `invalidate_diag_inputs`, and a schedule whose stamp is stale re-resolves
whether or not it asked to.

Without the stamp the config reload left a real window open. It applies the
switches first — which is also the instant `tcl-lsp.getEffectiveConfig` starts
reporting them — then walks the disk for `SpecTcl` packs, and only then
reschedules every open document. A keystroke arriving in between analysed under
the *previous* configuration, so O-codes a user had just disabled came back for
that one publish and cleared a moment later (issue #1651). Widening the
reschedule cannot close that window; only what the edit itself reads can.

The diagnostics path takes its base analysis through the **cancellable per-item
query** (`file_analysis_incremental`), not the coarse whole-file one. That is a
liveness requirement, not a preference: salsa's `set_text` takes global write
exclusivity, and it runs under the server's db mutex, so any read handle held
across an uncancellable analysis would block the next edit's write and stall
every other reader with it. The per-item query has a cancellation checkpoint at
each proc/method body, so an edit's `set_text` flips the cancel flag and the
in-flight read unwinds at the next item boundary.

**Parallel deep pass.** The three independent whole-file passes — the per-file
analyser walk (~82% of deep-pass wall-clock), the compiler/optimiser checks, and
cross-file resolution — run under one `tokio::join!` in `run_deep_diagnostics`,
collapsing the deep pass towards its longest single pass. The only thing given
up is fail-fast on a base-analysis cancellation: the other passes may do a
little wasted work before observing the same cancellation.

## Two-tier delivery: good value early, full value when ready

Both the token path and the diagnostics path race the enriched result against a
40 ms budget and serve a cheaper tier if it overruns.

### Semantic tokens

`semantic_tokens_core_data` races the enriched result against
`SEMANTIC_TOKENS_FAST_PATH_BUDGET` (40 ms) in a biased `tokio::select!`.
Whichever finishes first wins: a warm file serves the enriched result directly;
a cold or large file serves the coarse tier (`core_semantic_tokens::full` —
segmenter plus registry, no `CompilationUnit`) while the enriched computation
continues detached. When it finishes,
`SemanticTokensRefreshCtx::deliver_if_changed` diffs it against the last tokens
served for that URI and sends `workspace/semanticTokens/refresh` only if it
genuinely differs, so the editor re-requests and gets the enriched tier without
the server ever blocking on it.

`semantic_tokens_range` mirrors the tiering on a cache miss and converges the
same way: it takes the CU and analysis reads as `JoinHandle`s, and the ones the
budget drops are handed to a detached continuation that recomputes the
viewport-filtered range against the enriched unit and fires the same coalesced
refresh. Because a range response is never written to `last_semantic_tokens`,
the continuation diffs against the exact coarse vector it returned, so a static
cold viewport does not stay coarse until the next scroll or edit.

**Refresh coalescing.** `workspace/semanticTokens/refresh` carries no URI — it
asks the client to re-pull tokens for every open document — so many documents
finishing near-simultaneously (restoring several tabs) each firing their own
refresh is waste. `SemanticTokensRefreshCtx` collapses them with
`SEMANTIC_TOKENS_REFRESH_DEBOUNCE` (50 ms, matching the diagnostics debounce):
the first continuation to flip a shared `refresh_pending` flag owns one debounced
fire and any continuation arriving within the window rides along. Lossless,
because the fire is workspace-scoped regardless of which document triggered it.

### Diagnostics

The deep pass is raced against `DIAGNOSTICS_FAST_TIER_BUDGET` (40 ms). If it
overruns, a workspace-independent **fast tier** — the per-file analyser
diagnostics plus the pure source-style lints — is published first, and the deep
pass replaces it for the same version when it lands. Correctness rests on three
properties:

- **Flicker-safe partition.** The fast tier excludes exactly the codes a
  workspace or cross-file pass can *retract* — W120 and W123, classified once on
  `DiagCode::refined_by_workspace`, which the LSP layer only asks. The deep pass
  otherwise only *adds* diagnostics, so the fast tier is a strict subset and no
  fast-tier diagnostic is ever contradicted.
- **Currency and ordering.** The fast tier publishes strictly before the deep
  tier within a run (the deep future cannot publish until it is polled, after
  the fast publish), both are currency-guarded on the run's revision under the
  `documents` lock, and the per-document scheduler runs one worker at a time —
  so no edit can invert them and no cross-run interleaving is possible.
- **Push-only, deep-only pull.** The fast tier is push-only and never primes the
  pull cache; `textDocument/diagnostic` always serves or computes the complete
  deep set. A pull-capable client skips the fast tier entirely.

The fast tier is skipped for small documents by `DIAGNOSTICS_FAST_TIER_MIN_LINES`
(500): a trivial document's deep pass is dominated by one-time warm-up (registry
construction, the first salsa query), which can overrun a 40 ms budget on a cold
server even for a one-line file, so size gates the tier independently of machine
speed. The budget race is the other half — it suppresses the fast tier for large
but warm files whose memoised deep pass lands inside the budget.

**Workspace warm.** `project_class_index` and `project_proc_var_index` walk the
project's files serially, so a cold workspace's first enriched
`semantic_tokens_project` would analyse every file in that loop.
`spawn_workspace_warm`, kicked off detached after a workspace scan sets the
`Project` input, fans `file_analysis_incremental` across the blocking pool
(bounded by `WORKSPACE_WARM_MAX_CONCURRENCY`) so the tracked query's loop is all
cache hits. It is a pure cache optimisation: a concurrent real read dedups
against it, it never blocks a request, at most a permit's worth of snapshots is
live at once, and each read is the cancellable per-item query so a concurrent
`set_text` unwinds them at an item boundary.

Workspace-informed refinement (W120/W123, driven by the workspace index and
package resolver) is unconditional rather than gated on an opt-in toggle, so
`reschedule_all_open_documents` runs for every open document after the
`initialized` workspace scan and on workspace-folder or watched-file changes. An
editor that restores tabs before the scan completes does not keep a stale
false-positive W120 until the next edit.

## Document snapshots are shared, not copied

Every request handler works from a snapshot: `Backend::read_document` clones the
`DocumentState` out of the store and drops the lock, so handlers can cross
`.await` points and hand work to `spawn_blocking` without holding the mutex or
serialising against each other.

Both large fields are shared handles — `text: Arc<str>`, and `LineIndex`'s
backing store an `Arc<[u32]>` — so a snapshot is two reference-count bumps.
With `R` concurrent requests against a `B`-byte, `L`-line document, snapshot
memory is `O(B + 4L + R)` rather than `O(R × (B + 4L))`: on
`tcllib/modules/fumagic/filetypes.tcl` (1,320,164 bytes, 85,040 lines) a
snapshot copies nothing of the two large fields, where a deep copy was ~1.58 MiB
each. The two hottest consumers take the handle through rather than
re-materialising a `String`: `analysis_for` (whose common path is a cache hit
that never reads the text at all) and `DiagJob`, the per-run diagnostics
snapshot.

**The ownership contract.** `DocumentState` is both the mutable owner of an open
document and the immutable snapshot handed to readers, so sharing is safe only
under three rules, all documented on the type:

1. **Build-and-swap, never mutate in place.** `apply_content_change_indexed`
   splices into a fresh buffer and `LineIndex::apply_edit` builds a fresh
   vector; each is installed as a *new* handle. An outstanding snapshot observes
   the revision it was taken at for its whole lifetime, which is what lets an
   in-flight request finish against an older revision while the editor keeps
   typing.
2. **`text` and `line_index` are one snapshot, not two fields.** They are
   installed together in the single place that replaces them, so no reader can
   pair text from revision `N` with an index from revision `N-1`.
3. **Sharing is never mutable.** Nothing writes through an `Arc` a reader may
   hold; the one component that legitimately rewrites a buffer (the fix-all code
   action) takes its own owned copy.

Closed on-disk files are unchanged: `read_document`'s disk fallback mints a
fresh snapshot per read and does **not** install it in the open-document store,
so a cross-file read cannot retain every file it ever touched.

Pinned by `concurrent_snapshots_share_one_text_and_index_allocation` (32
overlapping readers share one allocation),
`an_edit_never_mutates_a_snapshot_an_in_flight_request_holds`,
`reading_a_closed_on_disk_file_does_not_retain_it`, and, in `tcl-lexer`,
`clone_shares_one_backing_allocation` /
`apply_edit_leaves_outstanding_clones_on_the_old_revision`.

## Where the remaining cost is

On a large file the deep pass is dominated by the analyser walk (~82% measured);
lexing and parsing are ~3 ms, the optimiser and checks the rest. Driving it
lower is the subject of [`incremental-analysis.md`](incremental-analysis.md) —
per-item recompute, the memoised lattices, and the whole-module lowering floor
that remains underneath them.

## Measuring

| Measurement | Command |
|---|---|
| analyser walk vs tail split; lattice costs | `cargo run --release -p tcl-compiler --example incr_experiments` |
| per-edit tail split (analyser / checks / taint solve) | `cargo run --release -p tcl-lsp-db --example tail_profile` (`FILE=` picks the document) |
| incremental-path fallback distribution | `cargo run --release -p tcl-compiler --example per_item_fallbacks` (`ROOT=` picks the corpus) |
| document-snapshot sharing | `cargo test -p tcl-lsp-server --lib -- concurrent_snapshots` and `cargo test -p tcl-lexer --lib line_index` |
| interactive latency (edit + trivial request) | drive the built server over JSON-RPC via the `lsp-client` skill |
| end-to-end LSP suite | `make test-rust` (runs `rust/tcl-lsp-server/tests/*_e2e.rs`) |

Large documents used for measurement:
`tmp/tcllib-2.0/modules/practcl/practcl.tcl` (8,463 lines),
`tmp/tcl9.0.3/library/http/http.tcl`, `tmp/tcllib-2.0/modules/tepam/`.
