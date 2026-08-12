# KCS: W138 — Why does the analyser say this format conversion needs a newer Tcl?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a `%` conversion in my `format` or `scan`
string requires a newer Tcl version?

## Why

`format` and `scan` take a small mini-language of `%` conversions, and
some conversions were added in later Tcl releases. `%b` (binary) arrives
in Tcl 8.6; on 8.4 or 8.5 the same call raises "bad field specifier". The
`%llu` unsigned-bignum combination arrives in Tcl 9.0; on 8.6 it raises
"unsigned bignum format is invalid". The analyser reads the literal
format string, finds the gated conversions, and compares each against the
file's effective Tcl version — the dialect profile, raised by any
`package require Tcl`.

## Symptoms

- A yellow squiggle under the format string, with a message like
  "`format` conversion %b binary conversion in 'format' requires Tcl 8.6
  but tcl8.5 provides 8.5".
- One diagnostic per gated conversion in the string.

## Example that triggers it

```tcl
puts [format "flags: %b" $mask]
```

Analysed under the `tcl8.5` dialect, the analyser reports **`W138`** on
the format string: `%b` needs Tcl 8.6.

## Fix

Raise the floor so the conversion is available:

```tcl
package require Tcl 8.6

puts [format "flags: %b" $mask]
```

Or rewrite the call using a conversion the older release accepts — for
`%b` on 8.5, build the binary text yourself, or print in hex with `%x`.

Only `printf`-style format strings are checked. `clock format`'s field
string, `binary`'s cursor spec, and `regsub`'s replacement template are
different mini-languages and are left alone. A dynamic format string
(`format $fmt $x`) is not checked — the analyser cannot see its text.

## How to suppress

Turn the code off for a project with `disabled = W138` under
`[diagnostics]` in `.tcl-lsp.ini`, for one file with a
`# tcl-lsp: disable=W138` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W138` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W135`, `W136`, `W137`, `W200`
