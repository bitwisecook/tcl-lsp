# KCS: Lexing contracts (token and range fidelity)

## Symptom

Diagnostics highlight wrong locations or parser behaviour changes after updates to escape handling, substitutions, or nested constructs.

## Operational context

Lexer output is the positional source of truth for segmentation, recovery, semantic analysis, and diagnostics ranges.

## Decision rules / contracts

1. Every emitted token must carry accurate start/end positions.
2. Nested constructs (braces, brackets, quotes) must preserve stack-safe token emission.
3. Lexer behaviour changes require downstream range regression checks.
4. Empty-delimiter convention (issue #527): a non-empty `{abc}` / `[..]` / `".."` token's inclusive `end.offset` sits on its **last inner character**, but an *empty* `{}` / `[]` / `""` token's `end.offset` already sits on the **closer**. Consumers must not re-derive the closer as `end.offset + 1` (it overshoots the empty case); use the authoritative `shared/ranges.py` accessors `word_closer_offset` / `word_end_position`, which detect emptiness from `tok.text` and stay correct for backslash-escaped quoted words.

## File-path anchors

- `compiler/parsing/lexer.py`
- `shared/tokens.py`
- `shared/ranges.py` (authoritative word-closer accessors)

## Failure modes

- Escaped/newline handling collapsing token boundaries.
- Nested substitutions emitting tokens with incorrect offsets.
- Token-kind drift breaking segmenter assumptions.

## Test anchors

- `tests/test_lexer.py`
- `tests/test_tcl_parse.py`
- `tests/test_tcl_parse_expr.py`
- `tests/test_word_span_tracking.py`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [parsing contracts](../../../docs/design/contracts/parsing.md)
- [range-drift troubleshooting](../../../docs/kcs/kcs-issue-range-drift.md)
