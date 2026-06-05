# Target architecture — zero-copy, single-parse, incremental, MVCC

> Forward-looking companion to
> [`current-architecture.md`](current-architecture.md). Records the
> architectural destination the Rust workspace is converging on, set by
> the maintainer:
>
> - aggressively **zero-copy** — borrow from one source buffer, do not
>   re-materialise text;
> - **parse to tokens once** where the parse is statically knowable;
> - build **single trees** — one CST, no second parse representation;
> - derive positional data (cursor ↔ byte ↔ line/column) from the
>   **tokens underlying the CST**, not from per-feature indices;
> - let edits **cascade**: changing an input automatically invalidates
>   the portions of derived data that depend on it and rebuilds them on
>   demand;
> - use **MVCC** where concurrent reads and writes need a consistent
>   view.
>
> The current-state critique and the staged route from here are in
> [`review-findings.md`](review-findings.md); this doc is the
> destination, not the diff.

## The one tension, stated up front

Two of the goals pull against each other, and the resolution is a
deliberate decision, not a free lunch:

- **Aggressive zero-copy** wants tree nodes to hold a `Span` into a
  single `Arc<str>` source and slice text on demand (`&source[span]`),
  copying nothing.
- **Structural sharing across edits / files** (rowan's trick: identical
  subtrees share one allocation, so an edit reuses untouched subtrees
  verbatim) wants green leaves to **own** their text (interned
  `SmolStr`), because a position-independent node cannot borrow from a
  position-bearing buffer.

rust-analyzer chose owned text to get sharing. The maintainer's stated
priority is zero-copy, which points the other way. This is decision
**D1** below; the rest of the architecture is the same either way.

## Layered model

### Layer 0 — Versioned, immutable source

Each document version owns its bytes once: `Source { version: u64,
text: Arc<str> }`. This is the only allocation of the source for that
version and the unit of MVCC — versions coexist; a reader holds an
`Arc` to the version it started against. Today the server stores a
mutable `String` per URI with no version (`DocumentState`,
`lib.rs:98`); this replaces it.

### Layer 1 — Tokens, once

One lex pass per version produces a structure-of-arrays token stream:
`Tokens { source: Arc<str>, kinds: Vec<TokenKind>, spans: Vec<Span> }`.
A token's text is `&source[span.as_range()]` — **zero copy**. This
removes the per-token `text: String` + `raw: String` the green tree
stores today (`green.rs:139`) and the 135 ad-hoc re-segmentations
(`segment_commands*`): everything downstream consumes this one stream.
Incrementally, only the edited region re-lexes.

### Layer 2 — One CST, everything else a view

The red-green CST is built from the token stream and is the **single**
parse representation. The token-loop segmenter is retired;
`SegmentedCommand`, the IR item-tree, and the lowering input become
**views / projections** over the CST (iterators yielding command and
word spans), not separate owned parses. This closes the two-parse-tree
gap (`segmenter.rs` vs `parsing/syntax/`) and the hand-maintained
agreement harness (`differential_segment.rs`).

