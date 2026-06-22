# SRV-ROPE — rope-backed document store: need-evaluation & track design

> **Status:** Design + experiment. This document evaluates *whether* a
> rope-backed `DocumentState` is worth building (with measurements, not
> assertions), and scopes the **SRV-ROPE** track so that — if/when it is built —
> it touches every layer it must to actually pay off. It supersedes the bare
> "deferred (architecture)" bullet that previously sat in the SRV-LSP residuals.

## TL;DR — the decision

A rope **as a standalone `DocumentState` swap is not worth it**, and the
experiment quantifies why. A rope **only pays off as the front of a broader
incremental-analysis track** that also makes the lexer, segmenter, and the salsa
`SourceFile` input chunk-aware — because the per-edit bottleneck is *analysis*
(re-lex + salsa invalidation + diagnostics), which a rope alone leaves O(n).
Two further findings sharpen this:

1. **The paramount metric is untouched.** Time-to-first-tokens is a full-buffer
   `didOpen` build; there is no edit, so a rope changes nothing on the one
   latency the rewrite optimises for (§"Principle 1" in `rust-rewrite.md`).
2. **Most of the *apply*-side win is available without a rope.** The current
   `String` edit path is slow because it rebuilds `LineIndex` (a byte-by-byte
   `\n` scan) **and** double-allocates a spliced `String` on *every* content
   change. A *persisted, incrementally-patched `LineIndex`* on the existing
   `String` store captures the bulk of the position-lookup and a good slice of
   the edit win at **~zero memory cost** and a fraction of the churn — and it is
   the recommended **first** step (SRV-ROPE Task 1) before any rope lands.

So: keep the `String` store; land the cheap incremental-`LineIndex` win first;
build the rope only when the analysis pipeline itself goes incremental, at which
point the rope's O(log n) edits and rope-slice re-lex compound instead of being
masked. The track below is scoped for that end state.

## What the server does today (the baseline being measured)

Per `textDocument/didChange` (`rust/tcl-lsp-server/src/lib.rs`):

1. `apply_content_change` — for **each** content change: `LineIndex::new(text)`
   (O(n) `\n` scan, fresh `Vec` alloc) → `offset_at_utf16` → splice a brand-new
   `String` (two memcpys + alloc). N changes ⇒ O(N·n).
2. `entry.text = text.clone()` — an O(n) buffer copy.
3. `db_set_source` → salsa `SourceFile::set_text(db).to(text)` — stores the
   `String`, bumps the input revision, invalidates every dependent query.
4. debounced `file_analysis_incremental` — re-lex / re-analyse (memoised
   per-item, but still O(dirty) ≥ O(changed region), and O(n) in the worst case).

A rope changes (1) to O(log n) per edit and (2) to an O(1) `Arc`-share clone, but
**cannot** change (3)'s `String` requirement (salsa interns a `String`) nor (4)'s
re-lex — unless those layers also become chunk-aware.

## Experiment

Two reproducible, workspace-excluded harnesses, measuring the two halves of the
decision:

```
# (a) apply-side numerator — what a rope speeds up, in isolation:
cargo run --release --manifest-path docs/design/rope/experiment/Cargo.toml
# (b) per-edit denominator — what that apply is a fraction of (real analyser + salsa db):
cargo run --release --manifest-path docs/design/rope/experiment-pipeline/Cargo.toml
```

Harness (a) depends on the **production** `tcl-lexer::LineIndex` (so the `String`
arm measures real code) and `ropey` 1.6 (the rope the plan ratified). All inputs
are ASCII, so byte == char == UTF-16 code unit and both arms do the same
*logical* work — isolating the structural (rope vs flat-buffer) difference.
Numbers below are from one run on the dev box; they are **indicative ratios**,
not absolutes.

Harness (b) is the denominator the governing caveat below turns on: it drives the
real `tcl-compiler` analyser and `tcl-lsp-db` salsa queries across a warm-db edit,
so the apply cost can be read as a *fraction* of true per-edit latency rather than
estimated. It confirms that fraction is **~0.02%**, not the 5–15% the caveat
previously assumed (see below).

### Edit application — ns per `didChange` carrying B edits

Rope persists across edits (the fair steady-state model: each iteration starts
from an O(1) `Arc`-share clone, not a `from_str` rebuild). `flatten` is the
`Rope::to_string()` the salsa input forces; `rope_full = rope_edit + flatten`.

| size  | B  | string (ns) | rope_edit | flatten | rope_full | full ÷ string |
|------:|---:|------------:|----------:|--------:|----------:|--------------:|
| 1KiB  | 1  |         627 |       421 |     157 |       578 | 0.92× |
| 1KiB  | 64 |      60 186 |    15 434 |     164 |    15 598 | 0.26× |
| 16KiB | 1  |       8 664 |       972 |     824 |     1 796 | 0.21× |
| 16KiB | 64 |     575 298 |    16 727 |     841 |    17 568 | 0.03× |
|256KiB | 1  |     274 556 |     1 225 |  10 097 |    11 322 | 0.04× |
|256KiB | 64 |  11 966 694 |    15 771 |  10 312 |    26 083 | 0.00× |
| 1MiB  | 1  |   1 313 611 |     1 355 |  72 101 |    73 456 | 0.06× |
| 1MiB  | 64 |  38 756 220 |    14 958 |  73 209 |    88 167 | 0.00× |
| 4MiB  | 1  |   7 375 485 |     1 527 | 353 275 |   354 802 | 0.05× |
| 4MiB  | 64 | 159 531 003 |    17 363 | 323 230 |   340 593 | 0.00× |

