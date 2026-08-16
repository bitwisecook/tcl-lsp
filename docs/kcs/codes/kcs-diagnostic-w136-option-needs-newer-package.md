# KCS: W136 — Why does the analyser say this option needs a newer package version?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a command option requires a newer package version than my `package require` provides?

## Why

Some options are added to a command in a later package release (for example, `entry -placeholder` arrives in Tk 8.7). The registry records that minimum as the option's `min_version`. When the document `package require`s the package at a version whose guaranteed floor is *below* that minimum, passing the option is a runtime error — the older command rejects the unknown switch. Raising the requirement — or dropping the option — avoids the failure.

## Symptoms

- A yellow squiggle under the option token, with a message like "Option '-placeholder' on 'entry' requires Tk 8.7 but `package require` guarantees only 8.6".

## Example that triggers it

```tcl
package require Tk 8.6
entry .e -placeholder "type here"
```

The analyser reports **`W136`** on `-placeholder`: the option needs Tk 8.7.

## Fix

```tcl
package require Tk 8.7
entry .e -placeholder "type here"
```

Raise the `package require` to at least the version the option needs. A `package require` *without* a version is treated as permissive (no floor to compare against), so it never draws W136.

A `package require` inside a branch that may not be taken does not raise the floor either — see [the guarded-require section of the W135 note](kcs-diagnostic-w135-command-needs-newer-package.md#a-guarded-package-require-does-not-raise-the-floor).

## How to suppress

Add `# noqa: W136` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W135 — command needs newer package version](kcs-diagnostic-w135-command-needs-newer-package.md)
- [W004 — option not available in dialect](kcs-diagnostic-w004-dialect-invalid-option.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W004`, `W135`
