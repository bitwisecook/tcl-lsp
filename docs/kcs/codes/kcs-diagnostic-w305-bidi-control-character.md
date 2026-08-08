# KCS: W305 — Why is there an error about a bidirectional formatting control?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

The analyser reports an error about a "bidirectional formatting control" on a line that looks completely normal — what is it seeing that I am not?

## Why

That is exactly the point: it is seeing something you are not.

Unicode has a small set of invisible characters that change the *order* in
which surrounding text is drawn — right-to-left override, the embedding pair,
the isolate pair. They do not change the order in which the text is stored,
parsed, or executed. So a file containing one renders to a reviewer in one
order and runs in another.

This is the **Trojan Source** attack (CVE-2021-42574). Its practical form is a
change that passes review because the reviewer's editor showed them something
different from what the interpreter will run — a statement that appears to be
inside a comment, an early `return` that appears to come after the check it
skips, a logging call that appears to redact a value it actually logs.

For iRules the exposure is direct: an iRule is a security control, reviewed by
reading it, and deployed to sit in front of production traffic. An override
that makes a `drop` render as a `log` is a complete bypass that no amount of
careful reading will catch.

It is reported at **error** severity rather than as a style warning because
there is no version of "the file lies to its reviewer" that is a matter of
taste, and because these characters have essentially no legitimate use in Tcl
source.

## Symptoms

- A red squiggle on a character you cannot see, often at a position where the
  line looks like it has an odd gap or where text after it reads backwards.
- The message names the character: "Bidirectional formatting control U+202E
  RIGHT-TO-LEFT OVERRIDE".
- Selecting text across the region behaves strangely — the caret jumps.

## Example that triggers it

A `U+202E` RIGHT-TO-LEFT OVERRIDE inside a comment makes the rest of the line
render reversed, so a reviewer sees a harmless sentence where a live statement
follows:

```tcl
when HTTP_REQUEST priority 500 {
    # ‮ }  ;# always allow — reviewed 2026-01
    HTTP::respond 200
}
```

The analyser reports **`W305`** on the invisible character itself, immediately
after the `#`.

## Fix

Delete the character. There is no automatic quick-fix, deliberately: removing
it changes what the file renders as, and only the author knows which of the
two readings was intended.

```tcl
when HTTP_REQUEST priority 500 {
    # always allow — reviewed 2026-01
    HTTP::respond 200
}
```

If the character genuinely belongs in your *data* — for example, you are
building a string for display in a bidirectional UI — write it as an escape so
it is visible in the source:

```tcl
set marker "‮"
```

## What it detects, and what it does not

W305 covers exactly the nine bidirectional **formatting controls** — the
embeddings, overrides and isolates:

| Codepoint | Name |
|---|---|
| `U+202A` | LEFT-TO-RIGHT EMBEDDING |
| `U+202B` | RIGHT-TO-LEFT EMBEDDING |
| `U+202C` | POP DIRECTIONAL FORMATTING |
| `U+202D` | LEFT-TO-RIGHT OVERRIDE |
| `U+202E` | RIGHT-TO-LEFT OVERRIDE |
| `U+2066` | LEFT-TO-RIGHT ISOLATE |
| `U+2067` | RIGHT-TO-LEFT ISOLATE |
| `U+2068` | FIRST STRONG ISOLATE |
| `U+2069` | POP DIRECTIONAL ISOLATE |

It scans the **whole file** — comments, command names, string bodies, the
whitespace between words — because a control reorders the text *around*
itself, so restricting the scan to argument tokens would miss the classic
comment-based attack outright.

Three things are deliberately **not** flagged by W305:

- **Right-to-left content.** Arabic, Hebrew, Thaana and N'Ko text in a string
  or a comment is ordinary content, not an attack. Flagging it would be a
  false positive that makes the real signal useless.
- **Directional marks** — `U+200E` LRM, `U+200F` RLM, `U+061C` ALM. These
  nudge the resolved direction of a single neutral character and are routinely
  written by hand in legitimate bidirectional text. They cannot reorder a run.
- **Zero-width and other invisible characters** (`U+200B`–`U+200D`, `U+2060`,
  a mid-file `U+FEFF`). These can hide or split an identifier, which is a real
  hazard — but it is the homoglyph hazard, so they stay with
  [`W108`](kcs-diagnostic-w108-non-ascii-characters.md).

A character flagged by W305 is never also flagged by W108; the two sets do not
overlap.

## How to suppress

Add `# noqa: W305` at the end of the offending line, set
`tclLsp.diagnostics.W305` to `false`, or add `# tcl-lsp: disable=W305` at the
top of the file.

Think hard before you do. Unlike a style lint, a suppressed W305 leaves a file
in the tree whose rendered form does not match its executed form — and the
suppression comment itself will render in whatever order the control dictates.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W108`, `W107`, `IRULE3102`
