# KCS: W130 — why does my editor say a package is not in the lockfile?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

default

## Question

Why does my editor report W130 on a `require` line in `tclpkg.tcl`?

## Why

The manifest declares a dependency, but the lockfile does not contain a
matching entry. The resolver has not run since the dependency was added, so
packages will not be available at runtime.

## Symptoms

- Yellow squiggle on a `require` or `dev-require` line in `tclpkg.tcl`.
- Problems panel shows: "tclpkg.tcl requires package but it is not in
  tclpkg.lock — run 'tcl pkg install'."

## Example that triggers it

```tcl
package myapp
version 1.0.0
require json 1.3.5
```

The analyser reports **W130** on the `require json 1.3.5` line when
`tclpkg.lock` does not contain an entry for `json`.

## Fix

```sh
tcl pkg install
```

This runs the MVS resolver, writes `tclpkg.lock`, and materialises
packages into `lib/`.

## How to suppress

Disable via editor settings: `tclLsp.diagnostics.W130: false`. This
diagnostic is on by default because an out-of-sync lockfile is almost
always unintentional.

## Related

- [KCS codes index](README.md)
- [tcl pkg](../features/kcs-feature-tcl-pkg.md) — the package manager
- Related codes: `W131`, `W132`
