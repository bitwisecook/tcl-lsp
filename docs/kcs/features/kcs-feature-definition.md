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

Three details of that search path follow Tcl exactly rather than
approximately:

- **`set` assigns a list, `lappend` appends words.** `set auto_path {/o/p1
  /o/p2}` names *two* directories, and `{/o/with space}` inside it names
  *one*; `lappend auto_path a b` names two, while `lappend auto_path {p q}`
  names the single directory `p q`.
- **Paths are Tcl slash form on every host.** `[file dirname [info script]]`
  resolves against the file's own directory on Windows too — `C:\repo\pkg`
  and `\\server\share\pkg` fold like `/repo/pkg` does, and `..` never climbs
  out of a drive or a UNC share. A *drive-relative* `C:lib` names a
  per-drive working directory nothing static can know, so it abstains.
- **A version constraint picks the release Tcl would load.** With 1.5 and 2.3
  both on the search path, `package require widget 2.0` navigates into 2.3 —
  `package vsatisfies` semantics (`2.0` means up to but excluding the next
  major), highest satisfying release wins. An unconstrained require still
  answers the first provider found.

### Caller-frame variables

A variable can be created by the procedure you call rather than by the code
you are reading — that is what `upvar` is for:

```tcl
proc setdef {varName} { upvar 1 $varName target; set target "default" }
proc build {} {
    setdef options       ;# this word creates `options` in build's frame
    return $options
}
```

There is no `set options` to jump to, so Go to Definition on `$options` goes
to the call-site word `options` on the `setdef` line — the creating write, and
the word you would rename. It works from either end: the same jump target is
reported whether the cursor is on the `$options` read or on the bare word
itself.

The creating call may sit inside an `if`, `while`, `foreach`, `catch` or
`switch` body — those run in the frame they are written in, so the jump still
works. A call inside a nested `proc`, an `apply` lambda, a `namespace eval` or
an `uplevel` body creates that frame's variable instead, and is not a
definition here.

When nothing binds the name, Go to Definition reports no location rather than
guessing. A `$`-led read never resolves to a command, proc, or method of the
same name — Tcl keeps those in a separate table from variables.

## Failure modes

- Definition not found after proc lookup or namespace resolution changes.
- A cross-file namespace variable stops resolving when the declaring file is
  no longer in the workspace index (it was closed *and* is outside every
  workspace folder).
- A callee that binds a *literal* caller-side name (`upvar 1 options options`)
  names it nowhere at the call site, so there is no word to jump to.
- A callee whose `upvar` level is not `1` (`upvar 0`, `upvar #0`, `upvar 2`)
  aliases some other frame, so its call site defines nothing where you are
  reading and no location is reported.

## Test anchors

- `tests/test_definition.py`
- `rust/tcl-lsp-server/tests/e2e/navigation.rs` — the caller-frame cases
- `rust/tcl-lsp-core/src/caller_frame.rs` — unit tests for the binding scan
- `rust/tcl-lsp-core/src/definition.rs` (`mod tests`)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs` (cross-file namespace
  variables, cross-file class-reference arguments)
- `rust/tcl-lsp-server/src/lib.rs` unit tests
  (`definition_resolves_through_a_document_auto_path_package`,
  `set_auto_path_puts_every_list_element_on_the_search_path`,
  `a_versioned_require_indexes_the_release_it_asks_for`)
- `rust/tcl-compiler/src/auto_path_eval.rs` (`mod tests`) — the list-arity and
  slash-form path rules
- `rust/tcl-lsp-core/src/package_resolver/tests.rs`
  (`resolve_picks_the_highest_release_satisfying_the_constraint`)

## Screenshots

- `15-definition` — peek definition inline

![peek definition inline](../screenshots/15-definition.png)

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
