# KCS: How do I tell tcl-lsp which encoding my Tcl reads scripts in?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, JetBrains, `tcl` CLI

## Question

My scripts hold non-ASCII text, such as `set name "Éric"`. The editor
never shows the value of `[string length $name]` or `[string range ...]`
over that text for my Tcl 8.6 target, and `tcl opt` leaves those calls
alone. For a Tcl 9 target it does. Why, and how do I fix it?

## Before you start

- Know how your Tcl reads the script. Tcl 9 reads every script as UTF-8.
  Tcl 8.4, 8.5 and 8.6 read it in the *system encoding*, which follows the
  locale: `iso8859-1` under `LC_ALL=C`, `utf-8` under a UTF-8 locale. A
  `source -encoding NAME` call overrides both.
- `tclsh` tells you its system encoding: `puts [encoding system]`.

## Answer

The same bytes are different text under different encodings. The bytes
`C3 89` are one character (`É`) to a UTF-8 reader and two (`Ã` and
U+0089) to an ISO 8859-1 reader. So `string range "Éabc" 1 2` is `ab` for
one and `\x89a` for the other. tcl-lsp cannot know an 8.x target's system
encoding, so by default it computes nothing from non-ASCII text for 8.x.

Declare the encoding, and those values become exact for every release:

- **VS Code:** set `tclLsp.sourceEncoding` to `utf-8`, `iso8859-1` or
  `ascii`. Also set the editor's `files.encoding` to the same encoding, so
  an open file is read the way Tcl reads it.
- **JetBrains:** *Settings → Tools → Tcl LSP → Source encoding*, and set
  the IDE's file encoding to match.
- **`tcl` CLI:** pass `--source-encoding iso8859-1` (or `utf-8`, `ascii`).
  Files are decoded with it, and `tcl opt` and `tcl fmt` write their result
  back in the same encoding.

The default, `auto`, keeps today's behaviour: UTF-8 decoding, with the
values computed for Tcl 9 only.

## How to tell it worked

With `iso8859-1` declared and a Tcl 8.6 target, `tcl opt` rewrites
`puts [string range "Éabc" 1 2]` (saved as UTF-8 bytes) to the bytes
`puts \x89a`. That is what `LC_ALL=C tclsh8.6` prints for the file. With
`auto` the call is left alone.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Value evaluation: `SOURCE_ENCODING`](../design/compiler/value-evaluation.md)
