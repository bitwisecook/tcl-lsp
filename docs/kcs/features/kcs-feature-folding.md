# KCS: feature — Folding

> **Audience:** User
> **Type:** Functionality

## Summary

Code folding for procs, namespaces, event handlers, and braced blocks.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Click fold markers in the gutter or use Ctrl+Shift+[ to fold.
- **Settings**: Toggle with `tclLsp.features.folding`.

## Operational context

Folding ranges are computed from the parsed AST, identifying proc bodies, namespace blocks, `when` event handlers, and multi-line braced expressions.

In VS Code the folding ranges also drive **sticky scroll** for Tcl code
languages. VS Code picks the sticky model from a fallback chain — outline
model → folding-range provider → indentation heuristic — and a non-empty
outline stops the chain. A Tcl script whose top level is `if` / `for` /
`foreach` has only single-line symbols in its outline, so the outline model
pins nothing and sticky scroll goes blank (issue #1122). The extension
therefore contributes
`"editor.stickyScroll.defaultModel": "foldingProviderModel"` as a
language-scoped default for every Tcl code language, which pins the same
block headers folding already knows about. BIG-IP config is the opposite
shape — a rich `module → kind → object` stanza outline and almost no folds —
so `tcl-bigip` stays on the outline model. Both are defaults: a user setting
for `editor.stickyScroll.defaultModel` still wins.

The version-pinned dialect languages (`tcl8.4`, `tcl8.5`, `tcl9.0`, `tcl9.1`)
are the one gap. A language id containing a `.` cannot be used as a
configuration override identifier — VS Code splits `[tcl8.4]` on the dot while
building the default-configuration value tree, throws, and drops every
remaining override in the block — so those four keep the outline model.

## File-path anchors

- `server/features/folding.py`

## Failure modes

- Folding ranges missing or incorrect after parser changes.

## Test anchors

- `tests/test_folding.py`

## Screenshots

- `20-folding` — code folded to show structure

![code folded to show structure](../screenshots/20-folding.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
