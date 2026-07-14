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
6. The canonical lossless **red-green concrete syntax tree** (`compiler/parsing/syntax/`) is authoritative for segmentation: `segment_commands()` builds the tree and derives `SegmentedCommand`s from it byte-identically. New consumers segment / descend through the tree (and its shared `green_tree.tokenise` memo) rather than spinning up a private `TclLexer` loop. See [syntax-tree.md](../compiler/syntax-tree.md).
7. A word's closing `}`/`]`/`"` is derived only via the authoritative `shared/ranges.py` accessors (`word_closer_offset` / `word_end_position`), never by re-deriving `tok.end.offset + 1` — that overshoots an empty `{}`/`[]`/`""` whose inclusive end already sits on the closer (issue #527).

## File-path anchors

- `compiler/parsing/command_segmenter.py`
- `compiler/parsing/syntax/` (red-green CST: `build.py`, `green.py`, `red.py`, `descend.py`, `segment.py`)
- `compiler/parsing/recovery.py`
- `compiler/parsing/known_commands.py`
- `compiler/parsing/argv.py`
- `compiler/parsing/command_shapes.py`
- `compiler/parsing/token_positions.py`
- `shared/ranges.py`
- `shared/tokens.py`

## Failure modes

- Unclosed delimiters causing cascading false errors after first parse fault.
- Segment boundary drift leading to command-name misclassification.
- Recovery paths producing ranges that no longer match source positions.

## Test anchors

- `tests/test_command_segmenter.py`
- `tests/test_syntax_tree.py`
- `tests/test_syntax_descend.py`
- `tests/test_word_span_tracking.py`
- `tests/test_recovery.py`
- `tests/test_tricky_edge_cases.py`
- `tests/test_parsing_helpers.py`
- `tests/test_token_positions.py`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [the canonical concrete syntax tree (CST)](../../../docs/design/compiler/syntax-tree.md)
- [lexing contracts](../../../docs/design/contracts/lexing.md)
- [shared utility contracts](../../../docs/design/contracts/shared-utility-contracts-rust.md)
- [compiler pipeline overview](../../../docs/design/compiler/compiler-pipeline-overview.md)
