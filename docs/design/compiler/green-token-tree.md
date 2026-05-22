# Green token tree and incremental reparse

> **Status:** Design — phase 1 (shared tokenisation memo) is implemented and
> shipped; phases 2–5 below are the planned evolution. Tracking issue: #477.

This document specifies the lossless, context-aware token tree that replaces
the "re-lex on demand" model in the analysis pipeline, and the incremental
reparse that lets a single edit re-tokenise only the affected region. It is
the design lock-in for the remaining phases of issue #477.

## Why

Tcl tokenises **differently depending on context** (a braced body is opaque
until you re-lex it as a script; a command substitution is opaque until you
re-lex its inner text; double-quoted text suppresses word splitting;
expressions use a separate tokeniser). Because of this, the pipeline cannot
flatten the document once and reuse one stream everywhere — historically each
subsystem re-lexed the bytes it cared about, in the mode it needed.

Phase 1 (already shipped — see
[lexing-segmentation.md](lexing-segmentation.md#shared-tokenisation-memo))
introduced a per-analysis memo, `core/parsing/token_cache.py`, keyed by
`(base_offset, base_line, base_col, insidequote, text)`. It cut
`TclLexer.get_token` calls roughly in half on the trigger corpus and gave a
~22% analysis speed-up with byte-identical output. Its limits — and the
reasons the tree is needed — are:

1. **Two anchorings of the same bytes don't share.** A proc body is lexed at
   its *absolute* offset by the lowerer and `compiler_checks`, and again at
   *base 0* by `var_refs` (which only wants position-independent names).
   Those are different memo keys, so the body is still tokenised twice.
2. **The memo is per-analysis and discarded.** It cannot survive an edit, so
   it does nothing for per-keystroke latency.
3. **It memoises lexing, not structure.** Every consumer still re-walks the
   token stream to rebuild words/commands; there is no shared tree to descend.

The green tree fixes (1) and (3) structurally and is the substrate for the
incremental reparse that fixes (2).

## Goals and non-goals

**Goals**

- One tokenisation per `(region, mode)` per document, shared by the segmenter,
  the lowerer, `compiler_checks`, and `var_refs`.
- A lossless tree: whitespace, comments, and delimiters are preserved so the
  original text is reconstructable (needed for formatting and exact ranges).
- Lazy, memoised descent into braced bodies and command substitutions.
- Incremental update: an edit re-tokenises only the nodes it touches and
  shifts the offsets of following nodes by the length delta.
- Byte-exact positions preserved for all 16 position-sensitive LSP consumers,
  especially `SemanticTokensDelta`.
- Error-recovery nodes so a half-typed edit still yields a usable tree.

**Non-goals**

- Replacing the expression tokeniser (`core/parsing/expr_lexer.py`). Expression
  regions are attached to the tree as opaque `expr`-mode nodes whose contents
  are produced by the existing Pratt path; the tree owns *where* an expression
  is, not *how* it parses.
- Changing diagnostic semantics. The tree is an internal representation; the
  validation bar is byte-identical diagnostics (see
  [Validation](#validation)).
- Cross-document sharing. The tree is per-document; each open document owns one.

## A. The green tree node model

The tree is a *green tree* in the rust-analyzer sense: immutable nodes with
relative widths, plus a thin cursor layer that resolves absolute positions on
demand. Concretely:

```
GreenNode
  kind:      NodeKind            # ROOT | BRACED | BRACKETED | QUOTED | EXPR | WORD | ERROR
  mode:      Mode                # script | quoted | expr | raw
  width:     int                 # byte length of this node's full span (incl. delimiters)
  # exactly one of:
  tokens:    tuple[Token, ...]   # leaf: the lexed token stream for this region (in `mode`)
  children:  tuple[GreenChild]   # interior: ordered (gap_text | GreenNode) entries
  # lazily computed:
  _descended: GreenNode | None   # memoised re-lex of an opaque leaf as a child subtree
```

Key properties:

- **Relative widths, not absolute offsets.** A node stores its byte *width*,
  not its position. Absolute offsets are computed by a cursor that walks from
  the root accumulating widths. This is what makes incremental offset-shifting
  O(affected nodes) rather than O(document): re-tokenising a region only
  changes the widths on the path to the root.
- **Losslessness.** Interior nodes interleave `GreenNode` children with the
  literal `gap_text` between them (separators, comments, delimiters), so
  concatenating a node's full text reproduces the source exactly.
- **Leaf token streams carry positions relative to the node.** A leaf's
  `Token` offsets are 0-based within the node; the cursor adds the node's
  absolute base when a consumer asks for an anchored token. (This is the
  generalisation of today's `base_offset` lexer parameter.)

### Mode tagging is load-bearing

Every node is tagged with the mode it was tokenised in. The mode is **explicit
and tested**, because re-tokenising a region in the wrong mode silently
corrupts analysis (the issue's stated top risk). Modes:

| Mode | Region | Lexer behaviour |
|---|---|---|
| `script` | top-level, braced body re-lexed as code, `[..]` inner | full Tcl tokenisation, `$`/`[` active, comments at command start |
| `quoted` | inside `"..."` | substitutions active, no word splitting, no comments |
| `expr` | `expr` argument / `if`/`while` condition | handed to `expr_lexer`; node is opaque to the script lexer |
| `raw` | braced *data* word (e.g. `set msg {…}`), switch list body | never descended as script unless a consumer explicitly requests it |

The distinction between a braced **body** (`script`/descendable) and a braced
**data** word (`raw`/opaque) is driven by the command registry's argument
roles (`ArgRole.BODY` / `ArgRole.EXPR`), exactly as `var_refs` and
`compiler_checks` decide today. The tree records the *role-resolved* mode so a
later consumer never has to re-derive it.

## B. Lazy, memoised descent

A braced body or command substitution starts life as an **opaque leaf**: its
`tokens` is the single `STR`/`CMD` token the outer lex produced. The first
consumer that needs to see inside calls `node.descend()`:

- `descend()` re-lexes the node's inner text in the appropriate child mode
  (`script` for a body, `script` for `[..]` inner, `expr` via `expr_lexer`),
  builds the child subtree once, memoises it in `_descended`, and returns it.
- Every later consumer (segmenter, `compiler_checks`, `var_refs`) reuses the
  memoised child. This is what eliminates the per-nesting-level re-scan that
  makes `_parse_brace` O(depth) today.

`var_refs` becomes a pure tree walk: it descends body/expr nodes, reads
`VAR`/`CMD` leaves for names, and never re-lexes at base 0 — removing the
second anchoring entirely (limit (1) above).

## How the tree subsumes `token_cache.py`

The shipped memo is the degenerate, flat form of this tree: a dict from
`(anchoring, text)` to a leaf token stream. When the tree lands, `tokenise_cached`
becomes the leaf-construction primitive *inside* `descend()`, and the
ContextVar-scoped dict is replaced by the tree's own `_descended` memoisation.
`token_cache_scope()` at `Analyser.analyse` / `lower_to_ir` is replaced by a
`DocumentTree` owned by `DocumentState` (and constructed on demand for the
non-LSP `analyse(source)` / `lower_to_ir(source)` entry points). No consumer
keeps calling `TclLexer` directly except the tree builder itself.

## Consumer migration

| Consumer | Today | With the tree |
|---|---|---|
| `command_segmenter._segment_raw` | iterates a (memoised) flat token list, builds `SegmentedCommand`s | walks the root node's `script`-mode children; `SegmentedCommand` is produced from tree nodes, preserving today's fields |
| `compiler_checks._process_text` / `_recurse_body_arguments` | re-lexes each body via the memo | `descend()`s the body node and walks it |
| `var_refs` | lexes at base 0 via the memo | descends body/expr nodes; reads name leaves |
| `lower_to_ir` / `_lower_script` | `segment_commands(body)` | segments from the already-built body subtree |

`SegmentedCommand`'s public shape (`argv`, `texts`, `single_token_word`,
`all_tokens`, `range`, `expand_word`, `is_partial`) is preserved so the lowerer
and analysers are untouched. The tree changes *how* segments are produced, not
their contract.

## C. Edit-range inference

pygls is configured for **FULL** text sync, so `did_change`
(`lsp/lifecycle.py`) receives the entire new source, not edit ranges. The
handler infers the changed span by a common-prefix / common-suffix diff of old
vs new source:

```
start = first index where old[i] != new[i]
old_end, new_end = last indices where old[-j] != new[-j]
changed = (start, old_end, new_end)   # → delete old[start:old_end], insert new[start:new_end]
```

This is O(len) byte scanning (cheap) and yields a single contiguous
`(start, old_len, new_len)` span for the common single-edit case. Multi-region
edits collapse to one enclosing span — correct, just less selective.

## D. Incremental tree update

Given the changed span and the previous `DocumentTree`:

1. **Locate** the smallest node whose span fully contains the changed span by
   walking children with the width-accumulating cursor.
2. **Re-tokenise** only that node's region in its own mode (and re-`descend`
   lazily as before). Its width changes by `new_len - old_len`.
3. **Shift** following siblings/ancestors: because nodes store *widths*, not
   offsets, only the widths on the root path need updating — every untouched
   subtree is reused by reference, including its memoised descents.
4. **Feed the existing caches.** The chunk-level machinery in
   `lsp/workspace/document_state.py` (`segment_top_level_chunks`,
   `find_first_dirty_chunk`, per-proc `(name, body-hash)` cache) consumes the
   reused-vs-rebuilt boundary: unchanged chunks keep their cached IR / CFG /
   SSA / analysis; only dirty chunks re-analyse. The tree makes the
   "which chunk changed" decision exact (node identity) rather than
   hash-comparison.

### Offset bookkeeping is the classic hazard

The relative-width model is chosen specifically to make drift hard: a node's
absolute position is *never stored*, so it cannot go stale. The only mutable
quantity on an edit is the width of nodes on the path from the edited leaf to
the root. This is verified by property tests (below).

## E. Error-recovery nodes

Today `segment_with_recovery` (`core/parsing/recovery.py`) parses twice on
unterminated delimiters, injecting `VirtualToken`s (E201/E202/E203) on the
second pass. In the tree, an unterminated region produces an `ERROR` node that
**attaches** the recovered subtree rather than aborting — tree-sitter style.
The recovery heuristics (scan forward for a known command at a line start) are
preserved; they decide where the `ERROR` node ends and the next sibling
begins. Virtual-token insertions remain un-memoised (they are request-specific)
exactly as in the current memo.

## Validation

The bar is unchanged and strict:

- **Byte-identical diagnostics** on the trigger corpus (`wcswidth.tcl`,
  `http.tcl`, `filetypes.tcl`) and the 800+-file real-world differential, plus
  the full `make test-py` suite green at each phase.
- **Mode-correctness tests:** assert that each region's node mode matches the
  registry-resolved role, so a body is never lexed as data or vice versa.
- **Property test — incremental equals full.** For random source + random edit
  sequences, the incrementally-updated tree must equal a from-scratch reparse:
  same node structure, same leaf tokens, same absolute offsets. This is the
  guard against offset drift and is run with a large random-seed budget.
- **`SemanticTokensDelta` parity:** deltas derived from the reused-vs-rebuilt
  node boundary must match deltas computed from a full re-tokenise.

> Note for contributors: validate with `make test-py` (parallel, excludes the
> pyvm `test_vm_*_test.py` tcltest suite), **not** a bare `pytest tests/` — the
> latter runs the Tcl tcltest corpus through the Python bytecode VM
> single-threaded and is ~15× slower for no extra coverage of this subsystem.

## Phase mapping (issue #477)

1. **Shared tokenisation memo** — *shipped.* `token_cache.py`; consumers:
   segmenter, `compiler_checks`, `var_refs`.
2. **Green token tree** — sections A–B; replace the flat memo with lazy
   memoised descent; migrate the segmenter and `compiler_checks`.
3. **Edit-range inference** — section C; prefix/suffix diff in `did_change`.
4. **Incremental tree update** — section D; re-tokenise dirty nodes, width-shift
   the rest; wire into chunk/proc caches and semantic-token deltas.
5. **Error-recovery nodes** — section E.

Phases 2 is pure throughput (no LSP behaviour change). Phases 3–5 deliver true
incrementality and are where the per-keystroke latency win lives.

## Risks

- **Context-sensitivity bugs** (wrong mode on a region) silently corrupt
  analysis — mitigated by explicit mode tags and mode-correctness tests.
- **Offset drift** on incremental update — mitigated by the relative-width
  model and the incremental-equals-full property test.
- **`SemanticTokensDelta` fragility** — the most position-sensitive consumer;
  gated by delta-parity tests before phase 4 ships.
- **Scope creep into the expr parser** — explicitly out of scope; expressions
  stay opaque `expr` nodes.

## Pointers

- Memo (phase 1): [`core/parsing/token_cache.py`](../../../core/parsing/token_cache.py)
- Lexer / modes: [`core/parsing/lexer.py`](../../../core/parsing/lexer.py),
  [`core/parsing/expr_lexer.py`](../../../core/parsing/expr_lexer.py),
  [`core/parsing/tokens.py`](../../../core/parsing/tokens.py)
- Segmentation: [`core/parsing/command_segmenter.py`](../../../core/parsing/command_segmenter.py)
- Recovery: [`core/parsing/recovery.py`](../../../core/parsing/recovery.py)
- Re-lexing consumers: [`core/compiler/compiler_checks.py`](../../../core/compiler/compiler_checks.py),
  [`core/compiler/var_refs.py`](../../../core/compiler/var_refs.py)
- Pipeline: [`core/compiler/lowering.py`](../../../core/compiler/lowering.py)
- LSP sync / caches: [`lsp/lifecycle.py`](../../../lsp/lifecycle.py),
  [`lsp/workspace/document_state.py`](../../../lsp/workspace/document_state.py),
  [`core/common/document_buffer.py`](../../../core/common/document_buffer.py)

## Related docs

- [lexing-segmentation.md](lexing-segmentation.md) — current lexer/segmenter
  contract and the shipped memo.
- [error-recovery.md](error-recovery.md) — virtual-token injection.
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md) — stage map.
