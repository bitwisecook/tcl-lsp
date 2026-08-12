# KCS: W135 — Why does the analyser say this command needs a newer package version?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command requires a newer package version than my `package require` provides?

## Why

Some commands only exist from a given version of their package onward (for example, the `ttk::` themed widgets require Tk 8.5). The registry records that minimum as the command's `min_version`. When the document `package require`s the package at a version whose guaranteed floor is *below* that minimum, the command will not exist at runtime and the call fails. Raising the requirement — or dropping the command — avoids a runtime error.

## Symptoms

- A yellow squiggle under the command head, with a message like "`ttk::button` requires Tk 8.5 but `package require` guarantees only 8.4".

## Example that triggers it

```tcl
package require Tk 8.4
ttk::button .b -text Hi
```

The analyser reports **`W135`** on the `ttk::button` call: `ttk::` widgets need Tk 8.5.

## Fix

```tcl
package require Tk 8.5
ttk::button .b -text Hi
```

Raise the `package require` to at least the version the command needs. A `package require` *without* a version is treated as permissive (no floor to compare against), so it never draws W135.

## A guarded `package require` does not raise the floor

Only a `package require` that definitely runs sets the version floor. One inside a branch that may not be taken — an `if` body, a `catch` script, a `try` body, or a `try` `on`/`trap` handler — is recorded as *guarded* and is ignored when comparing versions, because at runtime the package may never have been loaded on the path that reaches the command:

```tcl
catch {package require Tk 8.6}
ttk::button .b -text Hi     ;# still W135 — the require may not have run
```

A `try`'s `finally` script is the one exception: it always runs, whatever the body and the handlers did, so a `package require` there *does* raise the floor. Move the require out of the guard (or add an unguarded one) if you mean it to count.

## How to suppress

Add `# noqa: W135` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [W136 — option needs newer package version](kcs-diagnostic-w136-option-needs-newer-package.md)
- [W120 — missing package require](kcs-diagnostic-w120-missing-package-require.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W120`, `W136`
