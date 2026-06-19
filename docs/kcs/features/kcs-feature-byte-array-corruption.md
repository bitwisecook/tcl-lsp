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

## Example

```tcl
# ✗ S110 — the payload is decoded to a string, then re-encoded on write-back.
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    set body "$body INJECTED"
    HTTP::payload replace 0 100 $body
}

# ✓ Fix — re-binarify before the sink so the write is byte-for-byte.
when HTTP_REQUEST_DATA {
    set body [HTTP::payload]
    set body "$body INJECTED"
    binary scan $body c* -
    HTTP::payload replace 0 100 $body
}
```

The diagnostic points at the write-back, with related markers on the source
(`HTTP::payload`) and the string coercion. Adding the `binary scan` re-binarify
clears it.

## How to use

- **Editor**: the warning appears automatically on the `replace`/transform line.
- **Settings**: toggle with `tclLsp.diagnostics.S110`.
- **Suppress**: `;# tcl-lsp: disable-line S110` on the flagged line (see
  [Suppressing diagnostics](../kcs-howto-suppress-diagnostics.md)).

## File-path anchors

- `compiler/shimmer.py` — `_find_byte_array_corruption`
- `compiler/registry/runtime.py` — `byte_array_payload_commands`
- `dialects/f5/irules/*__payload.py` — `byte_array_payload=True`

## Test anchors

- `tests/test_shimmer.py::TestByteArrayCorruption`
- `tests/test_fp_sh.py::test_FP_SH_09_*`, `::test_FP_SH_10_*`
- `tests/lsp_e2e/test_irules_e2e.py::TestIrulesByteArrayCorruption`
- `docs/design/compiler/FP.md` — FP-SH-09, FP-SH-10
