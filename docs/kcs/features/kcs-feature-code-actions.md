# KCS: feature — Code Actions

## Summary

Quick fixes for diagnostics and refactoring actions: brace expressions, add
option terminators, modernise patterns, extract selection into proc, inline
proc, De Morgan's law, invert expression, IP conversion.

## Surface

lsp, mcp, vscode-command, all-editors

## How to use

- **Editor**: Ctrl+. on a diagnostic to see available fixes.
- **Extract to proc**: Select lines, press Ctrl+. and choose
  *Extract selection into proc*. The selected code is moved into a new `proc`
  with auto-detected variable parameters. The cursor lands on the proc name
  for immediate renaming (VS Code triggers the rename dialog automatically;
  other editors apply the edits and can use F2 on the new proc name).
- **Inline proc**: Place the cursor on a proc call and press Ctrl+. to inline
  a single-statement proc at its call site.
- **MCP**: `code_actions` tool — pass source and a line range.
- **VS Code commands**: `Tcl: Apply Safe Quick Fixes` (batch), `Tcl: Apply All Optimisations`.
- **Settings**: Toggle with `tclLsp.features.codeActions`.

## Operational context

Code actions are generated from diagnostics, optimiser suggestions, and
refactoring opportunities on selected code. Each action includes an edit that
can be applied to fix the issue or perform the refactoring. Safe fixes can be
batch-applied. The extract-to-proc refactoring attaches a post-edit command
(`tclLsp.renameSymbolAtPosition`) so the editor can position the cursor on the
new proc name — VS Code handles this automatically; other editors silently
ignore the command and the user can rename manually.

## File-path anchors

- `lsp/features/code_actions.py`

## Failure modes

- Code action produces invalid code.
- Safe-fix classification incorrect (destructive fix marked as safe).

## Test anchors

- `tests/test_code_actions.py`

## Screenshots

- `04-quickfix` — quick fix lightbulb menu

![quick fix lightbulb menu](../screenshots/04-quickfix.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../kcs-lsp-feature-providers.md)
