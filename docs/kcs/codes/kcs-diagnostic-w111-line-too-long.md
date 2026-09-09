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

Long lines reduce readability, make side-by-side diffs harder to review, and
break tools that assume a reasonable line width.

The limit is `tclLsp.style.lineLength`, which defaults to 120 characters. A
trailing carriage return is stripped before counting, so CRLF endings do not
inflate the length.

## Symptoms

- A yellow squiggle over the whole line, with the message "Line exceeds 120
  characters (143 characters)".

## Example that triggers it

```tcl
set result [some_very_long_command_name $arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7 $arg8 $arg9 $arg10 $arg11 $arg12 $arg13 $arg14 $arg15 $arg16]
```

The analyser reports **`W111`** on the whole line.

## Fix

```tcl
set result [some_very_long_command_name \
    $arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7 $arg8 \
    $arg9 $arg10 $arg11 $arg12 $arg13 $arg14 $arg15 $arg16]
```

Break the line using backslash-newline continuation.

## How to suppress

Add `# noqa: W111` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W112`, `W108`
