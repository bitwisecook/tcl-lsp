# KCS: E205 — Why does the analyser flag characters after a closing quote?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why do I see an error saying there are extra characters after a `"`?

## Why

A double-quoted word ends at its closing `"`. Tcl expects whitespace, a
newline, a `;`, or a `]` immediately after it. Anything else is a syntax
error — `tclsh` refuses to run the script with "extra characters after
close-quote". The analyser reports the same fault at the same place so
you catch it while editing.

The usual cause is a quote that closes earlier than intended, so the rest
of the intended string spills outside the quoted word.

The Rust runtime raises the same error when it evaluates such a script, so
the diagnostic and the run agree.

## Symptoms

- A red squiggle at the character following the `"`, with the message
  "extra characters after close-quote".
- The word is often the argument of a command that then looks like it has
  the wrong number of arguments, so a second diagnostic may follow.

## Example that triggers it

```tcl
puts "hello"world
```

The analyser reports **`E205`** at the `w` of `world` — the first
character after the closing quote of `"hello"`.

## Fix

```tcl
puts "hello world"
```

Either extend the quoted word to cover the whole string, or separate the
two words:

```tcl
puts "hello" world
```

A backslash-newline directly after the `"` is fine — that is a line
continuation, not extra text.

## How to suppress

`E205` is an internal parse error: it has no per-code entry in the
generated editor settings list. Silence it for one file with a
`# tcl-lsp: disable=E205` directive at the top of the file, or for a
whole project with `disabled = E205` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `E200`, `E202`, `E204`, `E206`
