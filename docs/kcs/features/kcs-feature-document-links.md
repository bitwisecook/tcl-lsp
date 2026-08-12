# KCS: feature — Document Links

> **Audience:** User
> **Type:** Functionality

## Summary

Clickable links on the files a script loads: the path argument of
`source`, and the package name in `package require`.

## Applies to

all-editors, analyser

## How to use

- **Editor**: Ctrl+Click a `source` path to open that file in a new tab.
  A `package require` name is underlined and carries a tooltip, but has
  no target to open — the package index is not scanned.
- **Settings**: Toggle with `tclLsp.features.documentLinks`.

## Operational context

![Document links on computed source paths](../screenshots/32-document-links.png)

A relative path resolves against the document's own directory, and `~`
expands against `$HOME`. A computed path resolves when it is built from
`[info script]`, `[file dirname …]`, `[file join …]`, `[file normalize
…]`, literal words, and variables the document assigns exactly once —
at the top level, or inside a `namespace eval` body (`variable dir
[file dirname [info script]]` referenced as `$::pkg::dir`) — so the
common idiom links either way it is spelled:

```tcl
set currentDir [file normalize [file dirname [info script]]]
source [file join $currentDir testUtilities.tcl]
```

Those variables chain, so a directory reached through an intermediate
links as readily as a direct one:

```tcl
set dir       [file dirname [file normalize [info script]]]
set sourceDir [file join $dir src]
source [file join $sourceDir generalClasses.tcl]
```

The same evaluator resolves the same expression everywhere it is asked:
the clickable link, go-to-definition and references across the sourced
file, cross-file diagnostics that follow `source`, and the `auto_path`
directories a `lappend auto_path $libDir` registers for `package
require` resolution. A value containing spaces stays one path element
(`set d {my dir}` joins to `my dir/x.tcl`, as `tclsh` does), because
variables resolve as values in the parsed expression, never by splicing
text.

Only the file name — `testUtilities.tcl` — is underlined, not the whole
`[file join …]` substitution. The substitution is code, with its own
highlighting; an editor paints a link range in one flat link colour, so
underlining all of it would hide the colouring of `file`, `join`, and
`$currentDir` (issue #775).

## Failure modes

- **No link on a computed path.** The path is outside the evaluator's
  supported subset, so the provider abstains rather than guess a target.
  A directory built with a command the subset does not model — `file
  readlink`, `pwd`, `exec` — is the usual cause, as is one assigned by
  more than one top-level `set`, since which value a given `source` sees
  is a question the provider does not ask. A re-assigned directory also
  stops anything computed from it resolving.
- **No link when the directory is set inside a `proc` or an `if`.**
  Only load-time assignments are read — the top level and `namespace
  eval` bodies, which run unconditionally when the file is sourced. A
  guarded assignment is not known: Tk's own `$::ttk::library` abstains
  because its `set` hides behind `if {![info exists library]}`, so its
  value genuinely is not static.
- **A variable from another file resolves only on agreement.** A
  namespace variable assigned in one file and read in a `source` in
  another (OSVVM's `$::osvvm::OsvvmScriptDirectory` shape) resolves
  when every file that sources the reader supplies the same value for
  it, established before its `source` statement. One dissenting or
  silent route drops the name: whichever file actually ran, a kept
  value is the value. A reader nothing sources gets no imports at all.
- **No link on a relative `file normalize`.** `[file normalize lib]`
  resolves against the interpreter's working directory at run time,
  which is not knowable statically, so it does not fold. Anchor it —
  `[file normalize [file join [file dirname [info script]] lib]]`.
- **No link when the substitution ends in a variable.** `source [file
  join $dir $name]` has no literal word to anchor the link on, so none
  is offered even when the path itself resolves.
- **No link on a relative path in an unsaved file.** There is no
  document directory to resolve against until the file is saved.

## Example

In this Tcl file:

```tcl
package require tcltest
set currentDir [file dirname [info script]]
source [file join $currentDir testUtilities.tcl]
source lib/helpers.tcl
```

`tcltest` is underlined with a tooltip, `testUtilities.tcl` and
`lib/helpers.tcl` open their files on Ctrl+Click, and the rest of the
`[file join …]` call keeps its normal highlighting.

## Discoverability

- [KCS feature index](README.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
