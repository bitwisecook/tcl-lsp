# Green token tree and incremental reparse

The lossless, context-aware token tree that replaces "re-lex on demand" in the
analysis pipeline, and the incremental reparse that lets a single edit
re-tokenise only the affected region.

Two design constraints shape the implementation and are easy to try to
"fix" wrongly:

- **Tokens keep absolute positions; nodes carry a `width`.** A
  rust-analyzer-style relative-width model would make offset-shifting O(path),
  but `Token.start.offset` is read as absolute by every position-sensitive
  consumer, so relative positions cannot be adopted byte-identically. Nodes
  store `width` for locating and shifting regions; leaves keep
  absolute-anchored tokens, and the unchanged tail is offset-shifted on edit
  (arithmetic, far cheaper than re-lexing) rather than resolved lazily.
- **`var_refs` does not route through the per-document tree.** It lexes at
  base 0 precisely so its text-keyed result cache (frozen sets of variable
  names) shares across different occurrences of the same body text and across
  documents — which a per-document, absolutely-anchored tree cannot reproduce.
  It consults the tree's leaf primitive but keeps its own result cache.

## Why

Tcl tokenises **differently depending on context** (a braced body is opaque
until you re-lex it as a script; a command substitution is opaque until you
re-lex its inner text; double-quoted text suppresses word splitting;
expressions use a separate tokeniser). Because of this, the pipeline cannot
flatten the document once and reuse one stream everywhere — historically each
subsystem re-lexed the bytes it cared about, in the mode it needed.

A flat per-analysis memo keyed by
`(base_offset, base_line, base_col, insidequote, text)` cuts token production
roughly in half on the trigger corpus, but it is only the degenerate form of
the tree. Its limits are why the tree exists:

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

- Replacing the expression tokeniser (`rust/tcl-lexer/src/expr_lexer.rs`). Expression
  regions are attached to the tree as opaque `expr`-mode nodes whose contents
  are produced by the existing Pratt path; the tree owns *where* an expression
  is, not *how* it parses.
