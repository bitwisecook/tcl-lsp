# KCS: IRULE5008 — Signed scan value compared with an unsigned constant

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, ssa

## Profiles

dialect:irule

## Question

Why does `if {$byte == 0x81}` never match after `binary scan … c byte`?

## Why

iRules run Tcl 8.4, which has no unsigned `binary scan` fields (the `cu`
suffix arrived in 8.5) and no unsigned `scan` conversion. The integer fields
store signed values:

| Field | Range |
| --- | --- |
| `binary scan … c` | `-128..127` |
| `binary scan … s` / `S` | `-32768..32767` |
| `binary scan … i` / `I` | `-2147483648..2147483647` |
| `scan … %d %i %o %x %u` | `-2147483648..2147483647` (`scan ffffffff %x v` stores `-1`) |

A byte `0x81` is stored as `-127`, so comparing it with `0x81` (129) is always
false, `!= 0x81` always true, and `> 127` never holds. The check follows the
scanned variable through the event body until it is reassigned.

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

Mask the value to unsigned before comparing:

```tcl
if {($type & 0xff) == 0x81} { log local0. "websocket text frame" }
```

or compare with the signed value, `if {$type == -127}`. Mind the parentheses:
`$type & 0xff == 0x81` parses as `$type & (0xff == 0x81)` (W154).

## How to suppress

Use `# tcl-lsp: disable=IRULE5008`, or disable IRULE5008 in editor diagnostic
settings.

## Related

- [KCS codes index](README.md)
- [W154 — operator precedence groups differently from how it reads](kcs-diagnostic-w154-operator-precedence.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
