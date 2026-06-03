# The canonical concrete syntax tree (red-green CST)

> **Status:** Cycles 1–2 shipped. The position-independent green tree, the lazy
> red overlay, and the constructor are live in `compiler/parsing/syntax/`;
> `command_segmenter._segment_raw` derives its `SegmentedCommand`s from the tree
> byte-identically (cycle 1); and lazy descent into braced bodies and `[…]`
> substitutions is in `syntax/descend.py`, byte-identical to the analyser's
> `green_tree` descent (cycle 2). Adoption by the formatter, minifier,
> `var_refs`, `compiler_checks`, the per-command tooling, and direct AOT lowering
> are the sequenced follow-ons (see [Roadmap](#roadmap)).

This document specifies the lossless, position-independent concrete syntax tree
(CST) that is becoming the **single representation** the whole pipeline rides
on: command segmentation, AOT lowering to the bytecode VM and WASM, the
formatter, the minifier, and the per-command `tcl` / `f5 irule` tooling. Today
each of those re-lexes the bytes it cares about — the formatter and minifier
each spin up their own `TclLexer`, the segmenter runs its own token loop,
`var_refs` and `compiler_checks` re-lex bodies. The CST replaces that with one
tree everyone descends.

## Why a *red-green* tree

The model is the Roslyn / rust-analyzer split:

- **Green** (`syntax/green.py`) is **position-independent**: a node knows only
  its *width* and its children, never an absolute offset. Two structurally
  identical regions (e.g. two empty `{}` words) can be the *same* object, and a
  green subtree is reusable verbatim when an edit shifts it — the property that
  makes incremental reparse and cross-edit caching cheap.
- **Red** (`syntax/red.py`) overlays a green tree with an *anchoring*
  (`base_offset` / `base_line` / `base_col`) and resolves absolute positions
  **lazily**, reproducing exactly the `Token` offsets/lines/columns the lexer
  emits.

This is deliberately the model the earlier *green token tree* (issue #477,
[`green-token-tree.md`](green-token-tree.md)) decided **not** to build: that
structure is a context-aware tokenisation memo whose tokens carry **absolute**
positions, because ~80 consumers read `Token.start.offset` as absolute. The CST
does not change those consumers — it is built *alongside*, verified
byte-identical, and adopted incrementally. (Naming wrinkle: `green_tree.py` owns
the name `GreenNode` but is a memo; the real CST lives under `syntax/`. Renaming
the memo is a deferred follow-up because it touches every `tokenise` /
`node_for` importer.)

## Node model

```
GreenNode   kind: DOCUMENT | COMMAND | WORD
            children: tuple[GreenNode | GreenToken, ...]
            expand_markers: tuple[GreenToken, ...]   # {*} prefixes (WORD)
            range_end_rel:  int | None               # COMMAND: see Ranges
            preceding_comment: str | None            # COMMAND: see Comments
            trailing: tuple[GreenTrivia, ...]         # DOCUMENT: dangling trivia

GreenToken  token_type: TokenType   # ESC | STR | CMD | VAR | EXPAND
            text: str               # lexer inner text  ("abc" for {abc})
            raw:  str               # full source slice  ("{abc}")  → width
            end_rel: int            # Token.end.offset − Token.start.offset
            in_quote: bool
            leading/trailing: tuple[GreenTrivia, ...]

GreenTrivia kind: WHITESPACE | EOL | COMMENT
            text: str
```

**Trivia is attached** (leading/trailing on the adjacent token), Roslyn-style,
so a `COMMAND` / `WORD` is pure syntax while every inter-word byte is still
preserved. A token's *narrow* width is `len(raw)`; its *full* width adds the
attached trivia. A node's full width is the sum of its children's — cached at
construction.

Two text fields are kept on a token because neither is derivable from the other
without re-encoding the lexer's delimiter conventions: `text` is what
`Token.text` carries downstream (inner), `raw` is what makes the tree lossless
(full span). `end_rel` is the position-independent shape that lets the red layer
reconstruct `Token.end` — which, by the #527 convention, sits on the last inner
character for a non-empty `{…}` but *on the closer* for an empty `{}`.

## Losslessness

Concatenating every element's `full_text` in document order reproduces the
source byte-for-byte. The constructor recovers each fragment's `raw` by
**start-to-start tiling**: the lexer advances its cursor monotonically, so
`source[tok[i].start : tok[i+1].start]` is exactly the bytes fragment *i*
occupies — delimiters included — which sidesteps the inner-end / empty-delimiter
(#527) convention entirely. A trailing comment that attaches to no command
(`puts hi ;# bye`) is held as `DOCUMENT.trailing`.

## Position-equivalence (the red layer)

`SyntaxTree(green, base…, text=…)` builds a region-relative line index (from the
known `text`, or the green tree's own reconstruction) and resolves a region
offset to an absolute `SourcePosition` exactly as `TclLexer._pos_at` does:
region-relative line bisect, then shift line by `base_line` and the *first*
line's column by `base_col`. A `SyntaxToken`'s raw start is `node_start +
leading_width`; its `to_token()` reproduces the lexer `Token` (type, inner text,
start, `end` via `end_rel`, `in_quote`). The red views are created lazily on
walk, so nothing is materialised that a consumer does not visit.

## Deriving `SegmentedCommand` (the first consumer)

`segment.segments_from_tree` walks the red tree and produces the segmenter's
public `SegmentedCommand` shape, byte-identical to the former hand-rolled loop:

- **`argv`** — one merged token per word (type/text/start from the first
  fragment, end from the last: the compound-word merge of `{a}b`).
- **`texts`** — `_word_piece` per fragment, reused from `command_segmenter`.
- **`all_tokens`** — every fragment *and* `{*}` marker, in document order.
- **`range`** — see below.
- **`preceding_comment`** / **`expand_word`** — see below.

Word-less commands (a line of only dangling `{*}` markers) are skipped, matching
the segmenter, which discards an `argv`-empty command.

### Ranges — the `word_boundary` rule

A command's faithful end is **not** geometric (last fragment's closer): the
segmenter derives it from a `word_boundary` tracker with a genuine quirk — `{*}`
advances the parser past a word without updating the boundary, so a trailing
dangling `{*}` (and even `a b{*}\`) yields a *stale* boundary the range falls
back from. The constructor replicates that tracker exactly and stores the result
as `range_end_rel`, a width **relative to the command's first token** (hence
position-independent). The red layer resolves it against the first token.

### Comments — `preceding_comment`

The segmenter's `last_comment` accumulates **across** dangling-`{*}` commands, so
a comment before a marker still attaches forward to the next real command. That
is non-local — the comment physically precedes the marker and must live in the
marker's leading trivia for losslessness — so the constructor computes
`preceding_comment` during the build (replaying the exact accumulate/blank-line-
reset rule) and stores it on the command. This doubles as the natural
"leading doc-comment" an AST wants.

## Descent into bodies (`syntax/descend.py`)

At its own level a `{…}` body or `[…]` command substitution is a single `STR` /
`CMD` token whose `raw` owns the full span — delimiters included. *Descending*
re-lexes its inner text (`token.text`) as a child tree anchored **one byte past
the opener** (`start.offset + 1`, `character + 1`), so the child owns the
delimiter-excluding interior with absolute positions matching where it sits in
the document — the "node owns its span, children exclude delimiters" shape,
realised for nested bodies. Descent is lazy and shares the lex memo (the child's
`build_document` tokenises through the same `green_tree.tokenise`).

- `descend_token(token, source)` descends an absolutely-positioned `STR`/`CMD`
  token; an unterminated region (closer absent — decided by `_terminated`, from
  the inner *length* so empty `{}`/`[]` classify correctly) yields a **recovered**
  child that still carries its inner tokens, the lossless representation of
  malformed input.
- `descend_command(cmd_name, args, arg_tokens, source)` descends each
  registry-resolved `ArgRole.BODY` argument (via `iter_body_arguments`); `EXPR`
  arguments stay with `expr_lexer` and data words are left opaque.

Because both this descent and the analyser's `green_tree.descend_token` /
`descend_command` tokenise the same inner text at the same anchor through the
shared memo, the descended token stream is **identical by construction** — the
parity is asserted directly (same child fragments, same terminated/recovered
classification) over the corpus, 8 000 randomised nested cases, and multi-level
descent (a substitution inside a descended body).

## Verification

The bar is byte-identity with the prior segmenter, established before wiring and
locked by the suite afterwards:

- **Losslessness** — `full_text == source` over the real-world corpus (157 Tcl
  8.6/9.0 library files + fixtures), 120k randomised sources, and the edge-case
  table in `tests/test_syntax_tree.py`.
- **Position-equivalence** — red fragment tokens equal the lexer's non-trivia
  token stream (offsets, lines, UTF-16 columns, `in_quote`) over the same
  corpus + fuzz, including multi-line and unicode bodies.
- **Segment byte-identity** — `segments_from_*` matched the former
  `_segment_raw` field-for-field (`range`, `argv`, `texts`, `single_token_word`,
  `all_tokens`, `preceding_comment`, `expand_word`) over the corpus, 120k
  randomised differential cases, and nested-body (non-zero base) anchoring,
  before the loop was replaced.
- **Full `make test-py`** green with the segmenter on the tree — the
  end-to-end proof that diagnostics, analysis, and AOT codegen are unchanged.

## Performance

Build + derive is ~1.55× the former bespoke loop on a library-file corpus
(build ~1.13×, derive ~0.42×). The dominant residual is the line index being
built twice — once in the lexer, once in the red layer; sharing the lexer's
index is the obvious follow-up. The overhead is amortised — and then erased — as
other consumers drop their own re-lexing onto the one tree. The lexing itself
(the real cost) is shared through the `green_tree` memo.

## Roadmap

Cycle 1 (foundation + segmenter) and cycle 2 (descent) are shipped. The
remaining follow-ons, each its own verified cycle:

1. ~~**Descent**~~ — *shipped (cycle 2).* `syntax/descend.py` re-lexes braced
   bodies and `[…]` substitutions as child CSTs, so a delimited region is a node
   owning its full span with delimiter-excluding children — what the formatter
   and lowering need to recurse.
2. **`compiler_checks`** descends the shared tree instead of `green_tree`,
   retiring its duplicate `_process_node` mini-segmenter (the immediate next
   cycle; byte-identical diagnostics gated by `test-py`).
3. **Formatter** and **minifier** onto the tree (they need exactly the lossless
   trivia model above), then **`var_refs`**.
4. **Per-command `tcl` / `f5 irule` tooling** reads structured command/word/arg
   nodes (registry-aware) rather than walking tokens.
5. **Direct AOT lowering** from an AST that points into the CST, retiring the
   `SegmentedCommand` intermediary on the hot path.

## Pointers

- Green layer: [`compiler/parsing/syntax/green.py`](../../../compiler/parsing/syntax/green.py)
- Red layer: [`compiler/parsing/syntax/red.py`](../../../compiler/parsing/syntax/red.py)
- Descent: [`compiler/parsing/syntax/descend.py`](../../../compiler/parsing/syntax/descend.py)
- Constructor: [`compiler/parsing/syntax/build.py`](../../../compiler/parsing/syntax/build.py)
- Segment derivation: [`compiler/parsing/syntax/segment.py`](../../../compiler/parsing/syntax/segment.py)
- Segmenter (consumer): [`compiler/parsing/command_segmenter.py`](../../../compiler/parsing/command_segmenter.py)
- Tests: [`tests/test_syntax_tree.py`](../../../tests/test_syntax_tree.py)

## Related docs

- [green-token-tree.md](green-token-tree.md) — the context-aware tokenisation
  memo (a different structure, despite the `GreenNode` name) and incremental
  reparse.
- [lexing-segmentation.md](lexing-segmentation.md) — lexer/segmenter contract.
