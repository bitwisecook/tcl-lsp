# KCS: IRULE4005 — racy static:: cross-event flow

## Symptom

A `static::` variable written outside `RULE_INIT` in one event and read
in another event produces no warning, masking a potential race condition
across concurrent connections.

## Operational context

`static::` variables persist across all connections on the same virtual
server.  Writing to them in per-request events (e.g. `HTTP_REQUEST`,
`HTTP_RESPONSE`) is inherently racy because multiple connections execute
concurrently on separate TMM threads.

IRULE4001 already warns at the write site ("write to `static::` outside
RULE_INIT").  IRULE4005 adds a cross-event dimension: when the variable
is also *read* in a different event, the race has observable consequences
beyond the write itself.

## Decision rules / contracts

1. `static::` variables participate in cross-event W211/W210 suppression
   regardless of which event defines them — the variable *is* used, so
   "unused variable" is incorrect.
2. When a `static::` def comes from a non-RULE_INIT event and a
   use-before-def exists in a different event, the variable is added to
   `ConnectionScope.racy_static_defs`.
3. The analyser emits IRULE4005 (WARNING) at the definition site in each
   non-RULE_INIT event that writes a racy `static::` variable.
4. `unset` of a `static::` variable is not treated as a definition for
   cross-event flow — it cannot seed a value in another event.
5. IRULE4005 is emitted from the IR/SSA-based connection scope analysis,
   not from source-text scanning.

## File-path anchors

- `compiler/connection_scope.py` — `racy_static_defs` computation
- `analyser/analyser.py` — `_emit_racy_static_diagnostics()`
- `server/server.py` — `_ALL_DIAGNOSTIC_CODES` registration
- `editors/vscode/package.json` — `tclLsp.diagnostics.IRULE4005` toggle

## Failure modes

- Missing IRULE4005 for write commands whose lowering does not produce
  `defs` entries.  Currently `set`, `append`, `lappend`, `incr`, and
  `array set` all produce correct defs (either via custom lowering hooks
  or via `ArgRole.VAR_NAME` in the command signature).
- False IRULE4005 if `unset` is incorrectly treated as a definition.
- Missing IRULE4005 if `ConnectionScope.racy_static_defs` fails to
  include a variable because `variable_scope_note()` returns non-None
  (scoping concern blocks cross-event flow).

## Test anchors

- `tests/test_checks.py::TestCrossEventScope`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [LSP diagnostics publication](../../../docs/design/contracts/lsp-diagnostics-publication.md)
- [compiler diagnostics integration](../../../docs/design/compiler/diagnostics-integration.md)
