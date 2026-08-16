# KCS: W113 — Why does the analyser warn when a proc shadows a built-in?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a procedure shadows a built-in command?

## Why

Defining a proc with the same name as a built-in command silently replaces the original. Callers expecting the built-in behaviour will get the custom proc instead, leading to subtle, hard-to-trace bugs.

## Symptoms

- A yellow squiggle appears under the proc name, with the message "proc 'set' shadows a built-in command".

## Example that triggers it

```tcl
proc set {name value} {
    puts "setting $name to $value"
}
```

The analyser reports **`W113`** on the proc name `set`.

## Fix

```tcl
proc my_set {name value} {
    puts "setting $name to $value"
}
```

Choose a name that does not collide with a built-in command.

## Package-gated commands are excluded

This warning only fires for a genuine core built-in. A proc named after
a command that is gated behind `package require` — a tcllib package,
`argparse`, an `itcl`/TclOO helper, … — is not flagged: that command
does not exist until its package is loaded, and even then a proc of the
same name is that package's own implementation, not a shadow of a core
built-in.

```tcl
package require argparse
proc ::argparse {args} {
    # ... this is argparse's own definition, not a redefinition
}
```

The one exception is a package a dialect profile ships **ambiently** —
an F5 command pack or an EDA vendor tool surface, part of that profile's
genuine, always-present command surface (not something the file ever
`package require`s) — so redefining one of those still warns.

## How to suppress

Add `# noqa: W113` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W116`, `W117`
