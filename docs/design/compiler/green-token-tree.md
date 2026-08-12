# Green token tree and incremental reparse

> **Status:** All five phases are implemented and shipped. The flat memo has
> been subsumed by `compiler/parsing/green_tree.py` (phases 1–2); the segmenter,
> `compiler_checks`, and `var_refs` consume it, with `compiler_checks`
> descending command substitutions and bodies through the tree. Incremental
> reparse from edit-range inference lives in `compiler/parsing/incremental.py`
> (phases 3–4). Unterminated descended regions are tagged `NodeKind.ERROR`
> (phase 5). Tracking issue: #477.
>
> **Phase-2 deviations from the original design, and why.** Two design
> assumptions collided with codebase realities and were reconciled:
>
> 1. *`var_refs` does not route through the per-document tree.* It lexes at
>    base 0 precisely so its text-keyed result-LRU (frozensets of variable
>    names) shares across different occurrences of the same body text and
>    across documents — sharing a per-document, absolutely-anchored tree
>    cannot reproduce. Its LRU already prevents re-tokenising repeated text,
>    so it consults the green-tree leaf primitive (`tokenise`) but keeps its
>    own result cache. The "two anchorings collapse to one" win in limit (1)
>    below is therefore *parity with the flat memo*, not a reduction: a body
>    is still lexed once at its absolute offset (shared by segmenter +
>    `compiler_checks` + lowerer via the tree) and once at base 0 by
>    `var_refs` (covered by its LRU).
> 2. *Tokens keep absolute positions; nodes carry a `width`.* The design's
>    relative-width-only model would make offset-shifting O(path), but
>    `Token.start.offset` is read as absolute by every one of the ~16
>    position-sensitive consumers, so converting to relative positions is
>    infeasible byte-identically. Nodes store `width` for locating and
>    shifting regions (phase 4); leaves keep absolute-anchored tokens and the
>    unchanged tail is offset-shifted on edit (arithmetic, far cheaper than
>    re-lexing) rather than resolved lazily.

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

