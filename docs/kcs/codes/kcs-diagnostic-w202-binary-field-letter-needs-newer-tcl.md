# KCS: W202 — Why does the analyser say my binary field letter needs a newer Tcl?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a `binary format` or `binary scan` field
letter is not available at my Tcl version?

## Why

Seven field letters arrive in Tcl 8.5 and do not exist on 8.4:

| Letter | Field |
| --- | --- |
| `t` | 16-bit integer, native byte order |
| `n` | 32-bit integer, native byte order |
| `m` | 64-bit integer, native byte order |
| `r` | 32-bit float, little-endian |
| `R` | 32-bit float, big-endian |
| `q` | 64-bit float, little-endian |
| `Q` | 64-bit float, big-endian |

On Tcl 8.4 each is rejected at run time with `bad field specifier`, for both
`binary format` and `binary scan` — they share one template parser. The
analyser reads the literal template and compares it against the file's
effective Tcl version: the dialect profile, raised by any `package require
Tcl`.

This is the sibling of [`W200`](kcs-diagnostic-w200-binary-format-modifier-needs-newer-tcl.md),
which gates the unsigned `u` suffix. They are separate codes because the fixes
differ: a suffix can simply be dropped, while an absent field letter needs a
different field with the byte order written out.

## Symptoms

- A yellow squiggle under the template, with the message "binary field
  specifier 'q' requires Tcl 8.5 but tcl8.4 provides 8.4."
- One diagnostic per template per code — every field shares the template
  token, so several gated letters give one squiggle, not one each.

## Example that triggers it

```tcl
# tcl-dialect: tcl8.4
set packed [binary format q 1.5]
```

The analyser reports **`W202`** on the template.

## Fix

Raise the floor so the field exists:

```tcl
# tcl-dialect: tcl8.4
package require Tcl 8.5

set packed [binary format q 1.5]
```

Or pick a field that 8.4 has. The native-order integers `t`, `n` and `m` have
explicit-endian equivalents on every release — `s`/`S` for 16-bit, `i`/`I` for
32-bit, `w`/`W` for 64-bit — so choose the byte order you actually want rather
than the host's:

```tcl
# tcl-dialect: tcl8.4
set packed [binary format i $n]
```

The floating-point fields `r`, `R`, `q` and `Q` have no 8.4 equivalent. On 8.4
use `f` or `d` (native order), or pack the bytes yourself.

A dynamic template (`binary format $fmt $x`) is not checked — the analyser
cannot see its text.

## How to suppress

Add `# noqa: W202` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = W202` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.W202` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W200`, `W137`, `W138`, `W148`
