# The canonical concrete syntax tree (red-green CST)

The lossless, position-independent concrete syntax tree (CST) is the
representation command segmentation rides on. `segment_commands` and every
`SegmentedCommand` consumer downstream of it — IR lowering, AOT codegen to the
bytecode VM and WASM, the analyser, and the per-command `tcl` / `f5 irule`
tooling — read one tree rather than each re-lexing the bytes it cares about.
Two consumers still lex for themselves and are named under
[Known gap](#known-gap--the-minifier): the minifier and the formatter.

Living in `rust/tcl-compiler/src/parsing/syntax/`: the position-independent
green tree, the lazy red overlay, the constructor, lazy descent into braced
bodies and `[…]` substitutions, and the `SegmentedCommand` derivation.

## Why a *red-green* tree

The model is the Roslyn / rust-analyzer split:

- **Green** (`parsing/syntax/green.rs`) is **position-independent**: a node knows only
  its *width* and its children, never an absolute offset. Structurally
  identical regions compare equal by value, but children are held inline in
  `Vec<GreenElement>`: the current CST does not pointer-share subtrees and has
  no cross-edit reuse or incremental reparse cache.
- **Red** (`parsing/syntax/red.rs`) overlays a green tree with an *anchoring*
  (`base_offset` / `base_line` / `base_col`) and resolves absolute positions
  **lazily**, reproducing exactly the `Token` offsets/lines/columns the lexer
  emits.

This is deliberately the model the *green token tree* proposal (issue #477,
[`green-token-tree.md`](green-token-tree.md)) decided **not** to take: that
design is a context-aware tokenisation memo whose tokens would carry
**absolute** positions, because ~80 consumers read `Token.start.offset` as
absolute. That proposal is unbuilt — nothing named `TokenRegion` exists in the
workspace — so the CST under `parsing/syntax/` is the only tree there is. It
does not change the absolute-offset consumers: `SyntaxToken::to_token`
reproduces the lexer `Token` they already read.

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
    pub token_type: TokenType, // Esc | Str | Cmd | Var | Expand | …
    pub text: String,          // lexer inner text  ("abc" for {abc})
    pub raw: String,           // full source slice  ("{abc}")  → width
    pub end_rel: u32,          // Token.end.offset − Token.start.offset
    pub content_offset: u8,    // delimiter bytes stripped to get `text`
    pub in_quote: bool,
    pub leading: Vec<GreenTrivia>,
    pub trailing: Vec<GreenTrivia>,
}

