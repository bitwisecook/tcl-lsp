# KCS: W131 — why is the lockfile out of sync?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

Reserved — see Status.

## Question

What will W131 report about `tclpkg.tcl` and `tclpkg.lock`?

## Status

W131 is specified in the diagnostic registry but is not yet emitted by
the analyser. No editor or CLI surface currently reports it, and there
is no `tclLsp.diagnostics.W131` setting to toggle. This page describes
the check as designed, for when it ships.

## Why

The manifest has changed since the lockfile was last written. If the
two files disagree on the dependency set, builds are not reproducible
until `tcl pkg install` re-syncs them.

## Intended message

"tclpkg.lock is out of sync with tclpkg.tcl — run 'tcl pkg install'."

## Example that would trigger it

Edit `tclpkg.tcl` to add or remove a `require` line, then save without
running `tcl pkg install`.

## Fix

```sh
tcl pkg install
```

## Related

- [KCS codes index](README.md)
- Related codes: `W130`, `W132`
