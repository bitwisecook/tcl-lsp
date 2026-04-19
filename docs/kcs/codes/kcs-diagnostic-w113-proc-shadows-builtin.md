# KCS: W113 — Why does the analyser warn when a proc shadows a built-in?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

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

## How to suppress

Add `# noqa: W113` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W116`, `W117`
