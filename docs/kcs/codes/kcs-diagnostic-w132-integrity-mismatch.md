# KCS: W132 — why does my editor say there is an integrity mismatch?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

default

## Question

Why does my editor report W132 on a package in `tclpkg.lock`?

## Why

The SHA-256 hash of the package in the content-addressable cache does not
match the hash recorded in the lockfile. The cached files may have been
modified or corrupted.

## Symptoms

- Red squiggle on the package entry in `tclpkg.lock`.
- Problems panel: "tclpkg.lock integrity mismatch — CAS hash differs
  from lockfile."

## Example that triggers it

Manually editing a file inside `~/.cache/tcl-lsp/tclpkg/cas/` changes
the worktree hash and triggers this diagnostic.

## Fix

Delete the corrupted cache entry and reinstall:

```sh
tcl pkg install
```

The resolver re-fetches the package and recomputes the hash.

## How to suppress

`tclLsp.diagnostics.W132: false`. Not recommended — integrity mismatches
can indicate supply-chain tampering.

## Related

- [KCS codes index](README.md)
- [Design: cache integrity](../../design/contracts/tclpkg-cache.md)
- Related codes: `W130`, `W131`
