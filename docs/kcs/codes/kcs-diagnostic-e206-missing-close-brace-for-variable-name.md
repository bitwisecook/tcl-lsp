# KCS: E206 — Why does the analyser say a `${name}` reference has no closing brace?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see an error saying a `${…}` variable reference is missing its
closing brace?

## Why

`${name}` is the braced form of a variable reference. It exists so a
variable name can contain characters that a bare `$name` cannot hold —
spaces, `-`, `(`, and so on. The name runs to the first `}`, so without
that brace Tcl cannot tell where the name ends and raises "missing
close-brace for variable name". The analyser reports the same fault.

## Symptoms

- A red squiggle where the reference runs out, with the message
  "missing close-brace for variable name".
- Everything after the `${` is swallowed into the name, so later commands
  on the same line are often mis-parsed too.

## Example that triggers it

```tcl
set user(name) "alice"
puts "Hello ${user(name)"
```

The closing `}` is missing after `user(name)`, so the analyser reports
**`E206`** on the unterminated reference.

## Fix

```tcl
set user(name) "alice"
puts "Hello ${user(name)}"
```

Add the missing `}`. If you did not want the braced form at all, a plain
`$user(name)` reads the same array element.

## How to suppress

`E206` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E206` directive at the top of the file, or for a
whole project with `disabled = E206` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E203`, `E204`, `E205`
