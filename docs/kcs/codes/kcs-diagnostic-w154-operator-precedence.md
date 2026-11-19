# KCS: W154 — Operator precedence groups differently from how it reads

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default, dialect:irule

## Question

Why is `$c & 0xff == 0x81` flagged?

## Why

In a Tcl expression, comparisons bind tighter than `&`, `^` and `|`, `+` and
`-` bind tighter than `<<` and `>>`, and `!` binds tighter than every
comparison. Without parentheses these read one way and parse another:

| Written | Parses as |
| --- | --- |
| `$c & 0xff == 0x81` | `$c & (0xff == 0x81)` — always `0` here |
| `1 << $n - 1` | `1 << ($n - 1)` |
| `!$x == 5` | `(!$x) == 5` |

The iRules word operators (`contains`, `starts_with`, `equals`, `not`, …) take
part with their own precedence.

## Symptoms

- A warning on the outer operator: "'==' binds tighter than '&', so this parses
  as `$c & (0xff == 0x81)`; if you meant `($c & 0xff) == 0x81`, add
  parentheses."
- The condition never (or always) holds at run time.

## Example that triggers it

```tcl
binary scan $data c type
if {$type & 0xff == 0x81} { puts hit }
```

## Fix

Two quick fixes are offered. **Regroup** writes the likely intent:

```tcl
if {($type & 0xff) == 0x81} { puts hit }
```

**Parenthesise as parsed** keeps the current behaviour and makes it explicit.
Hover an operator to see the expression's parse as a tree.

## How to suppress

Add the parentheses either way, use `# tcl-lsp: disable=W154`, or disable
W154 in editor diagnostic settings.

## Related

- [KCS codes index](README.md)
- [IRULE5008 — signed scan value compared with an unsigned constant](kcs-diagnostic-irule5008-signed-scan-comparison.md)
- [Execution evidence: `&` versus `==`](../../../evidence/tcl/binary-scan-signed-byte-comparison.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
