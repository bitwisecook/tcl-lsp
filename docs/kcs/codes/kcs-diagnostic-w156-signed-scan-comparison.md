# KCS: W156 — Signed scan value compared with an unsigned constant

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, ssa

## Profiles

default, dialect:irule

## Question

Why does `if {$byte == 0x81}` never match after `binary scan … c byte`?

## Why

`binary scan` integer fields store signed values on every release unless the
field carries the `u` suffix, which Tcl 8.4 — and so iRules — does not have.
Some releases also truncate `scan` integer conversions to signed 32 bits:

| Read | Stored as | Where |
| --- | --- | --- |
| `binary scan … c` | `-128..127` | every release and Jim |
| `binary scan … s`, `S`, `t` | `-32768..32767` | every release and Jim |
| `binary scan … i`, `I`, `n` | `-2147483648..2147483647` | every release and Jim |
| `scan … %d %i %o %x` (no `l`) | `-2147483648..2147483647` | Tcl 8.4, 9.0, 9.1 |
| `scan … %u` (no `l`) | `-2147483648..2147483647` | Tcl 8.4 |

A byte `0x81` is stored as `-127`, so comparing it with `0x81` (129) is always
false, `!= 0x81` always true, and `> 127` never holds. Tcl 8.5, 8.6 and Jim
store `scan %x` results at full width, so W156 does not report them there.

The check follows the scanned variable until it is reassigned. In procedure
bodies, where the analyser cannot prove `binary` still names Tcl's command,
it follows the variable within one straight run of code to an `if`, `while`
or `for` condition.

## Symptoms

- A warning on the variable: "$type holds a signed 8-bit value from
  `binary scan … c` (-128..127), so comparing it with 0x81 is always false."

## Example that triggers it

```tcl
when CLIENT_DATA {
  binary scan [TCP::payload] c type
  if {$type == 0x81} { log local0. "websocket text frame" }
}
```

## Fix

The message offers what the dialect supports:

- Read it unsigned (Tcl 8.5+ and Jim): `binary scan $data cu type`; for
  `scan`, add the `l` size: `scan $text %lx id`.
- Mask it: `if {($type & 0xff) == 0x81}`.
- Compare with the signed value: `if {$type == -127}`.

Mind the parentheses: `$type & 0xff == 0x81` parses as
`$type & (0xff == 0x81)` (W154).

The same signed value also goes wrong in `format %02x`, dotted-quad `format`,
`($hi << 8) | $lo`, and list or string indices; mask it before any of them.

## How to suppress

Use `# tcl-lsp: disable=W156`, or disable W156 in editor diagnostic settings.

## Related

- [KCS codes index](README.md)
- [W154 — operator mix groups differently from how it reads](kcs-diagnostic-w154-operator-precedence.md)
- [Execution evidence: signed bytes, masking and precedence](../../../evidence/tcl/binary-scan-signed-byte-comparison.md)
- [Execution evidence: scan and binary scan signedness by release](../../../evidence/tcl/scan-integer-signedness.md)
- [Execution evidence: downstream consumers of a signed value](../../../evidence/tcl/signed-scan-downstream-consumers.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
