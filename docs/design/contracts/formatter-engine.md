# KCS: Formatter engine contracts

## Symptom

Formatting output changes unexpectedly between runs, or style rewrites conflict with parser/feature expectations.

## Operational context

Formatting is implemented as an engine/config/runtime pipeline and surfaced through LSP formatting handlers. Correctness depends on stable token/body detection and deterministic rewrite behaviour.

## Decision rules / contracts

1. `format_tcl` is **idempotent for every input** — `format_tcl(format_tcl(x))
   == format_tcl(x)`. Valid Tcl reaches its formatted form in one pass (the
   confirming pass returns it unchanged). Structurally malformed input (an
   unbalanced `{` / `[` / `"`) can reconstruct non-idempotently — the lexer
   swallows the unterminated region to EOF and the reconstruction fabricates a
   closer a re-parse re-mangles, formerly growing the output by a delimiter on
   every pass — so `format_tcl` iterates to a fixed point and, if none exists
   within the pass cap, returns the input unchanged rather than mangle or
   unboundedly grow un-formattable code. The same `_minify_body_stable` wrapper
   guards the minifier. (Cost: valid input formats twice — once plus the
   confirming pass.)
2. Formatting decisions must preserve parseability and command semantics.
3. New formatting options require config wiring + regression coverage.
4. Formatter consumers should import `tooling/formatter/*` directly; do not add alternate import paths.

## File-path anchors

- `tooling/formatter/config.py`
- `tooling/formatter/engine.py`
- `tooling/formatter/formatter.py`
- `server/features/formatting.py`

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

- [KCS index](../../../docs/design/README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
- [parsing contracts](../../../docs/design/contracts/parsing.md)
