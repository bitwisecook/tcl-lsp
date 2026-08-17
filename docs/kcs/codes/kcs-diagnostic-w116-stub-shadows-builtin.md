# KCS: W116 — Why does the analyser warn about a stub shadowing a built-in?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a stub command shadows a built-in command?

## Why

A stub (e.g. from `package ifneeded` or a generated wrapper) that redefines a built-in command can silently alter the behaviour callers expect, causing unpredictable results across the application.

## Symptoms

- A yellow squiggle appears under the stub name, with the message "stub command 'puts' shadows a built-in".

## Example that triggers it

```tcl
interp alias {} puts {} ::mylog::puts_wrapper
```

The analyser reports **`W116`** on the alias target `puts`.

## Fix

```tcl
interp alias {} log_puts {} ::mylog::puts_wrapper
```

Give the stub a distinct name that does not collide with any built-in.

## How to suppress

Add `# noqa: W116` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W113`, `W117`
