# KCS: feature — Diagnostics

> **Audience:** User
> **Type:** Functionality

## Summary

Errors, warnings, security, taint tracking, and style checks shown as you type.

## Applies to

all-editors, MCP, Claude skill, diagnostic, warning

## How to use

- **Editor**: Diagnostics appear automatically as squiggly underlines. Hover for details.
- **MCP**: `analyze` (full analysis), `validate` (categorised report), `review` (security-focused).
- **Claude Code**: `/irule-validate`, `/tcl-validate`, `/irule-review`.
- **VS Code chat**: `@irule /validate`, `@tcl /validate`, `@irule /review`.
- **Settings**: Individual diagnostic codes can be toggled via `tclLsp.diagnostics.<CODE>`.

## Operational context

The analyser produces diagnostics in categories: errors (E-codes), security (S-codes), taint (T-codes), performance/style (W-codes), and optimiser suggestions (O-codes). Diagnostics are published on every document change via the LSP `textDocument/publishDiagnostics` notification (the default push model).

The server also supports pull-model diagnostics — `textDocument/diagnostic` for one document and `workspace/diagnostic` for the whole workspace — for editors that prefer to request diagnostics on demand. Pull responses return the same set the push model publishes, and an unchanged document is answered with a cheap `Unchanged` report. Pull mode is off by default and is enabled with `tclLsp.features.pullDiagnostics`; turning it on causes most clients to stop honouring the push notifications, so use one model or the other, not both.

## File-path anchors

- `analyser/_analyser/_diagnostics.py`
- `server/features/diagnostics.py`
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
- [LSP diagnostics publication](../../../docs/design/contracts/lsp-diagnostics-publication.md)
- [kcs-feature-cross-file-diagnostics.md](kcs-feature-cross-file-diagnostics.md) — how diagnostics account for procs and `package require`s in *other* files
