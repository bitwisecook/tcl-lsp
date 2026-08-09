# KCS: W107 — Why does the analyser say my file is not valid UTF-8?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lexing

## Profiles

default

## Question

The analyser says my file "is not valid UTF-8" and mentions U+FFFD — what does that mean, and why does it matter if everything still seems to work?

## Why

Tcl source files are read as UTF-8. When some of the bytes in a file are not
valid UTF-8, the toolchain cannot recover what they were meant to say, so it
substitutes the Unicode replacement character `U+FFFD` (�) for each ill-formed
sequence and carries on. That keeps the file analysable, but it means **the
text being analysed is not the file on disk**: content differs, and every
position after the first substitution is derived from characters that were
never there. Any other diagnostic in the affected region may point at the
wrong place.

It also matters at run time. **Tcl 9 refuses to read such a file at all** —
`source` fails with `invalid or incomplete multibyte or wide character`. Tcl
8.6 passes the bad bytes through instead, so the same file behaves differently
on the two interpreters. Whatever the file was supposed to contain, it is not
what either interpreter will see.

The most common causes are a file saved in a legacy 8-bit encoding
(ISO-8859-1, Windows-1252) but named as UTF-8, a truncated download or copy,
and text spliced together at a byte offset that fell in the middle of a
character.

## Symptoms

- One warning at the first `U+FFFD` in the file: "Source is not valid UTF-8:
  *&lt;kind&gt;* at byte offset *N*".
- Replacement characters (�) visible in the editor where accented or
  non-Latin text should be.
- W108 (non-ASCII character) firing on those same positions — it is describing
  the *replacement*, not the corruption.

## Example that triggers it

A file saved as ISO-8859-1 rather than UTF-8, so `é` is the single byte
`0xE9` instead of the two bytes `0xC3 0xA9`:

```tcl
# saved as ISO-8859-1 — the byte after "caf" is 0xE9
set drink "café"
puts $drink
```

The analyser reports **`W107`** once, at the first replacement character:

```
W107  1:18  Source is not valid UTF-8: truncated multi-byte sequence at byte
            offset 47 (1 ill-formed sequence in total, each replaced with
            U+FFFD). ...
```

## Fix

Re-save the file as UTF-8. In VS Code: **Change File Encoding → Reopen with
Encoding → Western (ISO 8859-1)** to see the intended text, then **Save with
Encoding → UTF-8**. From a shell:

```sh
iconv -f ISO-8859-1 -t UTF-8 drink.tcl > drink.utf8.tcl && mv drink.utf8.tcl drink.tcl
```

```tcl
# now genuinely UTF-8
set drink "café"
puts $drink
```

If the file was truncated rather than mis-encoded, re-fetch or re-export it —
there is nothing in the file to repair.

## What it detects, and what it does not

The check reports the **first** ill-formed sequence and names its class:
truncated multi-byte sequence, overlong encoding, lone surrogate (CESU-8 /
WTF-8), out-of-range lead byte, or stray continuation byte. It counts all of
them, but reports once — a mis-decoded file has one problem, not one per byte.

It cannot detect a legacy 8-bit file whose high bytes happen to *form* valid
UTF-8 sequences; that file is indistinguishable from a UTF-8 file containing
different characters, and the analyser abstains rather than guess.

There is one difference between reading a file and editing it. When the
toolchain reads the file itself (`tcl diag`, or an editor opening an unchanged
file) it has matching bytes, so it names the exact offset and class. When an
unsaved editor buffer no longer matches the file on disk, the original bytes
are not available. The analyser then stays silent rather than guess from a
`U+FFFD` character, because that character can be legitimate Tcl text.

## How to suppress

Set `tclLsp.diagnostics.W107` to `false`, or add
`# tcl-lsp: disable=W107` at the top of the file. Inline `# noqa` will not
help — the finding is about the file, not a line.

Suppressing it is rarely the right answer: the diagnostic is telling you that
every *other* diagnostic in the file may be pointing at the wrong place.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lexing](../../GLOSSARY.md#lexing)
- Related codes: `W109`, `W108`, `W118`
