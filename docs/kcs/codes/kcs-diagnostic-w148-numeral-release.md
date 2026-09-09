# KCS: W148 — Why is this numeral rejected by my Tcl release?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag a number that Tcl 9 accepts?

## Why

Tcl 9 added numeral spellings its predecessors do not parse: the explicit
decimal prefix `0d5`, and digit separators such as `1_000`. On Tcl 8.4–8.6 the
same word is not a number at all, so any arithmetic on it fails at run time.

Leading-zero octal (`010`) is never flagged: it is valid in Tcl 8.x and valid
as decimal in Tcl 9.

## Symptoms

- A yellow squiggle under the numeral, with the message "Numeral '0d5' is not
  accepted by the resolved Tcl numeral grammar."

## Example that triggers it

```tcl
# tcl-dialect: tcl8.6
set n 0d5
puts $n
```

The analyser reports **`W148`** on `0d5`.

## Fix

```tcl
# tcl-dialect: tcl8.6
set n 5
puts $n
```

Use a spelling the target release accepts — `5` for `0d5`, `1000` for
`1_000` — or raise the document's resolved Tcl release if the script really
does need Tcl 9 syntax.

The check abstains when the numeral is dynamic or the document has no
resolved release.

## How to suppress

Add `# noqa: W148` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = W148` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.W148` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W137`, `W138`, `W144`
