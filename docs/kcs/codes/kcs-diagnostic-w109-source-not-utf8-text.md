# KCS: W109 — Why does the analyser say my file "does not look like UTF-8 text"?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

I opened an iRule and got a single warning saying the file does not look like UTF-8 text, and no other diagnostics at all — why did the analyser stop?

## Why

The file is not UTF-8 — most often UTF-16, which is easy to produce
accidentally on Windows (PowerShell's `>` redirection and Notepad's "Unicode"
option both write UTF-16LE). Read as UTF-8, a UTF-16 file is not slightly
wrong, it is nonsense: every other byte is a NUL, so command names, braces and
strings all come apart.

Analysing it anyway is worse than useless. A three-line UTF-16 iRule used to
produce **87 diagnostics** — unmatched braces that are really NUL bytes,
non-ASCII characters that are half of a UTF-16 code unit, unresolved commands
whose names are interleaved with NULs — every one of them pointing at a
position that does not correspond to anything in the file, and none of them
mentioning the actual problem.

So the analyser **abstains**: it reports the one thing it can say truthfully
and stops. That is not the analyser giving up quietly — it is the analyser
declining to make up 87 answers.

## Symptoms

- Exactly one diagnostic on the file, at line 1, column 1.
- No other diagnostics at all, including ones you expect to fire.
- The file may render as gibberish, or with a `ÿþ` prefix, in a UTF-8 editor.

## Example that triggers it

Any iRule saved as UTF-16 — for instance from PowerShell:

```powershell
# writes UTF-16LE by default
"when HTTP_REQUEST { log local0. `"hit`" }" > rule.irule
```

The analyser reports **`W109`** once, at the start of the file:

```
W109  1:1  Source does not look like UTF-8 text — found a UTF-16 byte-order
           mark. Re-save the file as UTF-8; analysis of the rest of this file
           is skipped rather than reporting findings derived from mis-decoded
           bytes.
```

## Fix

Re-save the file as UTF-8. From PowerShell:

```powershell
Get-Content rule.irule | Set-Content -Encoding utf8 rule.irule
```

From a shell:

```sh
iconv -f UTF-16 -t UTF-8 rule.irule > rule.utf8.irule && mv rule.utf8.irule rule.irule
```

In VS Code: **Save with Encoding → UTF-8**. The diagnostics you expected
appear on the next analysis.

## What it detects, and what it does not

Two signals, both deliberately conservative:

- a **UTF-16 or UTF-32 byte-order mark** at the start of the file; or
- **NUL bytes at a density no real UTF-8 source has** — at least eight of
  them, and at least a quarter of the file. Mostly-ASCII UTF-16 is about half
  NUL, so the bar sits far above anything real text reaches.

A valid UTF-8 file that merely *contains* a few NUL bytes is real (if odd)
text and is **not** flagged; it is analysed normally.

The check names the *family* it matched and stops there. It does not claim to
know which encoding the file actually is — distinguishing UTF-16LE from, say,
a corrupt UTF-8 file is not provable from the bytes, and a confident wrong
guess in a review tool is worse than saying nothing.

One case it deliberately cannot see: if your **editor** recognises the UTF-16
and decodes it correctly before sending the buffer to the language server,
then the text really is fine and nothing fires. That is the right answer — the
file analyses correctly as text. Running `tcl diag` on the same path, which
reads the bytes itself, still reports W109.

## How to suppress

Set `tclLsp.diagnostics.W109` to `false`, or add
`# tcl-lsp: disable=W109` at the top of the file — though a file that is not
UTF-8 cannot carry a UTF-8 comment the analyser will read, so in practice the
setting is the only route.

Disabling it does not re-enable the rest of the analysis: the abstention
follows from the file not being text, not from the diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W107`, `W108`, `W118`
