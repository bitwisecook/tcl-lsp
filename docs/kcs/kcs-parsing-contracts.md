# KCS: Parsing pipeline contracts (segmentation + recovery)

## Symptom

Parser-facing diagnostics become inconsistent after syntax errors, or command boundaries shift unexpectedly after partial edits.

## Operational context

The parsing layer tokenises source, segments commands, and performs recovery for unclosed delimiters so downstream analysis can continue on malformed input.

## Decision rules / contracts

1. Command segmentation must preserve original token order and positional fidelity.
2. Recovery should prefer virtual-token insertion over source rewriting.
3. Partial/errored commands must still produce deterministic structures for downstream consumers.
4. Shared parsing helpers are authoritative for lifted behaviour:
   known-command cache (`known_command_names()`), argv span widening, single-arg `expr` shape extraction, and token content base/shift helpers.
5. Avoid reintroducing local caches or pass-specific reimplementations of known command and argv/word-shape reconstruction logic.

## File-path anchors

- `core/parsing/command_segmenter.py`
- `core/parsing/recovery.py`
- `core/parsing/known_commands.py`
- `core/parsing/argv.py`
- `core/parsing/command_shapes.py`
- `core/parsing/token_positions.py`
- `core/parsing/tokens.py`

## Failure modes

- Unclosed delimiters causing cascading false errors after first parse fault.
- Segment boundary drift leading to command-name misclassification.
- Recovery paths producing ranges that no longer match source positions.

## Test anchors

- `tests/test_command_segmenter.py`
- `tests/test_recovery.py`
- `tests/test_tricky_edge_cases.py`
- `tests/test_parsing_helpers.py`
- `tests/test_token_positions.py`

## Discoverability

- [KCS index](README.md)
- [lexing contracts](kcs-lexing-contracts.md)
- [shared utility contracts](kcs-core-lsp-shared-utility-contracts.md)
- [compiler pipeline overview](compiler/kcs-compiler-pipeline-overview.md)