- Changing diagnostic semantics. The tree is an internal representation; the
  validation bar is byte-identical diagnostics (see
  [Validation](#validation)).
- Cross-document sharing. The tree is per-document; each open document owns one.

## A. The green tree node model

> **Proposal, not a description of the current tree.** `TokenRegion` and
> `NodeKind` do not exist in the Rust tree — neither name appears in any Rust
> file, so sections A, B, and E below sketch a design rather than report one.
> Issue #1406 tracks the gap.
>
> What exists is the red-green concrete syntax tree under
> `rust/tcl-compiler/src/parsing/syntax/`: the position-independent `GreenNode`
> and its `SyntaxKind` (`green.rs`), the anchoring red overlay (`red.rs`), and
> `descend_token()` (`descend.rs`), which returns a `Descended` carrying the
> child region's `SyntaxTree` plus an `is_terminated()` flag. `SyntaxKind` has
> `Document`, `Command`, and `Word` — there is no error variant; a recovered
> `{…}` / `[…]` is signalled by `Descended::is_terminated()` returning `false`.
>
> One claim below is contradicted outright by the code: `green.rs` states the
> green layer "does not pointer-share subtrees, so there is no cross-edit
> reuse", so the memoised-descent and cross-consumer-sharing story in sections
> A and B is not what the current green layer provides. See
> [syntax-tree.md](syntax-tree.md) for the tree that does exist.

The proposed node model — a `TokenRegion` owning the tokenisation of one
region, tagged with its `Mode`:

```rust
struct TokenRegion {
    kind: NodeKind,        // Root | Braced | Bracketed | Quoted | Expr | Error
    mode: Mode,            // Script | Quoted | Expr | Raw
    text: String,          // the region's source text
    base_offset: u32,
    base_line: u32,
    base_col: u32,
    inside_quote: bool,
    width: u32,            // text.len() — used to locate/shift regions
    tokens: Vec<Token>,    // the lexed stream (positions ABSOLUTE)
    warnings: Vec<LexWarning>,
    descended: HashMap<(u32, Mode), TokenRegion>, // memoised child descents
}
```

Key properties:

- **Absolute positions + `width`.** A leaf's `Token` offsets/lines/cols are
  absolute (anchored at the node's `base_offset` / `base_line` / `base_col`),
  matching every consumer's expectation. `width = len(text)` is what the
  incremental layer uses to locate the edited region and offset-shift the
  unchanged tail; there is no cursor.
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
roles (`ArgRole::Body` / `ArgRole::Expr`), exactly as `var_refs` and
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

## The tree is the tokenisation memo

The tree subsumes what would otherwise be a flat tokenisation memo — a map from
`(anchoring, text)` to a leaf token stream. `tokenise` is the leaf-construction
primitive; the intern index above, plus each node's descended-children map, is
the memo. No consumer calls the lexer directly except the node builder.

## Consumers

| Consumer | How it reads the tree |
|---|---|
| segmenter | walks the root node's `script`-mode children; `SegmentedCommand` is produced from tree nodes |
| `compiler_checks` | descends the body node and walks it |
| `var_refs` | descends body/expr nodes and reads name leaves (through the leaf primitive, at base 0) |
| lowering | segments from the already-built body subtree |

`SegmentedCommand`'s public shape (`argv`, `texts`, `single_token_word`,
`all_tokens`, `range`, `expand_word`, `is_partial`) is preserved so the lowerer
and analysers see one contract regardless of how the segments were produced.
The tree changes *how* segments are produced, not their contract.

## C. Edit-range inference

pygls is configured for **FULL** text sync, so `did_change`
(`rust/tcl-lsp-server/src/lib.rs`) receives the entire new source, not edit ranges.
`infer_edit_range(old, new)` (`rust/tcl-compiler/src/parsing/syntax/build.rs`) recovers the
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

## D. Incremental tree update

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
   arithmetic via `rust/tcl-lexer/src/ranges.rs`, far cheaper than
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

## E. Error-recovery nodes

`segment_with_recovery` (`rust/tcl-compiler/src/analyser/syntax_checks.rs`) parses twice on
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

The descent half of this *is* live, in the CST's own form:
`descend_token()` (`rust/tcl-compiler/src/parsing/syntax/descend.rs`) descends
an opaque `Str` / `Cmd` token against the full source and returns a
`Descended` whose `is_terminated()` is `false` for an unterminated region — the
recovered inner tree is still built and returned rather than the descent
aborting. That flag, not a `NodeKind::ERROR`, is the representational hook;
consumers read the token stream rather than a kind, so the distinction is
byte-identical either way.

## Validation

The bar is strict:

- **Byte-identical diagnostics** on the trigger corpus (`wcswidth.tcl`,
  `http.tcl`, `filetypes.tcl`) and the 800+-file real-world differential, plus
  the full `make test-rust` suite green.
- **Mode-correctness + recovered-descent tests:** unit tests assert region
  modes, descent anchoring, and intern sharing. For the descent half that
  exists, `descend.rs`'s own tests
  (`descend_braced_body_is_terminated_and_lossless`,
  `descend_empty_brace_is_terminated`, `descend_unterminated_brace_is_recovered`)
  cover the terminated / recovered split via `Descended::is_terminated()`.
- **Property test — incremental equals full.** *Shipped*
  (`rust/tcl-compiler/src/parsing/syntax/build.rs`). For random source + random edit, the
  incrementally-rebuilt chunk list must equal a from-scratch
  `segment_top_level_chunks`: same chunk count, offsets, hashes, and every
  command's tokens / ranges / texts / `preceding_comment` / partial flags.
  Run over 4000 randomised edits including delimiter-unbalancing ones (which
  exercise the fallback). This is the guard against offset/column/comment
  drift.
- **`SemanticTokensDelta` parity:** because the rebuilt chunk list is
  byte-identical to a full pass, the existing semantic-token chunk cache and
  delta path are unaffected (covered by `rust/tcl-lsp-core/src/semantic_tokens.rs`).


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

- Red-green CST (the tree that exists): `rust/tcl-compiler/src/parsing/syntax/`
  — `green.rs` (`GreenNode`, `SyntaxKind`), `red.rs`, `descend.rs`
  (`descend_token`, `Descended`)
- Incremental reparse: `rust/tcl-compiler/src/parsing/syntax/build.rs`
- Offset-shift helpers: `rust/tcl-lexer/src/ranges.rs`
- Lexer / modes: `rust/tcl-lexer/src/lexer.rs`,
  `rust/tcl-lexer/src/expr_lexer.rs`,
  `rust/tcl-lexer/src/tokens.rs`
- Segmentation: `rust/tcl-compiler/src/segmenter.rs`
- Recovery: `rust/tcl-compiler/src/analyser/syntax_checks.rs`
- Re-lexing consumers: `rust/tcl-compiler/src/compiler_checks.rs`,
  `rust/tcl-compiler/src/var_refs.rs`
- Pipeline: `rust/tcl-compiler/src/lowering/`
- LSP sync / caches: `rust/tcl-lsp-server/src/lib.rs`,
  `rust/tcl-lsp-db/src/lib.rs`,
  `rust/tcl-lsp-db/src/lib.rs`

## Related docs

- [syntax-tree.md](syntax-tree.md) — the canonical **red-green concrete syntax
  tree** (CST), whose structural node is named `GreenNode` (distinct from the
  `TokenRegion` this document proposes). A *different* structure: the CST is
  position-independent (its `GreenNode`s carry only widths; a red overlay
  resolves absolute positions lazily), whereas this green token tree is a
  context-aware tokenisation *memo* whose tokens carry absolute positions
  because ~80 consumers read `Token.start.offset` as absolute. The CST is built
  *alongside*, verified byte-identical, and adopted incrementally; it rides this
  tree's `tokenise` memo for the leaf lexing.
- [lexing-segmentation.md](lexing-segmentation.md) — the lexer/segmenter
  contract and the tokenisation memo.
- [error-recovery.md](error-recovery.md) — ghost delimiter injection.
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md) — stage map.
