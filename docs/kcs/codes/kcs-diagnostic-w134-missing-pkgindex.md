# KCS: W134 — why does my editor say pkgIndex.tcl is missing?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

default

## Question

Why does my editor report W134 after installing a package?

## Why

The resolver found and fetched the package, but the extracted tree does
not contain a `pkgIndex.tcl` file. Without one, Tcl's `package require`
will not find the package at runtime.

## Symptoms

- Yellow squiggle on the package name in `tclpkg.lock` or on the
  `require` line in `tclpkg.tcl`.
- Problems panel: "Package resolved but no pkgIndex.tcl found —
  'package require' will fail at runtime."

## Example that triggers it

A package archive that ships only `.tcl` files without a `pkgIndex.tcl`.

## Fix

Add a `pkgIndex.tcl` to the package upstream, or create one manually in
`lib/<pkg>-<ver>/`:

```tcl
package ifneeded mypkg 1.0.0 [list source [file join $dir mypkg.tcl]]
```

## How to suppress

`tclLsp.diagnostics.W134: false`

## Related

- [KCS codes index](README.md)
- [Design: package loading](../../design/contracts/tclpkg-cache.md)
- Related codes: `W130`, `W131`
