# KCS: W144 — Deprecated at the resolved version

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that a command, subcommand, option, or argument
value is deprecated, when the code still runs?

## Why

Every versioned thing in the registry carries the same three releases: the
release that **introduced** it, the release that **deprecated** it, and the
release that **retired** it. They are independent, so a deprecated item is
still perfectly usable — it is simply on notice.

The three states get three different codes, so a message never has to guess
which one you meant:

| State | Code | Meaning |
|---|---|---|
| Not introduced yet | `W135` (command, subcommand, argument value) / `W136` (option) | The resolved version predates the introducing release. |
| Deprecated | **`W144`** | The resolved version is at or past the deprecating release, and the item is still available. |
| Retired | `W139` | The resolved version is at or past the retiring release, so the item is gone. |

The retiring release is **exclusive**: `retired: 10.0.0` means the item is
already gone in 10.0.0, not that 10.0.0 is the last release with it. A
retired item is never also reported as deprecated — `W139` supersedes
`W144`.

The version the analyser compares against is the resolved floor. For package
syntax, that is the active profile's library pin, raised by any unconditional
versioned `package require` in the file. For Tcl core syntax, it is the active
Tcl dialect version, likewise raised by an unconditional `package require Tcl`
floor. When no floor can be resolved, nothing is reported.

## Symptoms

- A yellow squiggle under a command, subcommand, `-option`, or literal
  argument value, with a message naming its package or Tcl core version.

## Example that triggers it

```tcl
# tcl-dialect: tcl9.0
interp slaves
```

`interp slaves` still works in Tcl 9.0.4, but Tcl 8.6 introduced the preferred
`interp children` spelling and Tcl 9 documents that form. The registry records
the compatibility spelling as deprecated from Tcl 8.6, so W144 is a warning,
not an availability error.

## Fix

Use the registry-provided quick fix when one is offered. For this example it
replaces only the subcommand word, producing `interp children`; the registry
marks that edit semantics-equivalent, so it is safe for bulk safe fixes. A
dynamic selector such as `interp $operation` deliberately receives no W144 or
quick fix because the analyser cannot prove which operation runs. When there
is no replacement, the warning is informational: the item works today and
will keep working until a retiring release is recorded, at which point the
code becomes `W139`.

## How to suppress

Add `# noqa: W144` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W135`, `W136` (not introduced yet), `W139` (retired),
  `IRULE1003` (the iRules-event-specific deprecation warning).
