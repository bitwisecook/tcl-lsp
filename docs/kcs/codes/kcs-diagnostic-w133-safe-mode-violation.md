# KCS: W133 — why would a manifest fail with "not permitted in safe mode"?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

Reserved — see Status.

## Question

What will W133 report about a line in `tclpkg.tcl`?

## Status

W133 is specified in the diagnostic registry but is not yet emitted by
the analyser. No editor or CLI surface currently reports it, and there
is no `tclLsp.diagnostics.W133` setting to toggle. This page describes
the check as designed, for when it ships.

## Why

The manifest is evaluated in a sandboxed Tcl interpreter that only
permits declarative directives (`package`, `version`, `require`, and
so on). Commands like `exec`, `open`, `source`, `file`, and `puts` are
blocked to prevent untrusted manifests from running arbitrary code.

## Intended message

"tclpkg.tcl directive not permitted in safe mode."

## Example that would trigger it

```tcl
package myapp
version 1.0.0
exec ls /        ;# ← would trigger W133
```

## Fix

Remove the non-directive command. The manifest should only contain
declarative directives — see `tcl pkg init` for the full list.

## Related

- [KCS codes index](README.md)
- [Design: manifest contracts](../../design/contracts/tclpkg-contracts.md)
