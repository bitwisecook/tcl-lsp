# KCS: H300 — Possible paste error — repeated assignment to same variable with same value

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, cfg

## Profiles

default

## Question

Why does the analyser hint that I may have pasted a duplicate assignment?

## Why

Two consecutive `set` statements that assign the same static value to the same variable are almost always a paste error. The second write is redundant: it cannot change the value, and the most likely explanation is that the author meant to assign a different variable or a different value. Variables whose names begin with `_` are exempt because they are conventionally used as throwaway or intentionally repeated placeholders.

## Symptoms

- A grey hint underline appears under the second `set` statement, with a message such as: "Possible paste error: repeated assignment to 'x' with static value '1'; did you mean to assign a different variable?"

## Example that triggers it

```tcl
set x 1
set x 1
```

The analyser reports **`H300`** on the second `set x 1` line.

## Fix

```tcl
set x 1
set y 1
```

Either assign a different variable name or a different value on the second statement. If the repeated write is intentional (e.g. resetting a counter inside a loop), rename one variable or add a `# noqa: H300` suppression.

## How to suppress

Add `# noqa: H300` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [cfg](../../GLOSSARY.md#cfg)
- Related codes: `W211`, `W220`
