# KCS: feature — Byte-array corruption (S110)

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, MCP, Claude skill, diagnostic, warning, shimmer

## Summary

`S110` warns when binary data is forced through character-string semantics and
then written back as bytes — a silent data-corruption bug. It is a correctness
check, unlike the `S100`/`S101`/`S102` [shimmer](../../GLOSSARY.md#shimmer)
performance warnings.

## Why it matters

A Tcl byte array and a character string are different internal
representations. When a byte array is read as a string, each byte `0x00`–`0xFF`
becomes a latin-1 character. Writing that string back to a byte sink re-encodes
it as UTF-8, so every byte `≥ 0x80` double-encodes (`c3 b3` → `c3 83 c2 b3`).
Case folding is worse: `string toupper` pushes `0xFF` to `U+0178`, which is no
longer a single byte.

This is the most common iRules payload-rewrite bug
([F5 KB K22406348](https://my.f5.com/manage/s/article/K22406348)), and the
`HTTP::payload replace` man page warns about it directly. It is silent on
Tcl 8.x and a runtime error on Tcl 9.x, so it usually ships untested with ASCII
data and corrupts in production with binary traffic.

## What fires it

- An iRules `*::payload` getter (`HTTP::payload`, `TCP::payload`, `UDP::payload`,
  `SCTP::payload`, `DIAMETER::payload`, `GTP::payload`, `MQTT::payload`) read,
  string-coerced, then written back with `<proto>::payload replace`.
- A `binary format`, `binary decode`, or `encoding convertto` result run
  through `string toupper`/`tolower`/`totitle` or `encoding convertto`.

## Examples

### iRules — the `*::payload` round-trip

This is the canonical bug. A request payload carrying a UTF-8 `ó` (the wire
bytes `c3 b3`) is read, joined into a string, and written back — so the two
bytes are decoded as the latin-1 characters `Ã` and `³`, then re-encoded as
UTF-8 (`c3 83` and `c2 b3`). The single `ó` becomes the four bytes
`c3 83 c2 b3` — the mojibake `Ã³`.

```tcl
# ✗ S110 — the payload is decoded to a string, then re-encoded on write-back.
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    append body " — appended"
    HTTP::payload replace 0 [HTTP::payload length] $body
}

# ✓ Fix — re-binarify before the sink so the write is byte-for-byte.
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    append body " — appended"
    binary scan $body c* -
    HTTP::payload replace 0 [HTTP::payload length] $body
}
```

The diagnostic points at the `HTTP::payload replace` write-back, with related
markers on the source (`HTTP::payload`) and the string coercion (`append`).
The `binary scan` forces a byte-array internal representation, so the write is
byte-for-byte; that clears the warning.

### Plain Tcl — case folding a byte array

Outside iRules the same hazard appears whenever a `binary format`,
`binary decode`, or `encoding convertto` value is run through a character-string
operation. Case folding is the clearest: it reinterprets each byte as a Unicode
code point, so `0xFF` (`ÿ`) upper-cases to `U+0178` (`Ÿ`) — no longer a single
byte.

```tcl
# ✗ S110 — string toupper mangles the high bytes of a byte array.
set packet [binary format c* {0x80 0xC3 0xFF}]
set upper  [string toupper $packet]
#                          80 c3 ff  ->  80 c3 78   on Tcl 8.x (silent),
#                                        a runtime error on Tcl 9.x

# ✓ Fix — keep binary data binary; do not apply string transforms to it.
set packet [binary format c* {0x80 0xC3 0xFF}]
set header [binary format a* "PKT"]
set frame  $header$packet            ;# byte-array concatenation stays binary
```

S110 fires at the `string toupper` use site here — the corruption is intrinsic
to the operation, so no write-back is needed.

## How to use

- **Editor**: the warning appears automatically on the `replace`/transform line,
  with related markers on the binary source and the string coercion.
- **Settings**: toggle with `tclLsp.diagnostics.S110`.
- **Suppress**: put `# noqa: S110` on the line above the flagged command, or
  `# tcl-lsp: disable=S110` at the top of the file (see
  [Suppressing diagnostics](../kcs-howto-suppress-diagnostics.md)).

## What is and is not corruption

`string range`, `string index`, `string reverse`, `string trim`, `string
trimleft`, and `string trimright` keep the byte-array representation (verified
against tclsh 8.6 and 9.0), so slicing or trimming a payload and writing it back
is byte-exact and does **not** raise S110. Only operations that build a
character string (`string map`/`replace`/`insert`/`cat`/`repeat`, `format`,
`join`, interpolation, `append`, …) or case-fold the bytes (`string
toupper`/`tolower`/`totitle`, `encoding convertto`) corrupt binary data. Which
operation does which is registry data — the
[`ByteArrayEffect`](../../design/compiler/byte-array-corruption.md) on each
command / subcommand — not a hardcoded list in the compiler.
