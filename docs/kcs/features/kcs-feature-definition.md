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

A `namespace import ::src::*` is order-gated the same way, on the *export*
side. The import binds the names `::src` exported **at the moment the import
runs**, and the export list keeps changing afterwards without reaching back: a
`namespace export -clear` written after the import does **not** revoke the
alias (the bare call still jumps to the source proc), and a `namespace export`
written after the import does **not** create one (the bare call resolves to
nothing through that import). A second, later `namespace import` takes its own
snapshot, so it can pick up a name the first one could not. Export patterns
are glob *patterns*, not command references — `namespace export get*` covers
`getX` and not `setX`, and `namespace export p` written before `proc p` still
exports it. The same gate applies to an *exact* `namespace import ::src::p`:
real Tcl silently binds nothing when `p` is not exported, so neither does
navigation — and that includes an import of a global command
(`namespace import ::p`), which needs a global `namespace export p` like
anything else.

Ordering follows the same load-order rule renames and aliases do. An import
written **inside a proc or method body** sees every top-level statement of its
own file, wherever written — the file loads before any body runs — so an
export further down the file still counts; an export written after the import
*in that same body* does not. Ordering only exists inside one document; when
the import and the export are in different files, nothing fixes which loads
first, so navigation keeps answering rather than guessing a revocation.

A call written **before** its own `namespace import` reaches nothing, and Go to
Definition says so: the import has not run yet, exactly as a `rename` written
below a call has not. The same load-order rule as everywhere else applies, so
this is about *statements*, not about lines — a call inside a proc or method
body still jumps through an import written further down the same file, because
the whole file loads before any body runs. That is the ordinary shape of a
library module: the procs first, the `namespace import`s at the bottom. An
import written after the call *inside that same body* is a later statement of
the running script, and the call before it resolves to nothing.

The tail-match fallback no longer papers over any of this. Go to Definition
keeps a lenient last resort — a proc whose defining namespace is not
statically visible at the call still jumps — but it will not answer with a
command whose only route to the call was an import the rules above have just
ruled out. Nothing else about the fallback changes: a same-file proc no import
mentions still jumps as before.

An import is not a permanent name, either. `namespace forget ::src::p` — or
the unqualified `namespace forget p`, which drops whatever this namespace
imported under that name — takes the alias away again, and a bare call written
after it resolves to nothing; a call written *before* it still jumps to the
source. Deleting the source command (`rename ::src::p {}`) has the same effect,
because the alias holds the command *object*: a plain `rename ::src::p
::src::pp` does **not** break it (the alias keeps working, only `namespace
origin` moves), and redefining the source is seen straight through the link.
Re-running the import after a forget brings the alias back.

Importing onto a name the target namespace already has is an error unless the
import carries `-force`, and the failed import installs nothing — so a bare
call still reaches the *local* definition, and Go to Definition stays on it.
"Already has" includes a name it imported earlier from somewhere else: a
second unforced import of the same name from a different namespace fails too,
and navigation keeps answering the first source — whichever way round the two
were spelled, `namespace import ::A::*` then `namespace import ::B::p` or the
other way about. "First" is the order they *run*, not the order they are
written: an import inside a proc body loses to a top-level import of the same
name anywhere in that file, even one written below it, because the file loads
before any body runs. With `-force` the import
replaces whatever was there, and from that point on the same bare call jumps
to the **source** instead — until a `proc` of that name is written, which
silently takes the name back and sends navigation to the new local
definition.

Imports chain. When `::A` imports `::B::*` and `::B` had imported `::C::*`
(and re-exported), a bare call in `::A` runs `::C`'s body, and Go to Definition
follows the whole chain to `::C`'s header — while a forget anywhere along it
kills the call, exactly as `namespace origin` reports. The chase is bounded
(eight hops, the same cap the rename / alias chase uses), so mutually
importing namespaces cannot spin; past that bound navigation abstains.

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

A **namespace name** is a jump target too. The `::tomato` of `namespace
children ::tomato`, `namespace exists ::tomato`, `namespace delete
::tomato`, `namespace upvar ::tomato v local`, `namespace inscope`, or a
second `namespace eval ::tomato { … }` block all land on the `namespace
eval` blocks that declare that namespace — every one of them, in source
order, because reopening a namespace extends the same namespace rather than
making a new one. The declaring block may live in another file. A
**relative** name resolves against the namespace it is written in, exactly
as Tcl resolves it: inside `namespace eval ::outer`, `namespace exists
inner` means `::outer::inner`, and the same words at the top level mean
`::inner`. Words that only look like namespaces are not: `namespace tail`
and `namespace qualifiers` take an arbitrary string, `namespace import` /
`export` / `forget` take glob patterns, and `namespace origin` / `which`
name commands. A namespace that exists only because it is a parent
(`namespace eval ::p::q::r { … }` really does create `::p::q`) has no name
of its own written anywhere, so nothing is reported for it.

Two spellings surprise people, and both follow the same relative rule. The
**empty** word names the global namespace at the top level — `namespace eval
{} { … }` really does reopen `::`, and `namespace children {}` lists the same
children as `namespace children ::` — so all three spellings are one symbol.
Written inside another namespace the same word means that namespace's
empty-named child, which Tcl refuses to create at all, so it resolves to
nothing rather than to the namespace around it. And a **braced** name is an
ordinary name: `namespace eval {my ns} { … }` creates a real namespace, and
`namespace children {my ns}` jumps to it.

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
- A namespace whose target is computed (`namespace eval $ns { … }`) names no
  fixed namespace, so neither the block nor any reference to it resolves.
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
- `rust/tcl-lsp-core/src/namespace_import.rs` (`mod tests`) — the
  per-import-site export snapshot and the import edge's own lifecycle
  (`alias_live_at`), both shared by the same-document and workspace resolvers
- `rust/tcl-lsp-server/tests/e2e/definition.rs`
  (`wildcard_import_survives_a_later_export_clear_cross_document`,
  `wildcard_import_ignores_an_export_written_after_it_cross_document`,
  `a_forgotten_wildcard_import_stops_resolving_cross_document`,
  `a_forced_import_shadows_the_local_command_cross_document`,
  `a_wildcard_import_chain_follows_to_the_original_source_cross_document`,
  `deleting_the_source_command_kills_the_import_cross_document`)
- `rust/tcl-lsp-server/tests/e2e/issue923_crossdoc.rs` (cross-file namespace
  variables, cross-file class-reference arguments)
- `rust/tcl-lsp-core/src/namespace_symbol.rs` (`mod tests`) — the shared
  namespace resolver definition, hover, and references all answer through
- `rust/tcl-lsp-server/tests/e2e/issue1088_namespace_symbols.rs` (namespace
  names as jump targets, in one file and across files)
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
