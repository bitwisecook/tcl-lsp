# KCS: feature — Formatting

## Summary

Configurable code formatting: indent size/style, brace style, line length, whitespace.

## Surface

lsp, mcp, vscode-command, all-editors

## How to use

- **Editor**: Format Document (Shift+Alt+F) or enable format-on-save.
- **MCP**: `format_source` tool — pass source and optional settings.
- **VS Code command**: `Tcl: Format Document`.
- **Settings**: Configure via `tclLsp.formatting.*`:
  - `indentSize` (default 4), `indentStyle` (spaces/tabs)
  - `braceStyle` (k_and_r)
  - `maxLineLength`, `goalLineLength`
  - `spaceAfterCommentHash`, `trimTrailingWhitespace`, `ensureFinalNewline`
  - `expandSingleLineBodies`, `blankLinesBetweenProcs`, and more.

## Operational context

The formatter rewrites source using the configurable style engine. It is idempotent: formatting already-formatted code produces no changes.

## File-path anchors

- `core/formatter/engine.py`
- `lsp/features/formatting.py`

## Failure modes

- Non-idempotent formatting (re-format changes output).
- Brace-style or indentation regressions.

## Test anchors

- `tests/test_formatter.py`

## Screenshots

- `07-formatting-after` — side-by-side before/after view (left pane unformatted, right pane formatted)

![formatting side-by-side before/after](../screenshots/07-formatting-after.png)

## Discoverability

- [KCS feature index](README.md)
- [Formatter engine contracts](../kcs-formatter-engine-contracts.md)
