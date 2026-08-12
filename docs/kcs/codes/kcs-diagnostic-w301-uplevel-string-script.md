# KCS: W301 — Does uplevel with a string-built script allow injection?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `uplevel` is called with a string-built script?

## Why

Multiple arguments or an unbraced script concatenate at runtime, creating an injection vector that lets attacker-controlled data execute as Tcl code.

## Symptoms

- A yellow squiggle appears under the `uplevel` call, with the message "uplevel with string-built script".

## Example that triggers it

```tcl
uplevel 1 "set x $userInput"
```

The analyser reports **`W301`** on the `uplevel` call.

## Fix

```tcl
uplevel 1 [list set x $userInput]
```

Use `list` to construct the script safely, preventing metacharacter injection.

## How to suppress

Add `# noqa: W301` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W101`, `W309`
