# KCS: feature — Cross-file diagnostics

> **Audience:** User
> **Type:** Functionality

## Summary

Diagnostics take account of procs defined in other files and of packages a
`source`d file loads, so a multi-file project stops seeing "Unknown command"
on its own helpers and stops being told to `package require` something it
already has.

## Applies to

all-editors, MCP, diagnostic, warning, multi-file, workspace, source, arity

## How to use

Nothing to enable — this is on by default and needs no configuration.

For example, if `deflib.tcl` defines a three-argument proc, a call in another
open project file is resolved to that definition and checked as though both
were in the same file:

```tcl
# deflib.tcl
proc libtest {left middle right} { return $left }

# caller.tcl
libtest 1 2 ;# E002: Too few arguments for 'libtest'
```

- Open a project with more than one `.tcl` file. Calls to a proc defined in
  a sibling file are recognised, and a call with the wrong number of
  arguments is reported as an error (`E002` too few / `E003` too many),
  exactly as it would be if the proc were in the same file.
- A file that does `source other.tcl`, where `other.tcl` does
  `package require Tk`, may use Tk commands without a `package require` of
  its own.
- `tclLsp.features.crossFileResolution` remains available for a *broader*,
  deliberately lossier match (any command whose bare name exists anywhere in
  the workspace). It is still off by default and is not needed for the
  behaviour above.

## Operational context

For direct calls, there is one cross-document command lookup, shared by go-to-definition,
find-references and diagnostics. It replays the call site's own resolution
candidates in C Tcl's `Tcl_FindCommand` priority order against the workspace
index. Because it matches fully-qualified names rather than bare tails, a
`proc ::deep::buried` does not silence a bare `buried` call that Tcl would
never route there — which is why it is safe to have on by default.

`source` is followed when its path can be resolved statically: a literal
path, or a computed one built from `[file dirname [info script]]` / `$dir`.
The sourced file's `package require`s become available **from the `source`
statement onward**, matching C Tcl, so a Tk command written above the
`source` line is still reported.

Where a fact cannot be proven the server deliberately stays quiet rather than
guessing. That happens when a `source` path cannot be resolved to a file in
the workspace, when the file `load`s an extension or mutates `auto_path`,
when it installs a `namespace unknown` handler or a dynamic `namespace
import`, when a user `proc unknown` has a dynamic dispatch shape, and when
`source` itself has been `rename`d or `interp alias`ed out from under its own
name. In all of those the set of available commands and packages is genuinely
unknowable, and a wrong warning on working code is worse than a missing one.

## Failure modes

- A cross-file call is reported as an unknown command while
  go-to-definition resolves it — the two have drifted onto different
  lookups (issue #1331).
- A Tk command is reported as needing `package require Tk` in a file that
  `source`s a file already requiring it (issue #1332).
- An arity error fires on a proc with an `args` tail, a defaulted
  parameter, or a computed parameter list — the envelope is being read from
  the raw formal count rather than the real arity.
- Diagnostics go quiet across a whole file: check whether something in it
  triggered a deliberate abstention (see above) rather than assuming a
  regression.

## Test anchors

- `editors/vscode/src/test/crossFileDiagnostics.test.ts`

## Discoverability

- [KCS feature index](README.md)
- [Cross-file diagnostics contract](../../../docs/design/contracts/cross-file-diagnostics.md)
- [Command-name resolution contract](../../../docs/design/contracts/command-resolution.md)
- [kcs-feature-diagnostics.md](kcs-feature-diagnostics.md)
- [kcs-feature-unknown-command-resolution.md](kcs-feature-unknown-command-resolution.md)
