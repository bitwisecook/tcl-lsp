# KCS: feature — Code Actions

> **Audience:** User
> **Type:** Functionality

## Summary

Quick fixes for diagnostics and refactoring actions: brace expressions, add
option terminators, modernise patterns, extract selection into proc, inline
proc, De Morgan's law, invert expression, IP conversion.

## Applies to

all-editors, MCP

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

Inline proc declines a proc whose body is branchy, and any call whose head
is **frame-sensitive** — a command that terminates a block, transfers
control, creates a scope alias, or creates a barrier (`return`, `break`,
`continue`, `tailcall`, `yield`, `uplevel`, `upvar`, `global`, `variable`,
`source`, `exit`, …). Lifting one of those out of its proc frame changes
what it returns from, breaks out of, or binds against. The set is registry
trait data, not a hand-maintained list, so a newly declared command is
gated automatically.

## File-path anchors

- `server/features/code_actions.py`

## Failure modes

- Code action produces invalid code.
- Inline proc is offered on, or withheld from, the wrong command because a
  registry trait is mis-declared. A flag collision once made every
  safe-interpreter-hidden command read as frame-sensitive, silently
  withholding the action from `file`, `exec`, `open`, and seven others —
  see
  [the note on that false positive](../kcs-issue-w129-false-positive-on-control-transfer-commands.md).
- Safe-fix classification incorrect (destructive fix marked as safe).

## Test anchors

- `tests/test_code_actions.py`

## Screenshots

- `04-quickfix` — quick fix lightbulb menu

![quick fix lightbulb menu](../screenshots/04-quickfix.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
