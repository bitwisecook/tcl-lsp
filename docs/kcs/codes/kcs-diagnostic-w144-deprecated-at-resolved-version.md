# KCS: W144 — Deprecated at the resolved package version

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

The version the analyser compares against is the resolved floor: the active
profile's library pin, raised by any unconditional versioned `package
require` in the file. When no floor can be resolved, nothing is reported.

## Symptoms

- A yellow squiggle under a command, subcommand, `-option`, or literal
  argument value, with a message like:
  "Option '-foo' on 'bar' is deprecated as of Tk 8.7; `package require`
  guarantees only 8.7."

## Example that triggers it

```tcl
when AUTH_SUCCESS {
    log local0. "authorised"
}
```

The `AUTH_*` iRules events were introduced in BIG-IP 9.0.0 and deprecated in
9.4.0. They still fire, so this is a warning rather than an error, and the
analyser names the deprecating release.

## Fix

Move to the replacement the documentation names — for the example above,
`AUTH_RESULT`. When there is no replacement, the warning is informational:
the item works today and will keep working until a retiring release is
recorded, at which point the code becomes `W139`.

## How to suppress

Add `# noqa: W144` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `W135`, `W136` (not introduced yet), `W139` (retired),
  `IRULE1003` (the iRules-event-specific deprecation warning).
