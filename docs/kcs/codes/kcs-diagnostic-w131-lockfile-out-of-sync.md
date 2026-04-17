# KCS: W131 — why does my editor say the lockfile is out of sync?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

default

## Question

Why does my editor report W131 on `tclpkg.tcl`?

## Why

The manifest has changed since the lockfile was last written. The two
files no longer agree on the dependency set, so builds may not be
reproducible.

## Symptoms

- Yellow squiggle on the `package` line of `tclpkg.tcl`.
- Problems panel: "tclpkg.lock is out of sync with tclpkg.tcl — run
  'tcl pkg install'."

## Example that triggers it

Edit `tclpkg.tcl` to add or remove a `require` line, then save without
running `tcl pkg install`.

## Fix

```sh
tcl pkg install
```

## How to suppress

`tclLsp.diagnostics.W131: false`

## Related

- [KCS codes index](README.md)
- Related codes: `W130`, `W132`
