# KCS: W132 — why does a package have an integrity mismatch?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic

## Profiles

Reserved — see Status.

## Question

What will W132 report about a package in `tclpkg.lock`?

## Status

W132 is specified in the diagnostic registry but is not yet emitted by
the analyser. No editor or CLI surface currently reports it, and there
is no `tclLsp.diagnostics.W132` setting to toggle. This page describes
the check as designed, for when it ships.

## Why

The SHA-256 hash of the package in the content-addressable cache does
not match the hash recorded in the lockfile — the cached files may
have been modified or corrupted.

## Intended message

"tclpkg.lock integrity mismatch — CAS hash differs from lockfile."

## Example that would trigger it

Manually editing a file inside the local tclpkg content-addressable
cache changes the worktree hash and would trigger this diagnostic.

## Fix

Delete the corrupted cache entry and reinstall:

```sh
tcl pkg install
```

The resolver re-fetches the package and recomputes the hash.

## Related

- [KCS codes index](README.md)
- [Design: cache integrity](../../design/contracts/tclpkg-contracts.md)
- Related codes: `W130`, `W131`
