# KCS: IRULE5006 — Why does the analyser warn about a top-level command in a nested body?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag a `proc`, `when`, or `timing` command used inside a nested body?

## Why

Commands like `proc`, `when`, and `timing` must be at the iRule top level; nesting them produces undefined behaviour.

## Symptoms

- A squiggle appears under the command, with the message "top-level-only command used inside a nested body".

## Example that triggers it

```tcl
if {1} {
  proc inner {} {}
}
```

The analyser reports **`IRULE5006`** because `proc` is defined inside a control structure.

## Fix

Move `proc` definitions outside all control structures:

```tcl
proc inner {} {}
if {1} {
  call inner
}
```

## How to suppress

Add `# noqa: IRULE5006` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `IRULE5005`, `IRULE5007`
