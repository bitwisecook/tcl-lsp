# Lexing contracts — token and range fidelity

## Symptom

Diagnostics highlight the wrong location, or parser behaviour changes after an
edit to escape handling, substitutions, or nested constructs.

## Operational context

`rust/tcl-lexer` is the positional source of truth for segmentation, recovery,
semantic analysis, and every diagnostic range in the stack. Nothing above it
re-derives a word's geometry.

A `Token` carries only a `Span`; its text, line, and column are resolved on
demand through a `SourceMap`. That is why the range accessors take a
`&SourceMap` — they recover a word's source geometry rather than caching a
denormalised copy of it.

## Decision rules / contracts

1. Every emitted token carries an accurate start and end offset.
2. Nested constructs (braces, brackets, quotes) preserve stack-safe token
   emission.
3. A change to lexer behaviour requires downstream range regression checks —
   ranges are consumed by diagnostics, code actions, semantic tokens, and the
   refactoring engine.
4. **A delimited word's closer is derived only through the authoritative
   accessors** `tcl_lexer::word_closer_offset` / `word_end_position`, never by
   computing `token.end.offset + 1`. A *non-empty* `{abc}` / `[…]` / `"…"`
   token's inclusive end sits on its **last inner character**, but an *empty*
   `{}` / `[]` / `""` token's end already sits on the **closer** — so
   `end + 1` overshoots, and a trailing empty `{}` swallows the enclosing
   body's `}`. The accessors detect emptiness from the lexer's content
   geometry and stay correct for backslash-escaped quoted words too.
5. Command and word *ranges* owned by the segmenter use the inner-end
   convention and widen only where they need the closer
   (`SourceMap::range_positions`, the segmenter's `command_span`). Callers
   that need to **slice source** — a refactor edit extracting a word's raw
   text — use the source-aware accessors instead.
6. Backslash decoding has exactly one implementation:
   `tcl_lexer::backslash_subst`, re-exported as `tcl_syntax::backslash::decode`
   ([shared-utility-contracts-rust.md](shared-utility-contracts-rust.md)).

## File-path anchors

- `rust/tcl-lexer/src/lexer.rs` — the tokeniser.
- `rust/tcl-lexer/src/tokens.rs`, `span.rs` — `Token`, `TokenType`, `Span`.
- `rust/tcl-lexer/src/source_map.rs`, `line_index.rs` — offset ↔ position.
- `rust/tcl-lexer/src/ranges.rs` — the authoritative closer accessors.
- `rust/tcl-lexer/src/substitution.rs`, `expr_lexer.rs`,
  `structural_index.rs` — substitution scanning, the `expr` sub-grammar, and
  the structural index.

## Failure modes

- Escape or newline handling collapsing token boundaries.
- Nested substitutions emitting tokens with incorrect offsets.
- Token-kind drift breaking segmenter assumptions.
- A consumer re-deriving a closer arithmetically and overshooting the empty
  case.

## Discoverability

- [Design doc index](../README.md)
- [parsing contracts](parsing.md)
- [the canonical concrete syntax tree](../compiler/syntax-tree.md)
- [range-drift troubleshooting](../../kcs/kcs-issue-range-drift.md)
