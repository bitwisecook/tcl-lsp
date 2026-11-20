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
| `scan … %lx` | `-2147483648..2147483647` | iRules (TMM on BIG-IP 21.1; stock 8.4 reads it at full width) |

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
  `scan`, add the `l` size: `scan $text %lx id`. Neither works in an iRule:
  TMM loads `cu` but fails at run time (W200), and truncates `%lx` too.
- Mask it: `if {($type & 0xff) == 0x81}`.
- Compare with the signed value: `if {$type == -127}`.

Mind the parentheses: `$type & 0xff == 0x81` parses as
`$type & (0xff == 0x81)` (W154).

## Where else it is reported

The same signed value goes wrong wherever an unsigned value is expected, so
W156 also reports it, unmasked, as:

| Use | What happens |
| --- | --- |
| `($hi << 8) \| $lo`, `$hi * 256 + $lo`, `^` | a negative part sign-extends: `0xc0 0xff` gives -1 or -16385, not 49407 |
| `format %x`, `%X`, `%o`, `%b`, `%u`, `%c` | `ffffffffffffff81` (`ffffff81` on Tcl 9), a huge or negative number, another character |
| `format %d.%d.%d.%d`, `"$a.$b.$c.$d"` | an address such as `192.-64.-1.1`, which `IP::addr` rejects |
| `lindex`, `string index`, `string range`, `lrange`, `lreplace` indices | a negative index selects nothing |
| `string repeat` count | a negative count returns the empty string |
| `TCP::collect`, `TCP::release`, `TCP::payload`, `UDP::payload`, `HTTP::collect`, `HTTP::payload`, `SSL::collect`, `SSL::payload`, `SSL::release` lengths and offsets | -1 means "everything", other negatives raise at run time, and `SSL::collect -1` never raises its data event |
| `node` and `pool … member` ports | refused |

The F5 rows were measured on BIG-IP 21.1; the registry records each as the
argument's integer domain (`arg_domains`). Mask the value first, for
example `[expr {$len & 0xffff}]`.

## How to suppress

Use `# tcl-lsp: disable=W156`, or disable W156 in editor diagnostic settings.

## Related

- [KCS codes index](README.md)
- [W154 — operator mix groups differently from how it reads](kcs-diagnostic-w154-operator-precedence.md)
- [Execution evidence: signed bytes, masking and precedence](../../../evidence/tcl/binary-scan-signed-byte-comparison.md)
- [Execution evidence: scan and binary scan signedness by release](../../../evidence/tcl/scan-integer-signedness.md)
- [Execution evidence: downstream consumers of a signed value](../../../evidence/tcl/signed-scan-downstream-consumers.md)
- [BIG-IP evidence: TMM `binary scan`, `scan` and `format`](../../../evidence/tcl/bigip-tmm-signed-scan-and-format.md)
- [Execution evidence: `string match` bracket sets](../../../evidence/tcl/string-match-bracket-set-edges.md)
- [BIG-IP measurements: signed integers reaching F5 commands](../../design/f5/bigip-irule-parser-measurements.md#signed-integers-reaching-f5-commands)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
