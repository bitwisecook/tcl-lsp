# KCS: W313 — Can a destructive file operation with a variable path be exploited?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser warn about a destructive file operation with a variable path?

## Why

An attacker who controls the path can delete, rename, or create files outside the intended directory.

## Symptoms

- A yellow squiggle appears under the path argument, with the message *"file delete with a variable path ($userPath) risks path-traversal. Normalise with [file normalize] and verify it stays within the intended directory."*
- Once the path is normalised but still unverified, the message changes to *"file delete with normalised path ($safe) — verify it stays within the intended directory."*

## Example that triggers it

```tcl
file delete $userPath
```

The analyser reports **`W313`** on the `file delete` call.

## Fix

```tcl
set safe [file normalize $userPath]
if {[string match "/srv/data/*" $safe]} {
    file delete -- $safe
}
```

Normalising alone is not enough — the warning stays until the path is also
checked against the directory it must stay inside. The `--` keeps a
`-`-prefixed value from being read as an option.

## How to suppress

Add `# noqa: W313` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `W300`, `W304`
