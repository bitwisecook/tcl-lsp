# The canonical concrete syntax tree (red-green CST)

The lossless, position-independent concrete syntax tree (CST) is the **single
representation** the pipeline rides on: command segmentation, AOT lowering to
the bytecode VM and WASM, the formatter, the minifier, and the per-command
`tcl` / `f5 irule` tooling all descend one tree rather than each re-lexing the
bytes it cares about.

Living in `rust/tcl-compiler/src/parsing/syntax/`: the position-independent
green tree, the lazy red overlay, the constructor, lazy descent into braced
bodies and `[…]` substitutions, and the `SegmentedCommand` derivation.

## Why a *red-green* tree

The model is the Roslyn / rust-analyzer split:

- **Green** (`parsing/syntax/green.rs`) is **position-independent**: a node knows only
  its *width* and its children, never an absolute offset. Two structurally
  identical regions (e.g. two empty `{}` words) can be the *same* object, and a
  green subtree is reusable verbatim when an edit shifts it — the property that
  makes incremental reparse and cross-edit caching cheap.
- **Red** (`parsing/syntax/red.rs`) overlays a green tree with an *anchoring*
  (`base_offset` / `base_line` / `base_col`) and resolves absolute positions
  **lazily**, reproducing exactly the `Token` offsets/lines/columns the lexer
  emits.

This is deliberately the model the earlier *green token tree* (issue #477,
[`green-token-tree.md`](green-token-tree.md)) decided **not** to build: that
structure is a context-aware tokenisation memo whose tokens carry **absolute**
positions, because ~80 consumers read `Token.start.offset` as absolute. The CST
does not change those consumers — it is built *alongside*, verified
byte-identical, and adopted incrementally. (The `lexer.rs` memo node is named
`TokenRegion`; the CST's structural node under `syntax/` is `GreenNode`, so the
two no longer collide.)

## Node model

```rust
pub struct GreenNode {
    pub kind: SyntaxKind,             // Document | Command | Word
    pub children: Vec<GreenElement>,  // GreenElement::Node | ::Token
    pub expand_markers: Vec<GreenToken>, // {*} prefixes (Word)
    pub trailing: Vec<GreenTrivia>,   // Document: dangling trivia
    pub range_end_rel: Option<u32>,   // Command: see Ranges
    pub preceding_comment: Option<String>, // Command: see Comments
}

pub struct GreenToken {
    pub token_type: TokenType, // ESC | STR | CMD | VAR | EXPAND
    pub text: String,          // lexer inner text  ("abc" for {abc})
    pub raw: String,           // full source slice  ("{abc}")  → width
    pub end_rel: u32,          // Token.end.offset − Token.start.offset
    pub content_offset: u8,
    pub in_quote: bool,
    pub leading: Vec<GreenTrivia>,
    pub trailing: Vec<GreenTrivia>,
}

pub struct GreenTrivia {
    pub kind: TriviaKind,      // Whitespace | Eol | Comment
    pub text: String,
}
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

## Descent into bodies (`parsing/syntax/descend.rs`)

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

Because this descent's `build_document` and a direct `green_tree.tokenise` of the
same region share the one tokenisation memo (same inner text, same anchor), the
descended token stream is **identical by construction** — the
parity is asserted directly (same child fragments, same terminated/recovered
classification) over the corpus, 8 000 randomised nested cases, and multi-level
descent (a substitution inside a descended body).

## Verification

The bar is byte-identity with the prior segmenter, established before wiring and
locked by the suite afterwards:

- **Losslessness** — `full_text == source` over the real-world corpus (157 Tcl
  8.6/9.0 library files + fixtures), 120k randomised sources, and the edge-case
  table in `rust/tcl-compiler/src/parsing/syntax/mod.rs`'s unit tests.
- **Position-equivalence** — red fragment tokens equal the lexer's non-trivia
  token stream (offsets, lines, UTF-16 columns, `in_quote`) over the same
  corpus + fuzz, including multi-line and unicode bodies. The randomised
  randomised generator now also covers carriage returns
  (lone `\<CR>` continuations), astral / combining unicode, and a quoted word
  whose entire content is a backslash-newline — the seams a `\n`-only, ASCII
  generator never reached. The last is a real fragment, not trivia: the lexer
  emits a separator backslash-newline as `SEP`, but a quoted-word content
  backslash-newline as an `ESC` token the tree must keep (a fold there dropped
  the only fragment of the word).
- **Segment byte-identity** — `segments_from_*` matched the former
  `_segment_raw` field-for-field (`range`, `argv`, `texts`, `single_token_word`,
  `all_tokens`, `preceding_comment`, `expand_word`) over the corpus, 120k
  randomised differential cases, and nested-body (non-zero base) anchoring,
  before the loop was replaced.
- **Full `make test-rust`** green with the segmenter on the tree — the
  end-to-end proof that diagnostics, analysis, and AOT codegen are unchanged.

## Performance

Build + derive is ~1.55× the former bespoke loop on a library-file corpus
(build ~1.13×, derive ~0.42×). The dominant residual is the line index being
built twice — once in the lexer, once in the red layer; sharing the lexer's
index is the obvious follow-up. The overhead is amortised — and then erased — as
other consumers drop their own re-lexing onto the one tree. The lexing itself
(the real cost) is shared through the `green_tree` memo.

## Known gap — the minifier's compact-mode scanners

Every consumer segments or lexes through the tree and its shared `tokenise`
memo, with one exception: the minifier's three compact-mode descent scanners
(`_scan_array_tokens`, `_scan_argument_tokens`, `_collect_string_literals`) are
offset-precise source-edit machinery that still runs a private lexer loop.
Folding them onto the tree's descent needs a compact-names byte-identical bar
over the corpus before it can land.

Cursor-local prefix lexers (hover, symbol resolution) and the separate expr and
`f5/query` tokenisers are deliberately not on this tree — they are different
tokenisers or cursor-local by nature, not duplicate segmenters.

## Pointers

- Green layer: `rust/tcl-compiler/src/parsing/syntax/green.rs`
- Red layer: `rust/tcl-compiler/src/parsing/syntax/red.rs`
- Descent: `rust/tcl-compiler/src/parsing/syntax/descend.rs`
- Constructor: `rust/tcl-compiler/src/parsing/syntax/build.rs`
- Segment derivation: `rust/tcl-compiler/src/parsing/syntax/segment.rs`
- Segmenter (consumer): `rust/tcl-compiler/src/segmenter.rs`
- Tests: `rust/tcl-compiler/src/parsing/syntax/` unit tests, plus the
  differential oracle in `rust/tcl-compiler/tests/differential_segment.rs`

## Related docs

- [green-token-tree.md](green-token-tree.md) — the context-aware tokenisation
  memo (a different structure — its node is `TokenRegion`) and incremental
  reparse.
- [lexing-segmentation.md](lexing-segmentation.md) — lexer/segmenter contract.
