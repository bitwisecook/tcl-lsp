# KCS: W311 — Why does the analyser warn about `-encoding binary` with a `-translation`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser flag a channel configured with `-encoding binary`
and a `-translation` that is not `binary`?

## Why

The two settings contradict each other. `-encoding binary` says "these
are raw bytes, do not re-encode them"; a `-translation` other than
`binary` still rewrites end-of-line sequences on the way through. Every
CR or LF octet in the data is at risk, so a binary payload can come out
of the channel different from how it went in. Where the channel carries
protocol data, the disagreement between what the channel promises and
what it does is also an encoding-differential attack surface.

## Symptoms

- A yellow squiggle on the `-translation` value (or, failing that, the
  `-encoding` value), with the message "Channel configured with -encoding
  binary and a non-binary -translation. Binary encoding implies no
  translation; the conflicting -translation may silently corrupt data or
  enable encoding-differential attacks."

## Example that triggers it

```tcl
set ch [open payload.dat rb]
fconfigure $ch -encoding binary -translation lf
set data [read $ch]
close $ch
```

The analyser reports **`W311`** on `lf`.

## Fix

```tcl
set ch [open payload.dat rb]
fconfigure $ch -translation binary
set data [read $ch]
close $ch
```

`-translation binary` sets both halves consistently: raw bytes, no
end-of-line rewriting. If you genuinely want text handling, drop
`-encoding binary` and name the character encoding the data is really in.

`chan configure` is checked exactly the same way as `fconfigure`.

## How to suppress

`W311` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=W311`
directive at the top of the file, or for a whole project with
`disabled = W311` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `S110`, `W126`, `W310`
