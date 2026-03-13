# KCS: feature — Diagnostics

## Summary

Errors, warnings, security, taint tracking, and style checks shown as you type.

## Surface

lsp, mcp, claude-code, all-editors

## How to use

- **Editor**: Diagnostics appear automatically as squiggly underlines. Hover for details.
- **MCP**: `analyze` (full analysis), `validate` (categorised report), `review` (security-focused).
- **Claude Code**: `/irule-validate`, `/tcl-validate`, `/irule-review`.
- **VS Code chat**: `@irule /validate`, `@tcl /validate`, `@irule /review`.
- **Settings**: Individual diagnostic codes can be toggled via `tclLsp.diagnostics.<CODE>`.

## Operational context

The analyser produces diagnostics in categories: errors (E-codes), security (S-codes), taint (T-codes), performance/style (W-codes), and optimiser suggestions (O-codes). Diagnostics are published on every document change via the LSP `textDocument/publishDiagnostics` notification.

## File-path anchors

- `core/analysis/analyser.py`
- `lsp/features/diagnostics.py`
- `ai/shared/diagnostics.py`
- `ai/shared/diagnostics.json`

## Failure modes

- Diagnostics missing after a parse or analyser change.
- Duplicate diagnostics from overlapping passes.

## Test anchors

- `tests/test_diagnostics.py`

## Screenshots

- `01-diagnostics-overview` — squiggly underlines and Problems panel
- `05-security-taint` — security and taint tracking diagnostics
- `08-style-warnings` — style warning diagnostics

![squiggly underlines and Problems panel](../screenshots/01-diagnostics-overview.png)
![security and taint tracking diagnostics](../screenshots/05-security-taint.png)
![style warning diagnostics](../screenshots/08-style-warnings.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP diagnostics publication](../kcs-lsp-diagnostics-publication.md)
