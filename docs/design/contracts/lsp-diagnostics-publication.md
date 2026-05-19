# KCS: LSP diagnostics publication model

## Symptom

Editor diagnostics flicker, regress between edits, or differ from expected suppression/severity behaviour.

## Operational context

The LSP layer coordinates analysis output publication, including tiered scheduling and conversion to client-visible diagnostics.

## Decision rules / contracts

1. Publish fast baseline diagnostics first; enrich with deep results asynchronously.
2. Suppression and code-family policy must remain centralized and deterministic.
3. New LSP-facing diagnostic families must map cleanly to existing filtering controls.

### Push vs pull diagnostics

The default mode is **push** (`textDocument/publishDiagnostics`): the
server sends diagnostics to the client after each analysis pass. This is
the mode that the test suite and most client configurations rely on.

**Pull diagnostics** (`textDocument/diagnostic`, `workspace/diagnostic`)
are an opt-in alternative enabled by `tclLsp.features.pullDiagnostics`.
When enabled, the server advertises `diagnosticProvider` in
`ServerCapabilities`, which causes `vscode-languageclient` (and other LSP
clients) to switch to pull mode and stop processing push notifications.

Because handler registration and capability advertisement happen at server
startup, `pull_diagnostics_enabled` is in `_RESTART_REQUIRED_TOGGLES`.
Changing it via `didChangeConfiguration` logs a warning but takes effect
only after the server process is restarted.

## File-path anchors

- `server/features/diagnostics.py`
- `server/async_diagnostics.py`
- `server/server.py`

## Failure modes

- Stale deep diagnostics publishing after newer edits.
- Inconsistent suppression handling across analyser vs compiler-pass findings.
- Client-facing severity drift after adding new diagnostic families.

## Test anchors

- `tests/test_diagnostics.py`
- `tests/test_diagnostic_phases.py`
- `tests/test_async_diagnostics.py`

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [compiler diagnostics integration](../../../docs/design/compiler/diagnostics-integration.md)
- [async tiering contracts](../../../docs/design/compiler/async-diagnostics-tiering.md)