Phase 1 (shipped, then subsumed by phase 2 — see
[lexing-segmentation.md](lexing-segmentation.md#shared-tokenisation-memo-now-the-green-token-tree))
introduced a per-analysis memo (the former `compiler/parsing/token_cache.py`
module, now removed and folded into `green_tree.py`), keyed by
`(base_offset, base_line, base_col, insidequote, text)`. It cut
`TclLexer.get_token` calls roughly in half on the trigger corpus and gave a
~22% analysis speed-up with byte-identical output. Its limits — and the
reasons the tree is needed — were:

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

- Replacing the expression tokeniser (`compiler/parsing/expr_lexer.py`). Expression
  regions are attached to the tree as opaque `expr`-mode nodes whose contents
  are produced by the existing Pratt path; the tree owns *where* an expression
  is, not *how* it parses.
- Changing diagnostic semantics. The tree is an internal representation; the
  validation bar is byte-identical diagnostics (see
  [Validation](#validation)).
- Cross-document sharing. The tree is per-document; each open document owns one.

## A. The green tree node model

> **Implementation note.** The original design here was a rust-analyzer-style
> green tree (relative widths + a cursor resolving absolute positions on
> demand). That was *not* implemented, because `Token` positions are read as
> absolute by every position-sensitive consumer (see the status note); the
> shipped `TokenRegion` therefore stores **absolute** positions and a `width`
> for shifting. The description below reflects the implementation.

A `TokenRegion` owns the tokenisation of one region, tagged with its `Mode`.
Concretely (`compiler/parsing/green_tree.py`):

```
TokenRegion
  kind:       NodeKind          # ROOT | BRACED | BRACKETED | QUOTED | EXPR | ERROR
  mode:       Mode              # script | quoted | expr | raw
  text:       str               # the region's source text
  base_offset/base_line/base_col, insidequote
  width:      int               # len(text) — used to locate/shift regions
  tokens:     tuple[Token, ...] # the lexed stream for this region (positions ABSOLUTE)
  warnings:   tuple[...]
  _descended: dict[(offset, Mode), TokenRegion]  # memoised child descents
```

Key properties:

- **Absolute positions + `width`.** A leaf's `Token` offsets/lines/cols are
  absolute (anchored at the node's `base_offset` / `base_line` / `base_col`),
  matching every consumer's expectation. `width = len(text)` is what the
  incremental layer uses to locate the edited region and offset-shift the
  unchanged tail (phases 3–4); there is no cursor.
- **Lazy descent, not interior child nodes.** A node does not eagerly hold
  interior `TokenRegion` children; instead `descend(token)` re-lexes an opaque
  `{…}` / `[…]` leaf on demand and memoises the child in `_descended`. Sharing
  of regions reached independently (segmenter vs `compiler_checks` vs lowerer)
  is provided by the analysis-scoped intern index, not by a parent→child link.

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
- Every later consumer (segmenter, `compiler_checks`) reuses the memoised
  child. This is what eliminates the per-nesting-level re-scan that makes
  `_parse_brace` O(depth) today. (`var_refs` keeps its own result-LRU — see
  the status note above.)

Cross-consumer sharing for regions reached *independently* (the segmenter
scanning a body, `compiler_checks` descending the same body, the lowerer
segmenting it) is provided by an analysis-scoped **intern index** keyed by
`(base_offset, base_line, base_col, mode, text)`. `descend()` registers its
children in that index, so a standalone `node_for(...)` lookup of the same
region returns the already-built node. The per-node `_descended` map is the
primary structure for nesting; the intern index is what lets the three
consumers meet on one node.

## How the tree subsumed the flat memo

The flat tokenisation memo (the former `token_cache` module) was the
degenerate form of this tree: a dict from `(anchoring, text)` to a leaf token
stream. It has been replaced by `green_tree.py`: `tokenise()` is the
leaf-construction primitive, the ContextVar-scoped intern index (above) plus
the per-node `_descended` map is the memo, and `green_tree_scope()` replaces
the former `token_cache_scope()` at `Analyser.analyse` / `lower_to_ir`. No
consumer calls `TclLexer` directly except the node builder. (Persisting one
tree per `DocumentState` across edits is phase 4.)

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

## C. Edit-range inference — *shipped*

pygls is configured for **FULL** text sync, so `did_change`
(`rust/tcl-lsp-server/src/lib.rs`) receives the entire new source, not edit ranges.
`infer_edit_range(old, new)` (`compiler/parsing/incremental.py`) recovers the
changed span by a common-prefix / common-suffix diff:

```
start   = first index where old[i] != new[i]
old_end, new_end = strip the common suffix, not crossing back past start
                   # → replace old[start:old_end] with new[start:new_end]
```

This is O(len) byte scanning and yields one contiguous span; multi-region
edits collapse to one enclosing span — correct, just less selective.
`offset_delta = new_end - old_end` is the shift applied to everything at/after
`old_end`. The inference runs in `DocumentState` (where both revisions are in
hand), not in `did_change`.

## D. Incremental tree update — *shipped*

The update is realised at the **top-level-command (chunk) granularity** the
rest of the pipeline already caches at, rather than by mutating a persistent
node tree in place. `incremental_top_level_chunks(old_source, old_chunks,
new_source, edit)` rebuilds the chunk list for the new source:

1. **Prefix reuse (verbatim).** Chunks whose tile ends *strictly* before the
   edit start are reused unchanged — same offsets, hashes, commands. (Strict:
   a chunk whose tile boundary *equals* the edit start has its `end_offset`
   pushed by an insertion there, so it goes into the window.)
2. **Suffix reuse (offset-shifted).** Chunks beginning after the first newline
   at/after `old_end` sit on lines strictly below the edit, so shifting their
   tokens/ranges by `(offset_delta, line_delta)` is exact — columns are
   unchanged because those tokens are on later lines. Shifting is integer
   arithmetic via `compiler/parsing/token_positions.py`, far cheaper than
   re-lexing.
3. **Window re-segmentation.** Only the span from one byte past the previous
   command's last character through the *first* post-edit command is
   re-tokenised. Starting in the gap before the next command (not at the next
   command's start) is what lets a comment there attach forward to the right
   command, matching a full pass.
4. **Boundary validation.** The first post-edit command (re-segmented in full
   context) must equal the old one shifted into place. If it does, the lexer
   has provably reached the same clean state at the reuse boundary, so the
   shifted suffix tail is sound; if not (an edit unbalanced a delimiter and
   bled across the boundary), the whole result is rejected.

The rebuilt list is **byte-identical** to `segment_top_level_chunks(new)`, so
the existing `find_first_dirty_chunk` + `_update_incremental` machinery (per
chunk IR / analyser-snapshot reuse, per-proc `(name, body-hash)` cache)
consumes it unchanged: same dirty boundary, same analysis reuse, just without
re-tokenising the unchanged prefix and suffix. Wired into both
`DocumentState._segment_chunks` paths (the quick source-only update and the
full analysis update).

### Why chunk granularity, and the fallback

A persistent in-place node tree with relative-width offset bookkeeping was the
original design, but `Token` positions are read as absolute by every
position-sensitive consumer (§ status note), so the realisable unit is the
chunk, whose `SegmentedCommand`s can be shifted wholesale. Any case the fast
path cannot *prove* equivalent — no clean multi-chunk suffix below the edit's
last line, a partial/recovered command in a reused region, or a failed
boundary validation — returns `None`, and the caller re-segments fully. The
`incremental == full` property test (4000 randomised edits, including
delimiter-unbalancing ones that exercise the fallback) is the guard against
offset/column/comment drift.

## E. Error-recovery nodes — *shipped*

`segment_with_recovery` (`compiler/parsing/recovery.py`) parses twice on
unterminated delimiters, injecting `VirtualToken`s (E201/E202/E203) on the
second pass; that recovery mechanism is **preserved unchanged** — it remains
the authority for top-level recovery and virtual-token insertions stay
un-memoised (request-specific), exactly as the design specifies.

What the tree adds is the lossless *representation*: descending an opaque
token whose closing delimiter is absent in the parent region yields a
`NodeKind.ERROR` node that still **carries the recovered inner token stream**
rather than aborting — tree-sitter style. Termination is decided by
`_delimiter_terminated` (the closing brace/bracket must sit one byte past the
token's inner content, in the parent region's text), which correctly handles
nesting because the lexer has already matched delimiter levels.

Because termination is a property of the *parent context* — not of the
region's own text — and the intern index is keyed without `kind` (so a shared
node's `kind` reflects whatever its first interner set), the ERROR distinction
is applied per descent via a thin wrapper over the shared (interned) tokens:
`descend` / `descend_token` return an `ERROR`-kind node that reuses the
interned token stream. The shared node's own `kind` is never relied upon.

This is live in `compiler_checks`, which descends command substitutions
(`_recurse_nested_commands`) and bodies (`_recurse_body_arguments`) through
`descend_token` against the full source — so the tree's descent (and its
ERROR tagging) is exercised in production, sharing tokenisation with the
lowerer rather than re-lexing. Consumers read the token stream, not the kind,
so the change is byte-identical; the ERROR kind is the representational hook
for future tooling (e.g. structural diagnostics).

## Validation

The bar is unchanged and strict:

- **Byte-identical diagnostics** on the trigger corpus (`wcswidth.tcl`,
  `http.tcl`, `filetypes.tcl`) and the 800+-file real-world differential, plus
  the full `make test-rust` suite green at each phase.
- **Mode-correctness + ERROR-node tests:** `tests/test_green_tree.py` asserts
  region modes, descent anchoring, intern sharing, and that unterminated
  `{...}` / `[...]` descents are tagged `NodeKind.ERROR` (terminated ones
  `BRACED` / `BRACKETED`).
- **Property test — incremental equals full.** *Shipped*
  (`tests/test_incremental_reparse.py`). For random source + random edit, the
  incrementally-rebuilt chunk list must equal a from-scratch
  `segment_top_level_chunks`: same chunk count, offsets, hashes, and every
  command's tokens / ranges / texts / `preceding_comment` / partial flags.
  Run over 4000 randomised edits including delimiter-unbalancing ones (which
  exercise the fallback). This is the guard against offset/column/comment
  drift.
- **`SemanticTokensDelta` parity:** because the rebuilt chunk list is
  byte-identical to a full pass, the existing semantic-token chunk cache and
  delta path are unaffected (covered by `tests/test_semantic_tokens_delta.py`).

> Note for contributors: validate with `make test-rust` (parallel, excludes the
> pyvm `test_vm_*_test.py` tcltest suite), **not** a bare `pytest tests/` — the
> latter runs the Tcl tcltest corpus through the Python bytecode VM
> single-threaded and is ~15× slower for no extra coverage of this subsystem.

## Phase mapping (issue #477)

1. **Shared tokenisation memo** — *shipped, then subsumed by phase 2.*
2. **Green token tree** — *shipped.* `green_tree.py`; sections A–B; lazy
   memoised descent + intern index; segmenter and `compiler_checks` migrated,
   `var_refs` on the leaf primitive.
3. **Edit-range inference** — *shipped.* `incremental.infer_edit_range`;
   section C; prefix/suffix diff in `DocumentState`.
4. **Incremental tree update** — *shipped.*
   `incremental.incremental_top_level_chunks`; section D; verbatim prefix +
   offset-shifted suffix + re-segmented window with boundary validation; wired
   into both `DocumentState._segment_chunks` paths and consumed by the existing
   chunk/proc caches.
5. **Error-recovery nodes** — *shipped.* Section E; `NodeKind.ERROR` tagging in
   `green_tree.descend` / `descend_token`, live in `compiler_checks`; the
   `recovery.py` virtual-token mechanism preserved as the recovery authority.

Phase 2 is pure throughput (no LSP behaviour change). Phases 3–4 deliver the
per-keystroke win by not re-tokenising the unchanged prefix/suffix on each
edit. Phase 5 gives the tree a lossless representation of malformed regions.

## Risks

- **Context-sensitivity bugs** (wrong mode on a region) silently corrupt
  analysis — mitigated by explicit mode tags and mode-correctness tests.
- **Offset / column / comment drift** on incremental update — mitigated by the
  boundary-validation step and the incremental-equals-full property test, with
  a full-re-segmentation fallback whenever equivalence cannot be proven.
- **`SemanticTokensDelta` fragility** — neutralised by the byte-identical
  guarantee: the rebuilt chunk list is indistinguishable from a full pass.
- **Scope creep into the expr parser** — explicitly out of scope; expressions
  stay opaque `expr` nodes.

## Pointers

- Green tree (phases 1–2): `compiler/parsing/green_tree.py`
- Incremental reparse (phases 3–4): `compiler/parsing/incremental.py`
- Offset-shift helpers (phase 4): `compiler/parsing/token_positions.py`
- Lexer / modes: `compiler/parsing/lexer.py`,
  `compiler/parsing/expr_lexer.py`,
  `shared/tokens.py`
- Segmentation: `compiler/parsing/command_segmenter.py`
- Recovery: `compiler/parsing/recovery.py`
- Re-lexing consumers: `rust/tcl-compiler/src/compiler_checks.rs`,
  `rust/tcl-compiler/src/var_refs.rs`
- Pipeline: `rust/tcl-compiler/src/lowering/`
- LSP sync / caches: `rust/tcl-lsp-server/src/lib.rs`,
  `rust/tcl-lsp-db/src/lib.rs`,
  `shared/document_buffer.py`

## Related docs

- [syntax-tree.md](syntax-tree.md) — the canonical **red-green concrete syntax
  tree** (CST), whose structural node is named `GreenNode` (distinct from this
  green token tree's `TokenRegion`). A *different* structure: the CST is
  position-independent (its `GreenNode`s carry only widths; a red overlay
  resolves absolute positions lazily), whereas this green token tree is a
  context-aware tokenisation *memo* whose tokens carry absolute positions
  because ~80 consumers read `Token.start.offset` as absolute. The CST is built
  *alongside*, verified byte-identical, and adopted incrementally; it rides this
  tree's `tokenise` memo for the leaf lexing.
- [lexing-segmentation.md](lexing-segmentation.md) — current lexer/segmenter
  contract and the shipped memo.
- [error-recovery.md](error-recovery.md) — ghost delimiter injection.
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md) — stage map.
