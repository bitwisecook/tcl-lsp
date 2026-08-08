# KCS: `string` subcommands used to corrupt binary values under the WASM runtime

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI

## Question

Why did `string index`/`range`/`replace`/`toupper` (and the rest of the
portable `string` ensemble) corrupt a value built by `binary format` or read
from a binary channel when running under `runtime/rust` (the Rust WASM
runtime), when the same script gave the correct answer under the bytecode VM
or `tclsh`?

## Symptoms

- `string index`/`string range` on a value containing a byte with the high bit
  set (anything not valid UTF-8 on its own, e.g. a lone `0xFF`) returned the
  3-byte UTF-8 encoding of U+FFFD (`ef bf bd`, the Unicode replacement
  character) instead of the original byte.
- The value's own `binary encode hex`/`string length` still reported the
  correct byte count and content — only operations that went through the
  portable `string` command surface lost data.
- `RUST_ISSUE_168` tracked this as a `cmd_string.rs`-local issue in four
  named functions; those functions are actually dead code (superseded by
  `tcl_cmd_core::string::dispatch_canon`) and were not where the corruption
  happened.

## Answer

The single fix point was `ValueOps::as_str` in
`runtime/rust/src/value_ops.rs`, the adapter every shared `tcl-cmd-core`
`string`/`dict` command uses to read a string operand. It called
`String::from_utf8_lossy`, which silently replaces any byte that is not part
of a valid UTF-8 sequence with U+FFFD — destructive for a value that is
conceptually a byte sequence rather than a genuine text string (`binary
format`'s output, a binary channel read, `binary decode`).

The fix is a matched pair of free functions in the same file:

- `bytes_to_str` (used by `as_str`) tries `str::from_utf8` first — genuine
  Tcl text is always valid UTF-8, so this is lossless and is the common case.
  Only when the bytes are *not* valid UTF-8 does it fall back: each raw byte
  `b` is escaped to the scalar `BYTE_ESCAPE_BASE + b`, a 256-codepoint range
  in Unicode Plane 16's Private Use Area — never produced by decoding valid
  UTF-8, and never realistically present in real script text, so it cannot
  collide with genuine characters.
- `str_to_bytes` (used by `new_str`/`new_string`, the construction side) is
  the inverse: a character in the escape range decodes back to its one raw
  byte; every other character — including genuine Latin-1-supplement text
  such as `é` — is encoded as ordinary UTF-8.

This is why the escape range is Plane 16 Private Use Area rather than Tcl's
own byte-array string-rep convention (raw byte `b` ↦ literal `U+00b`, which
the bytecode VM's `tcl-vm/src/cmd_binary.rs` already uses safely): the VM
never decodes arbitrary already-stored bytes back through this seam, so a
literal `U+00E9` is always genuinely `binary`-format-shimmered. `runtime/rust`
does decode existing text through this exact seam (`string index héllo 1`
must still return `é`), and `U+00E9` is indistinguishable from an escaped raw
byte `0xE9` under the literal convention — an earlier version of this fix
used the literal mapping and broke exactly that case
(`cmd_string::tests::utf8_char_indexing`) before switching to the
collision-free escape range.

### Known residual limitation

Tracked as
[issue #1347](https://github.com/bitwisecook/tcl-lsp/issues/1347).

`runtime/rust`'s `TclObj` has one byte buffer serving both roles real Tcl
keeps separate: the object's string rep (what `puts` writes) and a
byte-array's raw payload (what `binary encode`/`scan` read). A `string`
subcommand result that is genuine, non-ASCII Latin-1-supplement text (e.g.
`"CAFÉ"`, produced by `string toupper` on `"café"`) round-trips correctly
through `bytes_to_str`/`str_to_bytes`, matching what `tcl_cmd_core` computed —
but a case-changing subcommand (`toupper`/`tolower`/`totitle`) applied to a
value that *is* raw binary content is a no-op on the escaped bytes, rather
than the byte-for-byte-different Latin-1 case fold C Tcl's byte-array shimmer
performs. Leaving binary content untouched by a case-conversion command was
judged the safer trade-off over silently reintroducing a different kind of
corruption.

`binary encode hex [string toupper [binary format H* 41ff42]]` gives:

| Backend | Result |
|---|---|
| `tclsh8.6` | `417842` |
| `tclsh9.0` | error: `expected code point values below 0xff but value at byte offset 1 was 0x178` |
| the bytecode VM | `417842` — matches 8.6 |
| `runtime/rust` | `41ff42` — unchanged |

Note that 0xFF upper-cases to U+0178, which Tcl 9.0 refuses to encode as a
byte, so the correct answer is version-dependent; the bytecode VM already
matches 8.6.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- `docs/design/runtime/c-extension-abi.md` — why `TclObj.bytes` is one buffer
- [issue #1347](https://github.com/bitwisecook/tcl-lsp/issues/1347) — the
  residual case-fold limitation above
