# KCS: Formatter engine contracts

## Symptom

Formatting output changes unexpectedly between runs, or style rewrites conflict with parser/feature expectations.

## Operational context

Formatting is implemented as an engine/config/runtime pipeline and surfaced through LSP formatting handlers. Correctness depends on stable token/body detection and deterministic rewrite behaviour.

## Decision rules / contracts

1. Formatter output should be idempotent for stable inputs and config.
2. Formatting decisions must preserve parseability and command semantics.
3. New formatting options require config wiring + regression coverage.
4. Formatter consumers should import `core/formatting/*` directly; do not add alternate import paths.

## File-path anchors

- `core/formatting/config.py`
- `core/formatting/engine.py`
- `core/formatting/formatter.py`
- `lsp/features/formatting.py`

## Failure modes

- Non-idempotent rewrites that keep changing on repeated format operations.
- Body/expr boundary misclassification causing semantic changes.
- Option-specific regressions due to missing config propagation.

## Test anchors

- `tests/test_formatter.py`
- `tests/test_tcl_parse.py`
- `tests/test_tcl_parse_expr.py`
- `tests/test_core_lift_consumers.py`

## Discoverability

- [KCS index](README.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
- [parsing contracts](kcs-parsing-contracts.md)
