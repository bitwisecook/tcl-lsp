# KCS: feature — Formatting

## Summary

Configurable code formatting: indent size/style, brace style, line length, whitespace.

## Applies to

all-editors, MCP, transform

## How to use

- **Editor**: Format Document (Shift+Alt+F) or enable your editor's built-in format-on-save (e.g. VS Code's `editor.formatOnSave`).
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

- `tooling/formatter/engine.py`
- `server/features/formatting.py`

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
- [Formatter engine contracts](../../../docs/design/contracts/formatter-engine.md)
