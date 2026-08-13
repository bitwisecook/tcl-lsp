# KCS: E204 — Why does the analyser flag characters after a closing brace?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see an error saying there are extra characters after a `}`?

## Why

A braced word ends at its matching `}`. Tcl expects whitespace, a
newline, a `;`, or a `]` immediately after that brace. Anything else is a
syntax error — `tclsh` itself refuses to run the script with
"extra characters after close-brace". The analyser reports the same fault
at the same place so you see it while typing rather than at run time.

## Symptoms

- A red squiggle at the character following the `}`, with the message
  "extra characters after close-brace".
- The command containing the brace group is often mis-parsed as well, so
  a second diagnostic may appear nearby.

## Example that triggers it

```tcl
set names {alice bob}extra
```

The analyser reports **`E204`** at the `e` of `extra` — the first
character after the closing brace of `{alice bob}`.

## Fix

```tcl
set names {alice bob}
```

Separate the two words, or move the trailing text inside the braces if it
was meant to be part of the value:

```tcl
set names {alice bob extra}
```

A backslash-newline directly after the `}` is fine — that is a line
continuation, not extra text.

## When it does not fire

- **iRules `}{` word boundaries.** Under the iRules dialect an
  immediately following `{` opens the next word (the `when EVENT {…}{…}`
  shape), so it is treated as a separator rather than an error.

## How to suppress

`E204` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E204` directive at the top of the file, or for a
whole project with `disabled = E204` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E203`, `E205`, `E206`
