# KCS: W102 — Does subst on a variable allow command execution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn when `subst` is called on a variable?

## Why

Variable content undergoes `$` and `[]` substitution, allowing command execution if the value contains embedded commands or variable references.

## Symptoms

- A yellow squiggle appears under the `subst` call, with the message "subst on variable input".

## Example that triggers it

```tcl
subst $template
```

The analyser reports **`W102`** on the `subst` call.

## Fix

```tcl
subst -nocommands -novariables $template
```

Disable command and variable substitution so that only backslash substitution is performed.

## How to suppress

Add `# noqa: W102` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W101`, `W308`
