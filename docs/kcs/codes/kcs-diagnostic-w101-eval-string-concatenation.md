# KCS: W101 — Does eval with string concatenation allow code injection?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `eval` is called with a concatenated string?

## Why

An attacker who controls any part of the concatenated string can inject arbitrary Tcl commands.

## Symptoms

- A yellow squiggle appears under the `eval` call, with the message "eval with string concatenation".

## Example that triggers it

```tcl
set cmd "puts"
eval "$cmd $userInput"
```

The analyser reports **`W101`** on the `eval` call.

## Fix

```tcl
eval [list puts $userInput]
```

Use `list` to construct the command safely, preventing substitution of metacharacters.

## How to suppress

Add `# noqa: W101` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W102`, `W301`
