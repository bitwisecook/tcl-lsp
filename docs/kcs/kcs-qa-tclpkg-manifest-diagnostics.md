# KCS: What does the editor check inside a tclpkg.tcl manifest?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Question

Why does my `tclpkg.tcl` package manifest no longer show "Unknown command"
warnings, and what is checked instead?

## Answer

A file named `tclpkg.tcl` is a package manifest for the `tcl pkg` package
manager. Its directives (`package`, `version`, `require`, `entry`, …) are
not ordinary Tcl commands, and two of them (`package`, `entry`) share a
name with a real Tcl or Tk command.

The language server and `tcl diag` now recognise the manifest by its file
name and analyse it against the manifest's own command set:

- Each directive resolves to its manifest meaning — `entry main.tcl` is
  the entry-point declaration, never the Tk `entry` widget, so no
  "requires `package require Tk`" warning appears.
- Directive argument counts are checked against the manifest grammar —
  `require json` (missing the minimum version) is flagged.
- A misspelt directive still shows "Unknown command" — the manifest
  command set is closed, so typos surface.
- Hovering a directive shows its synopsis, for example
  `require <name> <minimum> ?-source <url>?`.

## Example

```tcl
package demo-app
version 0.1.0
require json 1.0.0
entry   main.tcl
```

Before this change every line above showed a warning; now the file is
clean, and only genuine mistakes are flagged.

The directive set lives in the command registry
(`rust/tcl-registry/src/scoped.rs`, `TCLPKG_MANIFEST_ENV`), mirroring the
`tcl pkg` manifest parser.