Reading it:
- **Single edit (B=1):** rope_full beats `String` everywhere ≥16KiB (0.04–0.21×)
  and ties at 1KiB. The win is **not** the splice — it's avoiding the
  `LineIndex::new` rebuild + double-alloc the `String` path repeats per edit.
- **Flatten tax is small** relative to the current path: at 1MiB it's ~72µs vs
  the `String` path's 1.31ms (~5%). The tax is real but cheap *because the
  baseline is wasteful*; optimise the baseline and the tax becomes the floor.
- **Bursts (B=64):** rope is 20–500× faster — the deferred note's "only helps
  bursts" claim, now quantified. Editors rarely send burst `contentChanges`.

### High edit rate — 500 sequential single-edit `didChange`s (total ms)

Each edit hands salsa a fresh `String` (the rope arm flattens every edit too).

| size  | string (ms) | rope_full (ms) | speedup |
|------:|------------:|---------------:|--------:|
| 16KiB |         4.5 |            0.9 |   5.1× |
|256KiB |        70.0 |            5.7 |  12.4× |
| 1MiB  |       301.6 |           37.1 |   8.1× |

The rope is 5–12× faster on the **apply + flatten** machinery sustained — but
this is apply machinery, *not* the full per-edit pipeline. The headline caveat
(below) governs whether 5–12× on this slice moves the user-visible needle.

### Position lookup — byte offset → (line, col), ns per lookup

| size  | LineIndex::new (cold) | rope (persistent) | ratio |
|------:|----------------------:|------------------:|------:|
| 16KiB |                 8 203 |               224 |    37× |
|256KiB |               129 398 |               202 |   641× |
| 1MiB  |               544 415 |               408 | 1 334× |
| 4MiB  |             2 171 669 |               206 |10 542× |

