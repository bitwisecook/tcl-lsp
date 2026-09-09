# KCS: W109 — Why does the analyser say my file "does not look like UTF-8 text"?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

Why did the analyser report only "does not look like UTF-8 text" and stop?

## Why

The file is not UTF-8 — most often UTF-16, which is easy to produce
accidentally on Windows (PowerShell's `>` redirection and Notepad's "Unicode"
option both write UTF-16LE). Read as UTF-8, a UTF-16 file is not slightly
wrong, it is nonsense: every other byte is a NUL, so command names, braces and
strings all come apart.

Analysing it anyway would produce dozens of findings — unmatched braces that
are really NUL bytes, non-ASCII characters that are half of a UTF-16 code
unit, unresolved commands whose names are interleaved with NULs — every one
pointing at a position that corresponds to nothing in the file, and none
naming the real problem.

So the analyser **abstains**: it reports the one thing it can say truthfully
and stops.

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

Two signals:

- a **UTF-16 or UTF-32 byte-order mark** at the start of the file; or
- **NUL bytes at a density no real UTF-8 source has** — at least eight, and at
  least a quarter of the file. Mostly-ASCII UTF-16 is about half NUL, so the
  bar sits far above real text.

A valid UTF-8 file that merely contains a few NUL bytes is analysed normally.

The check names the *family* it matched and stops there. Distinguishing
UTF-16LE from a corrupt UTF-8 file is not provable from the bytes.

If your editor recognises the UTF-16 and decodes it before sending the buffer
to the language server, nothing fires — the text really is fine. `tcl diag` on
the same path reads the bytes itself and still reports W109.

## How to suppress

Set `tclLsp.diagnostics.W109` to `false`. The file-level
`# tcl-lsp: disable=W109` directive cannot help: a file that is not UTF-8
cannot carry a UTF-8 comment the analyser will read. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

Disabling it does not re-enable the rest of the analysis: the abstention
follows from the file not being text, not from the diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W107`, `W108`, `W118`
