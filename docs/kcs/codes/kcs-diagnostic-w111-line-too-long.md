# KCS: W111 — Why does the analyser flag long lines?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why does the analyser warn when a line exceeds the maximum length?

## Why

Excessively long lines reduce readability, make side-by-side diffs harder to review, and may break tools that assume a reasonable line width.

## Symptoms

- A yellow squiggle appears at the end of the line, with the message "line exceeds maximum length".

## Example that triggers it

```tcl
set result [some_very_long_command_name $arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7 $arg8 $arg9 $arg10]
```

The analyser reports **`W111`** on the line that exceeds the configured limit.

## Fix

```tcl
set result [some_very_long_command_name \
    $arg1 $arg2 $arg3 $arg4 $arg5 \
    $arg6 $arg7 $arg8 $arg9 $arg10]
```

Break the line using backslash-newline continuation.

## How to suppress

Add `# noqa: W111` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W108`
