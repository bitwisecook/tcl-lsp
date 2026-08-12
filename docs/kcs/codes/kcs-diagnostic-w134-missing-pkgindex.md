# KCS: W134 — why would pkgIndex.tcl be reported missing?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

Reserved — see Status.

## Question

What will W134 report after installing a package?

## Status

W134 is specified in the diagnostic registry but is not yet emitted by
the analyser. No editor or CLI surface currently reports it, and there
is no `tclLsp.diagnostics.W134` setting to toggle. This page describes
the check as designed, for when it ships.

## Why

The resolver found and fetched the package, but the extracted tree has
no `pkgIndex.tcl` file. Without one, Tcl's `package require` will not
find the package at runtime.

## Intended message

"Package resolved but no pkgIndex.tcl found — 'package require' will
fail at runtime."

## Example that would trigger it

A package archive that ships only `.tcl` files without a
`pkgIndex.tcl`.

## Fix

Add a `pkgIndex.tcl` to the package upstream, or create one manually in
`lib/<pkg>-<ver>/`:

```tcl
package ifneeded mypkg 1.0.0 [list source [file join $dir mypkg.tcl]]
```

## Related

- [KCS codes index](README.md)
- [Design: package loading](../../design/contracts/tclpkg-contracts.md)
- Related codes: `W130`, `W131`
