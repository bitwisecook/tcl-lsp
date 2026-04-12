# KCS: W133 — why does my manifest fail with "not permitted in safe mode"?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, tclpkg

## Profiles

default

## Question

Why does my editor report W133 on a line in `tclpkg.tcl`?

## Why

The manifest is evaluated in a sandboxed Tcl interpreter that only permits
13 directives (`package`, `version`, `require`, etc.). Commands like
`exec`, `open`, `source`, `file`, and `puts` are blocked to prevent
untrusted manifests from running arbitrary code.

## Symptoms

- Red squiggle on the offending command in `tclpkg.tcl`.
- Problems panel: "tclpkg.tcl directive not permitted in safe mode."

## Example that triggers it

```tcl
package myapp
version 1.0.0
exec ls /        ;# ← triggers W133
```

## Fix

Remove the non-directive command. The manifest should only contain
declarative directives — see `tcl pkg init` for the full list.

## How to suppress

`tclLsp.diagnostics.W133: false`

This is not recommended — W133 fires only when a manifest tries to run
a command the safe-mode sandbox blocks. The diagnostic is the
user-visible side of a security boundary; the command is already refused
at runtime regardless of whether the diagnostic is shown.

## Related

- [KCS codes index](README.md)
- [Design: manifest contracts](../../design/contracts/tclpkg-manifest.md)
