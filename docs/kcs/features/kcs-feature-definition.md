# KCS: feature — Go to Definition

> **Audience:** User
> **Type:** Functionality

## Summary

Jump to proc or variable definitions within and across files.

## Applies to

all-editors, MCP, analyser

## How to use

- **Editor**: Ctrl+Click or F12 on a symbol.
- **MCP**: `goto_definition` tool — pass source, line, and character position.
- **Settings**: Toggle with `tclLsp.features.definition`.

## Operational context

Resolves proc calls, variable references, namespace-qualified names, and BIG-IP cross-object references to their definition locations. Uses shared proc-reference matching from `analyser/proc_lookup.py`.

## File-path anchors

- `server/features/definition.py`
- `analyser/proc_lookup.py`

Go to Definition follows the **command table as it stands where the cursor
is**, not merely how the word is spelled. A `rename OLD NEW` makes `NEW`
jump to `OLD`'s declaration; an `interp alias {} NAME {} TARGET` makes
`NAME` jump to `TARGET`, even when a same-named proc exists — the alias has
replaced it. Both are order-gated: a call written *before* the `rename` or
`interp alias` still resolves the ordinary way (or, if nothing else defines
it, not at all), which is what real Tcl does. A call inside a proc or class
body sees every one of the file's renames and aliases regardless of where
they are written, because the whole file loads before any body runs. An
alias that binds leading arguments (`interp alias {} c {} target extra`) is
not the same call, so Go to Definition abstains rather than pointing at a
signature that does not describe it.

A proc declared twice in one file is two definitions of one command. A call
between the two jumps to the **first** header, a call after both jumps to
the second, and a cursor on either header stays on that header.

A `TclOO` member name written as a bare word inside a class body is only a
jump target when it really is one: the cursor must sit on the member's own
declaration, or on a bare word that `link` (Tcl 9.0's `oo::Helpers::link`)
actually made callable. An un-linked bare sibling call raises `invalid
command name` in real Tcl, so Go to Definition abstains on it — in the
current file and across the workspace alike.

## Failure modes

- Definition not found after proc lookup or namespace resolution changes.

## Test anchors

- `tests/test_definition.py`

## Screenshots

- `15-definition` — peek definition inline

![peek definition inline](../screenshots/15-definition.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
