# KCS: T102 — Why does the analyser warn about tainted data in option position?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data passed in option position without a `--` terminator?

## Why

A value starting with `-` is interpreted as an option, letting the attacker change command behaviour.

## Symptoms

- A yellow squiggle under the argument, with the message "Tainted variable
  $pattern in option position of 'glob' without '--' terminator; risk of
  option injection".
- An **Insert '--' option terminator** quick fix on the diagnostic.

## Example that triggers it

```tcl
set pattern [gets stdin]
set matches [glob $pattern]
```

The analyser reports **`T102`** on `$pattern`: the value could start with `-`
and `glob` scans that position for options.

## Fix

```tcl
set pattern [gets stdin]
set matches [glob -- $pattern]
```

Add `--` before the argument to end option parsing. The check only looks at
commands the registry records a `--` terminator for, and only at positions
still inside the option-scanning region — a value already known to start with
a path separator, or to be an address, port, or host name, is left alone.

## How to suppress

Add `# noqa: T102` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = T102` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.T102` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T101`, `W304`
