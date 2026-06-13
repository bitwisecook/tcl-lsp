# Feasibility — can we reach the target from here?

> Companion to [`review-findings.md`](review-findings.md) (current
> state) and [`target-architecture.md`](target-architecture.md)
> (destination). Synthesises a five-thread deep sweep of the Rust
> workspace at #548, each thread testing one architectural goal for
> **hard blockers vs. work-items**, with file:line evidence.
>
> **Verdict: yes — every goal is reachable, and there are no hard
> architectural blockers.** The foundations that are usually the
> expensive part of this kind of rewrite are already in place.

## Headline

Across all six goals — zero-copy, single tree, incremental reuse,
per-item cascade/invalidation, MVCC, and embedded sub-language
injection — the sweep found **no hard architectural blocker**. The
remaining work is additive and sequenceable. Several of the hardest
structural prerequisites are *already done*, which is why the target is
within reach rather than a rewrite.

## Correction to the earlier review — there is one parse tree, not two

The review's headline that "two parse representations coexist" is
**wrong**, and the sweep (plus direct verification) corrects it:
`segment_commands_local` (`segmenter.rs:408`) builds the red-green CST
and derives `SegmentedCommand` from it; the token-loop segmenter was
retired in #538 (CST-PORT) and survives **only** as a frozen oracle in
`tests/differential_segment.rs`. So "`SegmentedCommand` as a view over
the CST" is already shipped, and the differential harness is a
regression oracle, not a live dual-parser sync.

The underlying *re-derivation* concern is still real, just different:
the tree is rebuilt from scratch on every call (no incremental reuse),
`descend_*` re-lexes inner text rather than navigating
(`descend.rs:108`), ~40 ad-hoc sub-word `Lexer::new(...)` scans bypass
the tree, and ~29 `LineIndex::new` builds re-index per feature. That is
"one tree, rebuilt and re-scanned repeatedly" — not "two parsers." The
two existing docs are corrected accordingly.

## What is already done (the expensive foundations)

| Foundation | Evidence |
|---|---|
| Single CST spine — `SegmentedCommand` is a view; one parser | `segmenter.rs:408-424`; oracle only in `differential_segment.rs` |
| Position-independent green tree with **relative** offsets (the hardest rowan prerequisite); absolute positions lazy in red | `green.rs:1-34`, `697-728`; `red.rs:139-155` |
| Span-only tokens (16-byte `Copy`, no lifetime) | `tcl-lexer/.../tokens.rs:128-153` |
| Substitution isolated behind one `Cow`-returning fn | `tcl-lexer/.../substitution.rs:38` |
| IR already half-defers substitution (`value_needs_backsubst`) | `ir.rs:177`; consumed `codegen/statements.rs:100` |
| Pure analysis entry points; no global mutable state on the analysis path; `unsafe` forbidden; no clock/rand | `lowering/mod.rs:1979`, `optimiser/manager.rs:37`, `analyser/state.rs:292` |
| Server already snapshot-shaped: snapshot-then-drop, `spawn_blocking` workers, immutable-by-clone analysis, no lock held across `.await` | `tcl-lsp-server/src/lib.rs` (did_open/did_change drop guards before await) |
| Per-item data model (procs/classes keyed by qname; `FunctionUnit` isolation; per-proc shape inference) + a real call graph | `compilation_unit.rs:160-264`; `param_traits.rs:67-119`; `interprocedural.rs:766` |
| Differential corpus gating segmentation / codegen / fold | `tests/differential_*.rs` |

## Per-goal readiness

