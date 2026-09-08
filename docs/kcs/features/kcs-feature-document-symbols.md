# KCS: feature — Document Symbols

> **Audience:** User
> **Type:** Functionality

## Summary

Outline of procs, namespaces, event handlers, variables, and `tcltest` definitions (test cases, constraints, custom match modes) in the current file.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Ctrl+Shift+O or the Outline panel.
- **MCP**: `symbols` tool — pass source code.
- **Settings**: Toggle with `tclLsp.features.documentSymbols`.

## Example

![symbol picker showing proc outline](../screenshots/17-document-symbols.png)

## What appears in the outline

The tree nests procs inside namespaces, variables inside procs, and iRules
`when` handlers at the top level.

- **iRules handlers.** `when EVENT { … }` is an entry of its own kind, named
  for its event and ranged over the whole handler. `when EVENT priority 500
  { … }` lists identically. A dynamic `when $evt { … }` is skipped rather than
  shown as `$evt`.
- **`tcltest` definitions.** `tcltest::test NAME …` lists as a function-like
  entry with its description as detail, `tcltest::testConstraint NAME value`
  as a constant, and `tcltest::customMatch MODE command` as an operator. All
  are recognised whether called qualified or through `namespace import
  ::tcltest::*`, and one inside `namespace eval` nests under it.
- **`TclOO` members.** A class body contributes `method`, `classmethod` and
  `self method` (shown with a `classmethod` detail), `constructor`,
  `destructor`, and `property` as children of the class. Both the prefix form
  (`self method make {n} {…}`) and the block form (`self { method make {n}
  {…} }`) list, and likewise for `private`. A `[self class]` introspection
  call inside a method body contributes nothing.
- **BIG-IP config.** A `.conf` file gets a `module → kind → object` tree built
  from the config stanza tree instead of the Tcl scope walk. Nameless
  singletons (`auth password-policy`, `net self-allow`, …) fall back to their
  kind label, so no entry is ever empty.

A name is resolved through constant propagation, so a literal, a quoted
string, and a constant `$var` all resolve; a genuinely dynamic name is skipped
rather than shown as raw `$var` text.

A `TclOO` member that a later word in the same body **deletes or renames**
(`deletemethod`, `renamemethod`) is dropped rather than listed, because real
Tcl removes it. `renamemethod old new` is a move: `old` goes and `new` takes
its place, carrying the source's parameter list, body, and visibility, with
its name anchored at the `renamemethod` destination word so go-to-definition
lands there. Retraction is side-scoped — an unwrapped `deletemethod` acts on
the instance side, a `self`-scoped one on the class-object side — matching the
interpreter. A body real Tcl would reject outright still lists its partial
class, so navigation degrades rather than vanishing, and the offending word
draws a [`W315`](../codes/kcs-diagnostic-w315-class-definition-cannot-run.md).

Which commands define, retract, or rename a symbol is registry data, so
supporting the next such command is a spec change rather than a compiler edit.
For the member model itself, see the
[TclOO implementation contract](../../design/contracts/tcloo-implementation.md).

## Failure modes

- Symbols missing or mis-nested after parser changes.
- VS Code drops the entire outline when any symbol name is empty, so BIG-IP
  nameless singletons must fall back to a kind label.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
