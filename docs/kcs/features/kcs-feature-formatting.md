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
  - `expandAbbreviations` (default `true`), `booleanForm` (default `true/false`)
  - `expandSingleLineBodies`, `blankLinesBetweenProcs`, and more.

### Keyword abbreviations and boolean form

Tcl accepts any unique prefix of an ensemble subcommand or an `-option`, so
`string le` is `string length` and `lsearch -noc` is `lsearch -nocase`. It
also accepts `true`, `yes`, `on`, `1` (and their prefixes) wherever a value is
consumed as a boolean. Both make a file read as several conventions at once,
so the formatter normalises them.

`tclLsp.formatting.expandAbbreviations` (default `true`) expands unique-prefix
abbreviations to their canonical spellings:

```tcl
string le $s          ;# → string length $s
lsearch -noc -al $x $p ;# → lsearch -nocase -all $x $p
```

An **ambiguous** prefix is never rewritten — the formatter does not guess.
`string l $s` keeps its bytes and gets the
[W145 ambiguity warning](../codes/kcs-diagnostic-w145-ambiguous-abbreviation.md)
with one quick fix per candidate. Strict tables, ensembles configured with
`namespace ensemble … -prefixes 0`, dynamic words, and command names (Tcl
never prefix-matches those) are all left alone.

`tclLsp.formatting.booleanForm` (default `true/false`; also `yes/no`,
`on/off`, `0/1`, `preserve`) normalises every word the registry proves is
consumed as a boolean. A **value-definition** site is never rewritten: `set
flag yes` keeps its bytes, because `$flag` may later be compared with `eq
"yes"`, matched by a `switch` arm, or written to a log — `true` and `yes` are
different strings even though both are truthy.

Both rewrites are idempotent, and both apply to whole-document formatting,
range formatting, and format-on-save alike. A range format only rewrites
words wholly inside the range.

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

## Failure modes

- Non-idempotent formatting (re-format changes output).
- Brace-style or indentation regressions.

## Screenshots

- `07-formatting-after` — side-by-side before/after view (left pane unformatted, right pane formatted)

![formatting side-by-side before/after](../screenshots/07-formatting-after.png)

## Discoverability

- [KCS feature index](README.md)
- [Formatter engine contracts](../../../docs/design/contracts/formatter-engine.md)
