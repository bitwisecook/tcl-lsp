# Expression operator groupings that change the result

## Question

For each unparenthesised operator mix the precedence diagnostics report (W154 for mixes with no reading convention, W155 for every other non-associative mix), does `expr` evaluate the written form the same as the grouping the analyser says it parses as, and differently from the other grouping? Each row prints the written, as-parsed and other-grouping results, in that order. The `NEG_MUL` row checks the one unary exemption: unary minus distributes over `*`. This establishes the C Tcl and Jim parse only; it does not measure the analyser, F5 word operators or BIG-IP.

## Assumptions

Each row used a fresh process on 2026-11-19 UTC with the exact script below on standard input, no command-line arguments and `LC_ALL=C`. Other inherited environment details: Not recorded. The Tcl rows are the session's reference `tclsh` builds; the Jim row is the pinned oracle built by `scripts/dev/ensure-jim-oracle.sh`. Each case is evaluated through `expr` on a braced literal, so no substitution is involved. Operand values were chosen so the two groupings differ; a row whose groupings coincide for other values is not claimed to differ everywhere. Tcl 8.4 has no `**`, so its `NEG_POW` and `POW_POW` rows report `error`.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
proc row {name written parsed other} {
    set out [list $name]
    foreach text [list $written $parsed $other] {
        set code [catch {expr $text} result]
        lappend out [expr {$code ? "error" : $result}]
    }
    puts $out
}
row BITAND_EQ {1 & 3 == 3} {1 & (3 == 3)} {(1 & 3) == 3}
row BITOR_BITAND {1 & 2 | 4} {(1 & 2) | 4} {1 & (2 | 4)}
row BITOR_BITXOR {1 | 2 ^ 3} {1 | (2 ^ 3)} {(1 | 2) ^ 3}
row BITAND_ADD {2 & 1 + 1} {2 & (1 + 1)} {(2 & 1) + 1}
row SHIFT_SUB {1 << 3 - 1} {1 << (3 - 1)} {(1 << 3) - 1}
row NOT_EQ {!2 == 1} {(!2) == 1} {!(2 == 1)}
row NOT_BITAND {!0 & 2} {(!0) & 2} {!(0 & 2)}
row CHAIN_LT {3 < 2 < 1} {(3 < 2) < 1} {3 < 2 && 2 < 1}
row CHAIN_EQ {2 == 2 == 2} {(2 == 2) == 2} {2 == 2 && 2 == 2}
row OR_AND {1 || 0 && 0} {1 || (0 && 0)} {(1 || 0) && 0}
row TERNARY_COND {1 + 0 ? 5 : 6} {(1 + 0) ? 5 : 6} {1 + (0 ? 5 : 6)}
row TERNARY_ELSE {1 ? 5 : 6 + 1} {1 ? 5 : (6 + 1)} {(1 ? 5 : 6) + 1}
row NEG_POW {-2 ** 2} {(-2) ** 2} {-(2 ** 2)}
row POW_POW {2 ** 3 ** 2} {2 ** (3 ** 2)} {(2 ** 3) ** 2}
row SUB_ADD {10 - 4 + 3} {(10 - 4) + 3} {10 - (4 + 3)}
row DIV_MUL {12 / 2 * 3} {(12 / 2) * 3} {12 / (2 * 3)}
row NEG_MOD {-7 % 3} {(-7) % 3} {-(7 % 3)}
row NEG_DIV {-7 / 2} {(-7) / 2} {-(7 / 2)}
row NEG_MUL {-7 * 2} {(-7) * 2} {-(7 * 2)}
```

## Results

### Tcl 8.4.20

- Launch argv: `tclsh8.4`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.20
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW error error error
POW_POW error error error
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty

### Tcl 8.5.19

- Launch argv: `tclsh8.5`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.19
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW 4 4 -4
POW_POW 512 512 64
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty

### Tcl 8.6.18

- Launch argv: `tclsh8.6`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 8.6.18
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW 4 4 -4
POW_POW 512 512 64
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty

### Tcl 9.0.4

- Launch argv: `tclsh9.0`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.0.4
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW 4 4 -4
POW_POW 512 512 64
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty

### Tcl 9.1.0

- Launch argv: `tclsh9.1`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 9.1.0
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW 4 4 -4
POW_POW 512 512 64
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty

### Jim 0.84-15-g2f8e155

- Launch argv: `jimsh`
- Input channel: standard input.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
VERSION 0.84-15-g2f8e155
BITAND_EQ 1 1 0
BITOR_BITAND 4 4 0
BITOR_BITXOR 1 1 0
BITAND_ADD 2 2 1
SHIFT_SUB 4 4 7
NOT_EQ 0 0 1
NOT_BITAND 0 0 1
CHAIN_LT 1 1 0
CHAIN_EQ 0 0 1
OR_AND 1 1 0
TERNARY_COND 5 5 7
TERNARY_ELSE 5 5 6
NEG_POW 4 4 -4
POW_POW 512 512 64
SUB_ADD 9 9 3
DIV_MUL 18 18 2
NEG_MOD 2 2 -1
NEG_DIV -4 -4 -3
NEG_MUL -14 -14 -14
```

- Standard error: empty
