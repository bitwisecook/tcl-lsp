# KCS: E101 — Why does the analyser flag a missing opening brace after `switch`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see a red squiggle when `switch` cases are listed without an enclosing brace?

## Why

Without braces around the switch body, Tcl treats each case as a separate argument. This leads to argument-count errors or silently wrong pattern matching at runtime.

## Symptoms

- A red squiggle appears just after the `switch` subject word, with the
  message "Missing '{' after switch — body cases follow without braces".

## Example that triggers it

```tcl
switch $x
  1 {puts one}
```

The analyser reports **`E101`** immediately after `switch $x`, where the
opening brace should have been.

## Fix

```tcl
switch $x {
  1 {puts one}
}
```

Wrap the entire set of cases in braces so the parser recognises them as a single switch body.

## How to suppress

`E101` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E101` directive at the top of the file, or for a
whole project with `disabled = E101` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E103`, `E200`
