# KCS: W200 — Why does the analyser flag a `u` in my binary format string?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn about an unsigned modifier in a `binary format`
or `binary scan` specifier?

## Why

The `u` modifier — `cu`, `su`, `iu`, `wu`, and the rest — arrives in Tcl 8.5
(TIP 275). On Tcl 8.4 the same format string is rejected at run time with
`bad field specifier "u"`. The analyser reads the literal format string and
compares it against the file's effective Tcl version: the dialect profile,
raised by any `package require Tcl`.

Tcl accepts the `u` after *any* field letter, not only the integer ones, so
`au` is flagged on 8.4 exactly like `iu`. Tcl has no `s` modifier: an `s`
after another specifier (`ss`, `is`) is a second short-integer field on every
release, so the analyser leaves it alone.

The field letters themselves can also postdate the target — `t`, `n`, `m`,
`r`, `R`, `q` and `Q` arrive in 8.5 too. That is
[`W202`](kcs-diagnostic-w202-binary-field-letter-needs-newer-tcl.md), a
separate code because the fix differs: a suffix can be dropped, an absent
letter needs a different field.

## Symptoms

- A yellow squiggle under the format string, with the message "unsigned
  modifier 'u' on binary format specifier requires Tcl 8.5 but tcl8.4 provides
  8.4."
- One diagnostic per format string — every field shares the format token, so
  several gated modifiers give one squiggle, not one each.

## Example that triggers it

```tcl
# tcl-dialect: tcl8.4
set data [binary format iu 42]
puts $data
```

The analyser reports **`W200`** on the format string.

## Fix

Raise the floor so the modifier is available:

```tcl
# tcl-dialect: tcl8.4
package require Tcl 8.5

set data [binary format iu 42]
puts $data
```

Or drop the modifier and mask the value yourself, which is what the plain
specifier does on 8.4.

A dynamic format string (`binary format $fmt $x`) is not checked — the
analyser cannot see its text.

## How to suppress

Add `# noqa: W200` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = W200` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.W200` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W202`, `W137`, `W138`, `W148`
