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

1. Use `core/common/source_map.py` for offset↔position conversions; do not add local converters.
2. Command-name knownness for parsing/recovery must come from `known_command_names()`, not module-local caches.
3. Multi-token argv span reconstruction must use `widen_argv_tokens_to_word_spans()`.
4. `extract_single_expr_argument()` must preserve source-faithful one-word shape (`$x` vs `${x}`, `[...]` retained).
5. Compiler passes must use shared helpers for value/word parsing (`value_shapes.py`, `var_refs.py`) rather than pass-local mini-parsers.
6. Proc reference matching precedence must come from `find_proc_by_reference()` / `iter_procs_by_reference()`.
7. Package suggestion ranking semantics are shared and fixed: `exact=0`, `startswith=1`, `contains=2`; caller controls limit.
8. iRules event context helper contract is `(event_name | None, anchor_line)` and should prefer the innermost enclosing `when`.

## File-path anchors

- `core/common/source_map.py`
- `core/parsing/known_commands.py`
- `core/parsing/argv.py`
- `core/parsing/command_shapes.py`
- `core/parsing/token_positions.py`
- `core/compiler/value_shapes.py`
- `core/compiler/var_refs.py`
- `core/analysis/proc_lookup.py`
- `lsp/features/package_suggestions.py`
- `lsp/features/irules_context.py`

Primary consumers:

- `core/common/position.py`
- `core/bigip/parser.py`
- `core/bigip/rule_extract.py`
- `core/bigip/validator.py`
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
- `lsp/features/definition.py`
- `lsp/features/references.py`
- `lsp/features/rename.py`
- `lsp/features/call_hierarchy.py`
- `lsp/features/signature_help.py`
- `lsp/features/code_actions.py`
- `lsp/features/completion.py`
- `lsp/features/semantic_tokens.py`
- `lsp/features/inlay_hints.py`
- `lsp/server.py`

## Failure modes

- Same cursor maps to different offsets between providers.
- Bare `$x` rewritten as `${x}` in `expr`-shape extraction and downstream heuristics diverge.
- Proc navigation features disagree on ambiguous short names.
- Code actions and server command rank package suggestions differently.
- iRules collect bootstrap insertion fails due to event-context tuple/order drift.

## Test anchors

- `tests/test_source_map.py`
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

- [KCS index](README.md)
- [parsing contracts](kcs-parsing-contracts.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
- [command registry event model](kcs-command-registry-event-model.md)
- [compiler downstream pass contracts](compiler/kcs-downstream-pass-contracts.md)