A persistent line index (rope's, or a persisted `LineIndex`) avoids the
per-lookup rebuild. **Caveat:** the real server already amortises `LineIndex`
across a `lift_span` batch, so the *realised* gain is far below these
cold-rebuild ratios — but it is non-zero, and it is exactly the win a persisted
`String`-side `LineIndex` also captures (no rope required).

### Memory — many small open documents (heap bytes)

| N    | file  | strings | ropes | rope ÷ string |
|-----:|------:|--------:|------:|--------------:|
| 1000 | 2KiB  |    1MiB |  2MiB | 1.43× |
| 5000 | 1KiB  |    4MiB |  9MiB | 1.90× |
|  200 | 16KiB |    3MiB |  3MiB | 1.02× |

Rope's B-tree leaf chunks cost **1.4–1.9× memory for small documents** — a real
**downside** for the "lots of small files" workload (a workspace of many small
iRules / config snippets), and one a `String` store does not pay.

## Analysis against the dimensions asked for

- **Lots of small files:** rope *loses* — 1.4–1.9× memory, and per-edit apply at
  ≤16KiB is already sub-microsecond on `String`, so the apply win is irrelevant.
- **Large files:** rope *wins on apply* (single-edit apply at 4MiB drops from
  ~7.4ms to ~0.35ms), but see the caveat — apply is a fraction of total per-edit
  latency until the analysis pipeline is incremental.
- **High edit rates:** rope wins 5–12× on apply+flatten; whether that surfaces
  depends on the analysis-pipeline share of per-edit latency (caveat).
- **MVCC / salsa:** a rope **cannot** make salsa incremental. `set_text` interns
  a `String`, bumps the input revision, and invalidates dependents regardless of
  how the buffer is stored; the rope must *flatten* (O(n)) before every
  `set_text`, and the write-vs-read exclusivity (`set_text` waits for outstanding
  db-handle clones to drop — see the `DiagInputs` doc-comment) is untouched. Real
  incrementality requires the **input itself** to be chunk-addressable so salsa
  can intern unchanged chunks and the lexer re-lexes only the dirty span — that
  is the large, cross-crate scope the track must own, not a `DocumentState` swap.
- **Time-to-first-tokens:** unaffected (full-buffer `didOpen`, no edit).

### The governing caveat

Every "rope wins" above is on **edit application + position mapping** measured in
isolation. In the running server the per-edit critical path is dominated by
**re-lex + salsa invalidation + diagnostics**, all O(n) and rope-invariant today.
Harness (b) and the production profiler quantify the slice the rope can address —
and it is far smaller than the 5–15% this caveat originally estimated:

| measurement | per-edit latency | apply cost | **apply share** |
|---|--:|--:|--:|
| harness (b), 16KiB of procs | 282 ms | 8.5 µs | **0.00%** |
| harness (b), 64KiB of procs | 4.46 s | 34 µs | **0.00%** |
| `tail_profile`, `linalg.tcl` (2.3k lines) | 419 ms | ~85 µs | **~0.02%** |
| `tail_profile`, `practcl.tcl` (8.5k lines) | 1 623 ms | ~320 µs | **~0.02%** |

(`cargo run --release -p tcl-lsp-db --example tail_profile FILE=…`, warm db,
single-char edit — the real "both queries per `didChange`" server shape.) A 5–12×
speedup on a ~0.02% slice is invisible end-to-end. Two further facts deepen this:
**re-lex itself is tiny** (16–260 µs vs hundreds of ms of analysis — for
`linalg.tcl`, `run_all_checks` alone is ~411 ms of the 419 ms), and the
segmenter-level incremental path (`analyse_incremental`) plus the
`structural_index` / `reparse_window` substrate exist but are **test-only** — the
live salsa path re-lexes and re-segments the whole file every edit.

That is why the rope is **gated on the pipeline going incremental**: once re-lex
is bounded to the dirty span (the structural-state index from
[`compiler/error-recovery-rust-port.md`](../compiler/error-recovery-rust-port.md)
is the substrate), the rope's O(log n) edit + rope-slice re-lex compound, and the
flatten tax is replaced by handing the lexer a rope *slice* of the dirty region.
But even that is Amdahl-bounded: re-lex + re-segment is single-digit-ms against a
floor of hundreds of ms, so the dominant per-edit cost — CU lowering +
`run_all_checks` + `optimise_unit` — is attacked by the **rope-independent**
incremental-lowering track ([`rust/incremental-analysis.md`](../rust/incremental-analysis.md),
Approaches A/B), not by SRV-ROPE. The rope is the last and smallest lever, not the
first.

## The SRV-ROPE track (scope — "touches everything it needs to")

Ordered so each task ships independently green and the cheap wins land first.
Owns the document-store seam across `tcl-lexer`, `tcl-lsp-db`, `tcl-lsp-server`;
depends on the incremental substrate (FE-LEX CST descent + structural-state
index, already landed).

1. **Persisted incremental `LineIndex` on the `String` store** *(S, no rope, do
   first)* — hold the `LineIndex` beside `DocumentState.text`; patch it in place
   on edit (shift line-starts past the splice; add/remove entries for `\n` delta)
   instead of `LineIndex::new` per change; reuse it for `lift_span` / position
   lookups. Captures most of the position-lookup win and a chunk of the apply
   win at ~0 memory cost. Parity test: patched index byte-identical to a rebuilt
   one over a fuzz corpus of edits.
2. **Rope behind a feature flag in `DocumentState`** *(M)* — `Arc<ropey::Rope>`
   store; `apply_content_change` becomes a rope transaction returning
   `(rope, changed_byte_range)`; a burst of `contentChanges` applies as **one**
   transaction. Flatten to `String` for salsa at the seam (the documented tax).
   Gate behind config so it can be A/B'd against the `String` store; memory-
   regression guard for the many-small-docs workload.
3. **Rope-slice `SourceMap` in `tcl-lexer`** *(M)* — `LineIndex::from_rope_slice`
   + `Lexer::with_source_map` over a flattened dirty slice (the seam the
   lexer-vs-document-layer design in `rust-rewrite.md` already anticipates), so
   re-lex consumes a rope range without a whole-buffer copy. No rope reference
   leaks into `tcl-lexer`.
4. **Chunk-addressable salsa `SourceFile` input** *(XL, the real prize)* — make
   the salsa input chunk/line-addressable so `set_text` interns only changed
   chunks and `file_analysis_incremental` / `compilation_unit` re-lex + re-segment
   only the dirty span (bounded by the structural-state index). This is where the
   O(n) flatten + O(n) re-lex finally become O(dirty). Touches `tcl-lsp-db`
   (input shape + queries), `tcl-compiler::parsing` (incremental segmenter), and
   the recovery index. Differential gate: incremental re-lex/re-segment
   byte-identical to a from-scratch build over the edit-fuzz corpus.
5. **MVCC window** *(S, folds into 4)* — minimise the `set_text` write-lock hold;
   ensure the chunk input lets salsa share durably without an O(n) copy; measure
   write-vs-read contention under the high-edit-rate harness.
6. **Benches + gates** *(S)* — fold this experiment into a committed
   `perf_track`-style bench: assert **no time-to-first-tokens regression**
   (paramount metric) and track edit-latency + burst + many-small-docs memory as
   the track lands task-by-task.

**Exit criterion:** the rope is kept only if, with the pipeline incremental
(Task 4), the end-to-end per-edit latency on large files improves materially
*and* the many-small-docs memory regression is held under ~1.2×. If Task 1 (the
cheap incremental `LineIndex`) already captures the realistic win, Tasks 2–5 stay
deferred and the `String` store is retained — the experiment is the gate.