pub struct GreenTrivia {
    pub kind: TriviaKind,      // Whitespace | Eol | Comment
    pub text: String,
}
```

Both structs also carry a private `full_width: u32`, cached at construction
and excluded from equality since it is derived from the other fields.

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

`segment::segments_from_tree` walks the red tree and produces the segmenter's
public `SegmentedCommand` shape; `segments_from_document` is the entry
`segment_commands_local` uses, taking the green `Document` and a `SourceMap`
directly. This *is* the segmenter — `segment_commands_local`
(`rust/tcl-compiler/src/segmenter.rs`) calls `build_document` then
`segments_from_document`, with no token loop of its own:

- **`argv`** — one merged token per word (type/text/start from the first
  fragment, end from the last: the compound-word merge of `{a}b`).
- **`texts`** — `word_piece` per fragment, reused from
  `rust/tcl-compiler/src/segmenter.rs`.
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

At its own level a `{…}` body or `[…]` command substitution is a single `Str` /
`Cmd` token whose `raw` owns the full span — delimiters included. *Descending*
re-lexes its inner text (`sm.token_text(token)`) as a child tree anchored
**`content_offset` bytes past the opener** — normally one, but zero for a
synthetic error-recovery token whose span already starts at the content. The
child therefore owns the delimiter-excluding interior with absolute positions
matching where it sits in the document — the "node owns its span, children
exclude delimiters" shape, realised for nested bodies. Descent is lazy (built
only when a caller asks).

- `descend_token(sm, token, config)` descends an absolutely-positioned
  `Str`/`Cmd` token; an unterminated region (closer absent — decided by
  `terminated`, from the inner *length* so empty `{}`/`[]` classify correctly)
  yields a **recovered** child (`Descended::is_terminated() == false`) that
  still carries its inner tokens, the lossless representation of malformed
  input. Any other token kind is anchored at its own position and is always
  recovered.
- `descend_command(registry, sm, cmd_name, args, arg_tokens, config)` descends
  each registry-resolved `ArgRole::Body` argument (via
  `CommandRegistry::arg_indices_for_role`), returning a `CommandBody` per
  descended body. `ArgRole::Expr` arguments stay with `expr_lexer`,
  `ArgRole::LambdaLiteral` arguments are deliberately excluded (a lambda body
  runs in its own namespace and needs an isolated scope, not just a sub-span),
  and data words are left opaque.

There is no shared tokenisation memo: `build_document` runs a fresh
`Lexer::with_source_map` each time. The descended stream is nonetheless
**identical by construction** to a direct re-lex of the same region, because
both tokenise the same inner text at the same anchor under the same
`LexerConfig`; the parity is asserted directly (same child fragments, same
terminated/recovered classification) over the corpus, 8 000 randomised nested
cases, and multi-level descent (a substitution inside a descended body).

## Verification

The bar is byte-identity with a frozen copy of the pre-CST token loop, kept as
a permanent regression net:

- **Losslessness** — `full_text == source` over the real-world corpus (157 Tcl
  8.6/9.0 library files + fixtures), 120k randomised sources, and the edge-case
  table in `rust/tcl-compiler/src/parsing/syntax/mod.rs`'s unit tests.
- **Position-equivalence** — red fragment tokens equal the lexer's non-trivia
  token stream (offsets, lines, UTF-16 columns, `in_quote`) over the same
  corpus and fuzz, including multi-line and unicode bodies. The randomised
  generator covers carriage returns (lone `\<CR>` continuations), astral and
  combining unicode, and a quoted word whose entire content is a
  backslash-newline — the seams a `\n`-only, ASCII generator never reaches.
  The last is a real fragment, not trivia: the lexer emits a separator
  backslash-newline as `Sep`, but a quoted-word content backslash-newline as
  an `Esc` token the tree must keep.
- **Segment byte-identity** — `rust/tcl-compiler/tests/differential_segment.rs`
  compares the production `segment_commands_with_offset_and_config` against
  `frozen_oracle::segment_local`, a byte-for-byte snapshot of the token loop
  taken before the segmenter derived from the CST. Comparing against the live
  `segment_commands_local` would be tautological, since that is now the CST
  derivation. Fields compared: `range`, `argv`, `texts`, `single_token_word`,
  `all_tokens`, `preceding_comment`, and `expand_word`, over the edge-case
  table, the `tmp/tcl{8.4,8.5,8.6,9.0}` corpus when present, and nested-body
  (non-zero base) anchoring.
- **Full `make test-rust`** green with the segmenter on the tree — the
  end-to-end proof that diagnostics, analysis, and AOT codegen agree.

## Performance

Build + derive is ~1.55× the bespoke loop it replaced on a library-file corpus
(build ~1.13×, derive ~0.42×). The dominant residual is the line index being
built twice — once in the lexer, once in the red layer; sharing the lexer's
index is the obvious follow-up. The overhead amortises as other consumers drop
their own re-lexing onto the one tree.

## Known gap — the minifier

The segmenter derives from the tree, and everything downstream of
`segment_commands` therefore rides it. Two source-rewriting consumers do not:

- The **minifier** (`rust/tcl-lsp-core/src/minify.rs`) — `minify_body` runs
  its own `Lexer::new(source).tokenise_all()` and hands the raw token slice to
  `parse_commands`, and `collect_string_literals` runs a second private loop
  with an explicit descent stack over `Str` / `Cmd` inner text.
- The **formatter** (`rust/tcl-lsp-core/src/formatting/engine.rs`) — lexes the
  document and each nested body with `Lexer::with_source_map(…)
  .tokenise_all()`.

Both are offset-precise source-edit machinery. Folding them onto the tree's
descent needs a byte-identical bar over the corpus for every tier before it
can land.

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

- [green-token-tree.md](green-token-tree.md) — the unbuilt proposal for a
  context-aware tokenisation memo and incremental reparse.
- [lexing-segmentation.md](lexing-segmentation.md) — lexer/segmenter contract.
