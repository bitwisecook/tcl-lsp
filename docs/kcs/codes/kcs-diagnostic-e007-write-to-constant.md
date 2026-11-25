# KCS: E007 — Why can’t I change this variable?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

tcl9.0, tcl9.1

## Question

Why does the analyser report `E007` when `set` writes to a variable?

## Why

Tcl 9 lets `const` initialise a variable whose value stays fixed.
Reading that variable is valid. The built-in `set` command rejects writes
to it.

## Symptoms

The editor marks the target with `E007` and the message “Cannot write to
a constant variable”.

## Example that triggers it

```tcl
const limit 3
set limit 4
```

## Fix

If the value should stay fixed, choose its final value in `const` and remove
the later assignment. If it needs to change, initialise it with `set`.

## How to suppress

Add `# noqa: E007` on the line above the assignment.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
