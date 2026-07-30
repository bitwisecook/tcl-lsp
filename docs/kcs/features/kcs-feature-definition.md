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
they are written, because the whole file loads before any body runs — but
only those written *outside* that body. A `rename` or `proc` that is itself
a statement of the body being read is an ordinary statement of the running
script, so it counts only from where it is written onward, exactly as at the
top level. An alias that binds leading arguments (`interp alias {} c {}
target extra`) is not the same call, so Go to Definition abstains rather
than pointing at a signature that does not describe it.

When a name carries both a `rename` and an `interp alias`, the one written
**later** is what the call reaches — an alias silently replaces whatever the
name held, and so does a rename.

A proc declared twice in one file is two definitions of one command. A call
between the two jumps to the **first** header, a call after both jumps to
the second, and a cursor on either header stays on that header.

A `rename` moves the command **object**, so it splits a redefined name into
two genuinely different commands: after `proc p …`, `rename p oldp`, `proc p
…`, the name `oldp` jumps to the *first* header — a later `proc p` cannot
change what `oldp` runs — while `p` jumps to the second.

A `TclOO` member name written as a bare word inside a class body is only a
jump target when it really is one: the cursor must sit on the member's own
declaration, or on a bare word that `link` (Tcl 9.0's `oo::Helpers::link`)
actually made callable. An un-linked bare sibling call raises `invalid
command name` in real Tcl, so Go to Definition abstains on it — in the
current file and across the workspace alike.

A **namespace-qualified variable** jumps across files. `$::tomato::version`
lands on the `variable version` token of the `namespace eval tomato { … }`
that declares it, wherever that block lives, and so does the relative
spelling `$tomato::version` written at the global level. A **bare** `$v`
does not: it names whatever the surrounding scope chain supplies, which no
other file can know, so it stays a within-file question. A `$name`-shaped
run of characters inside a comment or a data brace substitutes nothing in
real Tcl and is likewise never a jump target.

A command a `package require`d package provides also jumps: the package's
`pkgIndex.tcl` is resolved through the search path, including the one the
file builds for itself with `lappend auto_path [file dirname [file dirname
[info script]]]` and friends. Only statically resolvable path expressions
count — literals, `~`, `[info script]`, `[file dirname …]`, `[file join …]`.
A path built from a variable resolves to nothing rather than to a guess.

## Failure modes

- Definition not found after proc lookup or namespace resolution changes.
- A cross-file namespace variable stops resolving when the declaring file is
  no longer in the workspace index (it was closed *and* is outside every
  workspace folder).

## Test anchors

- `rust/tcl-lsp-core/src/definition.rs` (`mod tests`)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs` (cross-file namespace
  variables, cross-file class-reference arguments)
- `rust/tcl-lsp-server/src/lib.rs` unit tests
  (`definition_resolves_through_a_document_auto_path_package`)

## Screenshots

- `15-definition` — peek definition inline

![peek definition inline](../screenshots/15-definition.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
