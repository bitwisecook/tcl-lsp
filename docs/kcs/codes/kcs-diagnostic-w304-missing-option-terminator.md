# KCS: W304 — Can a missing option terminator cause option injection?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn about a missing `--` option terminator?

## Why

User-controlled values starting with `-` are interpreted as options, enabling option injection that can alter the command's behaviour.

## Symptoms

- A yellow squiggle appears under the command call, with the message "missing option terminator --".

## Example that triggers it

```tcl
file exists $path
```

The analyser reports **`W304`** on the `file exists` call.

## Fix

```tcl
file exists -- $path
```

Add `--` before user-supplied arguments to prevent option injection.

## How to suppress

Add `# noqa: W304` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W300`, `W313`