| Goal | Verdict | What's needed |
|---|---|---|
| **Single parse tree** | ✅ **Done** | — (retired token loop in #538) |
| **Zero-copy** | Reachable with a value model | A `Text { Span \| Owned }` (or rope) threaded through `segmenter::texts`, `argv_texts`, `ExprNode.*.text`, `Assign*.value` — fields that today eagerly `String` but mostly hold source slices. Substitution/concat/`::`-normalise are the genuine `Owned` cases. Green CST must use `Arc<str>`/relative spans, not absolute `Span`, to keep reuse. |
| **Positions from the tree** | Reachable (consolidation) | Route the ~29 `LineIndex::new` sites through one index beside the CST; lands the C1 UTF-16 fix once. |
| **Incremental reuse** | Reachable with work | `Rc`/`Arc` (or interned) green children so surviving subtrees reuse by pointer (today `children: Vec<GreenElement>` owned, `green.rs:320`); a reparse driver (edit-range → bounded re-lex → splice); descent reuse instead of re-lex (`descend.rs:108`). Foundation (position-independence, relative offsets) already done. |
| **Per-item cascade** | Reachable with substantial refactor | Per-item diagnostic buckets (today flat `Vec`, `types.rs:467`); use the existing call graph to scope invalidation; an incremental driver (none today — server evicts the whole result per keystroke, `lib.rs:1278`). Depends on the determinism fix below. |
| **MVCC** | Reachable with refactor | Add `version` to `DocumentState` (params already carry it, ignored); `Arc`-share `AnalysisResult` (it's `Clone`/immutable, never `Arc`-wrapped — deep-cloned per request); version-gate published results; `publish_diagnostics(.., Some(version))`. Additive — the server is already shaped like MVCC minus the version field. |
| **Sub-language injection** | Reachable with work (heaviest) | New `ArgRole::{Regexp,FormatString,Glob,…}` (enum is `Body`/`Expr`-only today, `arg_role.rs:11`); a foreign-node `SyntaxKind` (Tcl-only `Document`/`Command`/`Word` today, `green.rs:51`); a `descend` that dispatches by role (hardwired to Tcl `build_document` today, `descend.rs:108`); net-new regexp/format/glob/BIG-IP sub-grammar parsers. Substrate (red-green CST, `with_parts` anchoring, registry role-dispatch) is correct. |

## The cross-cutting prerequisite: determinism

The cascade and the differential tests both need deterministic,
order-stable output. The pipeline is pure (no global mutable state on
the analysis path, no clock/rand; `unsafe` forbidden) **except**:
**diagnostics are emitted in `HashMap`-iteration order and never
sorted** before the wire (`analyser/state.rs:1875` drives emission over
`cu.procedures: HashMap`; the comment there claiming "insertion order"
is factually wrong for a `HashMap`; the boundary `lift_analyser_diagnostics`
preserves order without sorting). Under the default `RandomState`,
identical inputs do not produce byte-identical diagnostic ordering
across processes.

This is a "stabilise output ordering" task, not a blocker — and the
optimiser already does it right (`optimiser/helpers/select.rs:37`).
**Fix:** sort `result.diagnostics` by `(span.start, span.end, code,
severity)` before return; optionally switch the hot iteration maps
(`Module::procedures`, `cfg.blocks`, `def_use.chains`, `sccp.values`,
`ssa.blocks`) to `BTreeMap`. Cheap and high-leverage — it both unlocks
memoisation and hardens the existing differential suite. (Two minor
notes: `expr_cache` is a process-global `OnceLock<Mutex<…>>` but is off
the analysis path and content-deterministic; `document_links` reads
`$HOME` but has a pure variant — lift it into query inputs.)

## Unlock ordering (what gates what)

Dependency-ordered so the cheap, de-risking wins land first:

1. **Determinism** — sort diagnostics. Gates the cascade and hardens
   differential tests. Cheapest; do first.
2. **Version + `Arc`-sharing** (MVCC core) — fixes C2, removes the
   per-request deep clones. Cheap, high user-visible value.
3. **Positions service + C1** — one index beside the tree; UTF-16 once.
4. **`Text` value model** — zero-copy; also informs green-tree text
   storage, so sequence it before the incremental work.
5. **Incremental green-tree reuse** — `Rc`/`Arc` children + reparse
   driver + descent reuse. The sustained-latency win.
6. **Per-item cascade** — per-item buckets + call-graph-scoped
   invalidation. The biggest refactor; rides on (1) and (5).
7. **Injection** — `descend` dispatch + sub-grammar parsers. Heaviest
   net-new; last. Registry data population (#548) runs alongside, as it
   supplies the dispatch typing.

## Risks / watch-items

- **`AnalyserSnapshot` is dormant *and* a weak foundation** — it's
  prefix-linear (not per-item) and deep-clones the whole `AnalysisResult`
  per checkpoint (`snapshot.rs:97`), and it only covers the cheap
  segment-walk, not the dominant CFG/SSA/interproc tail. Do **not**
  build the cascade on it as-is; the per-item model is the right shape.
- **Whole-module lowering transforms** (`inline_uplevel_passthrough`,
  `specialise_factories`, `compilation_unit.rs:212-221`) splice across
  items, so a single proc's CFG isn't trivially re-lowerable in
  isolation — the per-item boundary needs care (re-lower the proc plus
  its uplevel-inlined callees).
- **`analyse` rebuilds the registry 2–3× per call** (`state.rs:359`,
  `:453`, `diagnostics.rs:1809`) — make it a static input early; it's a
  free win that also matters under memoisation.
- **Green-tree zero-copy tension** — green must hold `Arc<str>` +
  relative range, not a document-absolute `Span`, or it loses the
  structural-sharing that makes reuse cheap.
- **Tie-break residue** — even the optimiser's stable sort falls back to
  `HashMap` order on exact `(start, priority, length)` ties
  (`select.rs:37`); add a total tie-break when stabilising.

## Bottom line

There is a clear way to achieve every goal. No hard blockers; the
costly structural groundwork (single CST, position-independent green
tree with relative offsets, span-only tokens, isolated substitution,
pure analysis, snapshot-shaped server, per-item data + call graph) is
already laid. Sequence the cheap unlocks — determinism, MVCC
version/`Arc`, the positions service — first: they de-risk the rest and
deliver C1, C2, and immediate perf wins on their own. The large refactor
(per-item cascade) and the heaviest net-new work (sub-language
injection) come afterwards, on foundations that already exist.

## Related

- [`review-findings.md`](review-findings.md) — current-state findings
  (now corrected on the single-tree point).
- [`target-architecture.md`](target-architecture.md) — the destination.
- [`rust-rewrite-registries.md`](../../../rust-rewrite-registries.md) —
  per-entry registry parity (#548), supplying the injection dispatch
  data.
