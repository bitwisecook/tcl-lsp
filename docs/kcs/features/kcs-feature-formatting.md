# KCS: feature — Formatting

> **Audience:** User
> **Type:** Functionality

## Summary

Configurable code formatting: indent size/style, brace style, line length, whitespace.

## Applies to

all-editors, MCP, transform

## How to use

- **Editor**: Format Document (Shift+Alt+F), enable your editor's built-in format-on-save (e.g. VS Code's `editor.formatOnSave`), or turn on the server's own format-on-save with `tclLsp.features.willSaveWaitUntil`.
- **MCP**: `format_source` tool — pass source and optional settings.
- **VS Code command**: `Tcl: Format Document`.
- **Settings**: Configure via `tclLsp.formatting.*`:
  - `indentSize` (default 4), `indentStyle` (spaces/tabs)
  - `braceStyle` (k_and_r)
  - `maxLineLength`, `goalLineLength`
  - `spaceAfterCommentHash`, `trimTrailingWhitespace`, `ensureFinalNewline`
  - `lineEnding` (default `auto`)
  - `expandSingleLineBodies`, `blankLinesBetweenProcs`, and more.

### Line endings

`tclLsp.formatting.lineEnding` defaults to `auto`: formatted output — and every
newline the server inserts through a code action ("Generate docstring", "Extract
into variable", "Add `package require …`", a `# noqa` suppression) — reuses
whichever line ending the file already has. A file with no line break at all
gets a line feed. Set `lf`, `crlf`, or `cr` to force one instead, which rewrites
every line ending in the file the next time it is formatted.

Line endings are also what the W118 warning reports; see
[Why W112 and W118 have no quick fix](../kcs-qa-why-w112-w118-have-no-quick-fix.md).

### Format on save

The server advertises `textDocument/willSaveWaitUntil`, so it can format a
document as part of the save itself rather than relying on the editor's own
format-on-save. It is **off by default**; enable
`tclLsp.features.willSaveWaitUntil` to turn it on. When enabled, a save runs the
same formatter as **Format Document** and applies the resulting edits, using the
`tclLsp.formatting.lineLength` resolved for the document's workspace folder. When
disabled, save makes no formatting edits, and the editor's `editor.formatOnSave`
(if set) still applies through the ordinary `textDocument/formatting` request.

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
