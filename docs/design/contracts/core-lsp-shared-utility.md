# KCS: Core/LSP shared utility contracts

## Symptom

Behaviour drifts between features/passes because equivalent logic (offset mapping, proc lookup, package ranking, event context, word-shape parsing) is reimplemented in multiple places.

## Operational context

Shared utility modules were lifted to provide one internal contract surface across `core/` and `lsp/`:

- source/position mapping
- parsing word-shape helpers
- compiler word/value-shape helpers
- proc reference matching
- package ranking
- iRules enclosing-event discovery

## Decision rules / contracts

### Position infrastructure

1. Use `DocumentBuffer` (`shared/document_buffer.py`) as the primary per-document position infrastructure. It provides cached `lines`, O(log n) `offset_to_position`, `position_to_offset`, `chunk_line_range`, and `range_from_offsets`. Do not construct standalone `SourceMap` instances in new code.
2. `DocumentState.buffer` is the canonical `DocumentBuffer` for an open document. It is lazily created and invalidated when `source` changes.
3. Use `DocumentBuffer.lines` instead of `source.split("\n")`. The result is cached per buffer.
4. Use `position_from_offset()` (`shared/ranges.py`) instead of `position_from_relative()` when a `line_starts` array is available. It is O(log n) vs O(text_len).
5. `SourceMap` (`shared/source_map.py`) is retained for backward compatibility in non-hot-path code (explorer, scripts, tests). Do not add new usages in `lsp/` or `core/` hot paths.

### Other contracts

6. Command-name knownness for parsing/recovery must come from `known_command_names()`, not module-local caches.
7. Multi-token argv span reconstruction must use `widen_argv_tokens_to_word_spans()`.
8. `extract_single_expr_argument()` must preserve source-faithful one-word shape (`$x` vs `${x}`, `[...]` retained).
9. Compiler passes must use shared helpers for value/word parsing (`value_shapes.py`, `var_refs.py`) rather than pass-local mini-parsers.
10. Proc reference matching precedence must come from `find_proc_by_reference()` / `iter_procs_by_reference()`.
11. Package suggestion ranking semantics are shared and fixed: `exact=0`, `startswith=1`, `contains=2`; caller controls limit.
12. iRules event context helper contract is `(event_name | None, anchor_line)` and should prefer the innermost enclosing `when`.

## File-path anchors

- `shared/document_buffer.py` — `DocumentBuffer`, `EditDescriptor`, `compute_line_starts`, `update_line_starts`
- `shared/source_map.py` — legacy `SourceMap`, `offset_to_line_col` (non-hot-path only)
- `shared/ranges.py` — `position_from_offset`, `position_from_relative`
- `shared/position.py` — `offset_at_position`, `find_command_at_position`
- `compiler/parsing/known_commands.py`
- `compiler/parsing/argv.py`
- `compiler/parsing/command_shapes.py`
- `compiler/parsing/token_positions.py`
- `core/compiler/value_shapes.py`
- `core/compiler/var_refs.py`
- `core/analysis/proc_lookup.py`
- `lsp/features/package_suggestions.py`
- `lsp/features/irules_context.py`

Primary consumers:

- `lsp/workspace/document_state.py` — `DocumentState.buffer` property
- `lsp/features/semantic_tokens.py` — `position_from_offset` with shared `_line_starts`
- `lsp/server.py`
- `core/bigip/parser.py`
- `core/bigip/rule_extract.py`
- `core/bigip/validator.py`
- `shared/position.py`
- `core/analysis/analyser.py`
- `core/compiler/compiler_checks.py`
- `core/compiler/lowering.py`
- `core/compiler/optimiser/`
- `core/compiler/core_analyses.py`
- `core/compiler/taint/`
- `core/compiler/shimmer.py`
- `core/compiler/ssa.py`
- `core/compiler/interprocedural.py`
- `core/compiler/gvn.py`
- `core/refactoring/_spans.py`
- `lsp/features/definition.py`
- `lsp/features/references.py`
- `lsp/features/rename.py`
- `lsp/features/call_hierarchy.py`
- `lsp/features/signature_help.py`
- `lsp/features/code_actions.py`
- `lsp/features/completion.py`
- `lsp/features/semantic_tokens.py`
- `lsp/features/inlay_hints.py`

## Failure modes

- Same cursor maps to different offsets between providers.
- Bare `$x` rewritten as `${x}` in `expr`-shape extraction and downstream heuristics diverge.
- Proc navigation features disagree on ambiguous short names.
- Code actions and server command rank package suggestions differently.
- iRules collect bootstrap insertion fails due to event-context tuple/order drift.
- Stale `DocumentBuffer` served after source change (buffer not invalidated).
- O(n) position computation in hot path because `SourceMap` used instead of `DocumentBuffer`.

## Test anchors

- `tests/test_document_buffer.py`
- `tests/test_source_map.py`
- `tests/test_semantic_tokens_delta.py`
- `tests/test_parsing_helpers.py`
- `tests/test_token_positions.py`
- `tests/test_compiler_helpers.py`
- `tests/test_proc_lookup.py`
- `tests/test_proc_lookup_lsp_features.py`
- `tests/test_package_suggestions.py`
- `tests/test_irules_context.py`
- `tests/test_bigip_rule_extract.py`
- `tests/test_bigip_validator.py`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [parsing contracts](../../../docs/design/contracts/parsing.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [command registry event model](../../../docs/design/contracts/command-registry-event-model.md)
- [compiler downstream pass contracts](../../../docs/design/compiler/downstream-pass-contracts.md)