Green stores structure; the text-storage choice is **D1**. Red anchors
absolute positions lazily and carries the **one** line index beside the
tree (`SyntaxTree` already holds `line_starts`, `red.rs:57` — that
co-located index is the maintainer's explicitly-acceptable pattern).

### Layer 3 — Positions come from the tree

A single position service lives with the CST and is the only place
byte ↔ (line, column) and the UTF-16 conversion happen:

- cursor `(line, UTF-16 column)` → byte offset via the one
  `line_starts` index;
- byte offset → token / node via binary search over token spans (or
  red-tree descent).

This replaces the 29 independent `LineIndex::new` builds and lands the
C1 UTF-16 fix **once**, on the shared index, instead of at ~50 byte-
column call sites.

### Layer 4 — Demand-driven incremental graph (the cascade)

Derived data is expressed as **memoised queries** over inputs, with
dependencies tracked so a change invalidates exactly its dependents and
recomputes them lazily, only when next demanded:

```
input    source(file)          = version + Arc<str>
input    registry              = built-in command shapes (static)
derived  tokens(file)          ← source(file)
derived  cst(file)             ← tokens(file), command_shape(head)*  // shape-directed descent
derived  command_shape(name)   ← registry, else proc_shape(name)
derived  proc_shape(name)      ← analysis(def of name)               // discovered: signature_scan / param_traits
derived  item_tree(file)       ← cst(file)        // whitespace-stable firewall
derived  analysis(item)        ← item_tree(file), registry
derived  diagnostics(file)     ← analysis(item)*
derived  semantic_tokens(file) ← cst(file), registry
derived  workspace_index       ← item_tree(file)*
```

Three properties do the heavy lifting:

- **Firewall queries.** `item_tree` is keyed on structure, not bytes,
  so a whitespace-only edit re-lexes and re-parses but leaves
  `item_tree` unchanged — and every analysis downstream is reused. This
  is what makes "edit one character" cheap.
- **Per-item granularity.** `analysis(item)` is keyed per proc / class,
  so editing one proc invalidates only that proc's analysis, not the
  document's. Combined with CST subtree reuse, a keystroke recomputes a
  bounded slice, not the world.
- **Shape-directed descent.** How a command's arguments are parsed
  depends on `command_shape(head)` — registry-known for built-ins,
  discovered for user procs (D4). A proc's shape becoming known
  cascades a targeted re-parse of just its call sites, nothing more.

This replaces the current "rebuild everything every keystroke": the
`CompilationUnit` built 2–3× (`analyser/diagnostics.rs:1813`,
`optimiser/manager.rs:51`, `irules_checks.rs:464`), the registry
rebuilt inside every `analyse()` (`state.rs:359`), and the absence of
any reuse across edits. The registry becomes a static input, built once.

### Layer 5 — MVCC where reads and writes overlap

Reads run against an immutable snapshot; writes publish a new version:

- The current derived state is an immutable `Arc<Snapshot>` behind an
  `ArcSwap` (lock-free reads). A request `load()`s the current `Arc`
  and holds it for its duration — so it sees one consistent version,
  and cloning into a `spawn_blocking` worker is an `Arc` bump, not the
  deep clone of `AnalysisResult` done today.
- An edit builds the next version and swaps it in atomically. In-flight
  reads either complete against their (older) snapshot and have their
  result dropped by version tag, or are cancelled when they are now
  pointless.

This is the structural fix for C2 (no document-version guard,
`lib.rs:1111`), for the coarse per-map `Mutex`es that serialise
unrelated readers (`lib.rs:125`), and for the snapshot deep-clones. It
is "MVCC **where necessary**": the document and its derived snapshots,
not every small map.

## Key decisions to confirm

| | Decision | Options | Lean |
|---|---|---|---|
| **D1** | Green-leaf text storage | (A) `Span` into `Arc<str>` — true zero-copy, weaker cross-file dedup, incremental reuse by re-spanning (tree-sitter model); (B) owned interned `SmolStr` — rowan-style sharing and hash-consing, not zero-copy | **A**, per the stated zero-copy priority — but only if single-document zero-copy matters more than cross-file subtree sharing |
| **D2** | Incremental engine | (A) `salsa` — proven demand-driven memoised query graph with exactly the cascade/invalidate/rebuild semantics described; a dependency and a paradigm; (B) hand-rolled revision tags — lighter, coarser, more code over time | **A** (`salsa`) for the semantics; adopt behind the project's default-off-until-baked discipline |
| **D3** | MVCC flavour | finish-and-version-tag reads (complete, drop if stale) plus cancel-when-pointless (rust-analyzer's `Cancelled`) | both — tag for correctness, cancel for latency |
| **D4** | "Parse once" boundary | see below | once per *(span, known-as-script)*, memoised |

### D4 — "where knowable" is progressive: registry-known now, proc-shapes discovered

Tcl is contextually parsed — a `{...}` word is a string *or* a script
depending on the command — so "parse once" has two tiers of knowability
that resolve at different times.

- **Registry-known shapes — pass one.** For any command in the registry
  (every built-in), `CommandSpec` already encodes which arguments are
  bodies / scripts / expressions / variable names (`BodyKind`,
  `arg_roles`, `arg_role_resolver`). Descent is *deterministic*, not a
  guess: the first pass descends `if`'s condition as an expression and
  its arms as scripts, `proc`'s body as a script, `foreach`'s var list,
  and so on — each parsed once and memoised as a sub-tree.

- **Discovered shapes — cascade on first analysis.** A call to a *user*
  proc whose definition has not yet been analysed has an **unknown**
  shape. `customcmd $x {maybe a body}` is parsed conservatively — the
  braced word left as an opaque literal — because nothing yet says it
  is a script. When that proc's definition is analysed
  (`signature_scan` / `param_traits` infer that the parameter is used as
  a script via `eval $body`, an expression via `expr $cond`, or a
  variable via `upvar $name`), its shape becomes known and **the call
  sites that parsed it conservatively must re-parse the affected
  argument** — now descending the braced word as a script sub-tree.
  This is the maintainer's example: shape discovery drives targeted
  re-parsing of already-seen call sites.

In the graph this is one edge: a call site's descent depends on
`command_shape(head)` — the registry shape for built-ins, the derived
`proc_shape(name)` for user procs. When `proc_shape` transitions
unknown → known, the engine invalidates exactly the call-site parses
that read it and rebuilds them. That is "more reparsing as the shape is
discovered," automatic and bounded to the affected spans, rather than
the unconditional `re-lex on every visit` of today's `descend.rs:108`.

The apparent cycle — parsing a proc's body needs its shape, but the
shape is inferred from analysing the body — is shallow and terminating
in practice: a parameter's shape is fixed by how it is used with
*built-in* commands (`eval`, `expr`, `upvar`, …), which are
registry-known, so `proc_shape` is computable from the **pass-one,
registry-only** descent — no fixpoint over user code is needed. Where a
shape is genuinely undecidable (computed command names, dynamic
dispatch), the parse stays conservative — opaque word, generic call —
matching the codebase's existing "fall through to the generic `IRCall`
when it cannot be proven safe" rule.

This is also a *latency* win, not a cost. The conservative pass-one
parse needs only the static registry, so it paints fast (good for
time-to-first-semantic-tokens); the refined parse arrives later as an
ordinary cascade once shape discovery completes. Progressive
enhancement, not a blocking dependency — and `signature_scan`, the pass
that discovers these shapes, is already default-on.

## How the target resolves the open findings

| Finding (see review-findings.md) | Resolved by |
|---|---|
| C1 — byte columns, not UTF-16 | Layer 3 — one position service, one conversion |
| C2 — no version guard / stale overwrite | Layer 5 — versioned snapshots, MVCC |
| Two parse representations | Layer 2 — CST is the only parse; segmenter is a view |
| Re-derivation (135 / 29 / 34 rebuilds) | Layers 1, 3, 4 — parse and index once, reuse |
| `CompilationUnit` built 2–3× | Layer 4 — one memoised graph, shared |
| Registry rebuilt per `analyse()` | Layer 4 — static input, built once |
| Deep clones into `spawn_blocking` | Layer 5 — clone an `Arc`, not the data |
| Per-keystroke full recompute (TTFST) | Layer 4 — firewall + per-item granularity |
| Memory duplication (third priority) | Layers 0–2 — one buffer, one tree, `Span` not `String` |

## Migration

This is the destination of the convergence plan in
[`review-findings.md`](review-findings.md#convergence--one-parse-spine-without-a-big-bang-rewrite),
and it stays incremental for the same reason: `segment_commands*` is a
single chokepoint, the CST already exists and is differentially proven
against the segmenter, and `salsa` can wrap the existing pure functions
query-by-query. Suggested order: land the CST-as-spine (Layers 1–3)
first — it is pure refactor under the existing differential corpus and
delivers C1 and the zero-copy token stream — then introduce the query
graph (Layer 4) one query at a time, and finally the snapshot/MVCC
runtime (Layer 5), which is mostly server-side and closes C2.

The failure mode to avoid is adopting the *vocabulary* (snapshots,
queries) without the *firewall + per-item granularity* that makes the
cascade cheap — a memoised graph with one coarse `analysis(file)` query
re-runs the whole file on every keystroke and buys nothing.

## Related

- [`current-architecture.md`](current-architecture.md) — the crate
  graph and ownership rules this builds on.
- [`review-findings.md`](review-findings.md) — current-state findings,
  the duplication map, and the staged convergence plan.
- [`docs/rust-rewrite.md`](../../rust-rewrite.md) — chunking strategy
  and chunk log.
