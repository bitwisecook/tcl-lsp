# KCS: W130 — why is a package not in the lockfile?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

Reserved — see Status.

## Question

What will W130 report about `tclpkg.tcl` and `tclpkg.lock`?

## Status

W130 is specified in the diagnostic registry but is not yet emitted by
the analyser. No editor or CLI surface currently reports it, and there
is no `tclLsp.diagnostics.W130` setting to toggle. This page describes
the check as designed, for when it ships.

## Why

The manifest declares a dependency, but the lockfile has no matching
entry. If the resolver has not run since the dependency was added,
`tcl pkg install` needs to run before the package is available at
runtime.

## Intended message

"tclpkg.tcl requires package but it is not in tclpkg.lock — run 'tcl
pkg install'."

## Example that would trigger it

```tcl
package myapp
version 1.0.0
require json 1.3.5
```

If `tclpkg.lock` has no entry for `json`, W130 would report on the
`require json 1.3.5` line.

## Fix

```sh
tcl pkg install
```

This runs the resolver, writes `tclpkg.lock`, and materialises
packages into `lib/`.

## Related

- [KCS codes index](README.md)
- [tcl pkg](../features/kcs-feature-tcl-pkg.md) — the package manager
- Related codes: `W131`, `W132`
