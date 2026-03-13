# KCS: Lexing contracts (token and range fidelity)

## Symptom

Diagnostics highlight wrong locations or parser behaviour changes after updates to escape handling, substitutions, or nested constructs.

## Operational context

Lexer output is the positional source of truth for segmentation, recovery, semantic analysis, and diagnostics ranges.

## Decision rules / contracts

1. Every emitted token must carry accurate start/end positions.
2. Nested constructs (braces, brackets, quotes) must preserve stack-safe token emission.
3. Lexer behaviour changes require downstream range regression checks.

## File-path anchors

- `core/parsing/lexer.py`
- `core/parsing/tokens.py`
- `core/parsing/substitution.py`

## Failure modes

- Escaped/newline handling collapsing token boundaries.
- Nested substitutions emitting tokens with incorrect offsets.
- Token-kind drift breaking segmenter assumptions.

## Test anchors

- `tests/test_lexer.py`
- `tests/test_tcl_parse.py`
- `tests/test_tcl_parse_expr.py`

## Discoverability

- [KCS index](README.md)
- [parsing contracts](kcs-parsing-contracts.md)
- [range-drift troubleshooting](compiler/kcs-troubleshooting-range-drift.md)
