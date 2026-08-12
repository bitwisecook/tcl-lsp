# KCS: W141 — Why does the analyser say this option value has the wrong shape?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that the value I passed to an option is
malformed, even though it is not from a fixed list of choices?

## Why

Some options accept an open-ended value that still has to be *shaped* a
particular way. `return -errorstack` is the clearest case: `tclsh` rejects
an odd-sized list with "forbidden odd-sized list for -errorstack". The
value is not wrong because it is outside a closed set — any even-sized
list is fine — it is wrong because it is structurally malformed.

W141 is the sibling of `W127`. `W127` reports a value that is outside a
command's closed set of allowed values; W141 reports a value that fails
the option's own content check.

## Symptoms

- A yellow squiggle under the option's value word, with a message like
  "value must be an even-sized list (option '-errorstack' on 'return')".

## Example that triggers it

```tcl
proc load {path} {
  return -code error -errorstack {CALL load extra} "cannot read $path"
}
```

The `-errorstack` value has three elements. The analyser reports
**`W141`** on that value word.

## Fix

```tcl
proc load {path} {
  return -code error -errorstack {CALL load} "cannot read $path"
}
```

Give the option a value of the shape it declares — here, a list with an
even number of elements.

A dynamic value (`-errorstack $stack`) is skipped: the analyser cannot
see what the variable holds, so it abstains rather than guess.

## How to suppress

Turn the code off for a project with `disabled = W141` under
`[diagnostics]` in `.tcl-lsp.ini`, for one file with a
`# tcl-lsp: disable=W141` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W141` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W127`, `W146`, `W004`
