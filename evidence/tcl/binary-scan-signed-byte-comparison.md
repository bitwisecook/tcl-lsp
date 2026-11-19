# Signed `binary scan` bytes, unsigned masking and `&` versus `==` precedence

## Question

After `binary scan \x81 c type`, does `type` hold `-127`; does `($type & 0xff) == 0x81` hold; does `$type == -127` hold; does `$type == 0x81` fail; and does the unparenthesised `$type & 0xff == 0x81` evaluate as `$type & (0xff == 0x81)` rather than `($type & 0xff) == 0x81`? Secondary rows record the signed `s` and `I` fields, whether the `cu` unsigned suffix is accepted, and what `scan ffffffff %x` stores. This backs the fix text of W156 (a signed scan value compared with an unsigned constant) and W154 (precedence grouping); it does not measure the analyser, compiler or VM.

## Assumptions

Each row used a fresh process on 2026-11-19 UTC with the exact script below on standard input, no command-line arguments and `LC_ALL=C`. Other inherited environment details: Not recorded. The Tcl rows are the session's reference `tclsh` builds; the Jim row is the pinned oracle built by `scripts/dev/ensure-jim-oracle.sh` at the revision in `rust/tcl-test-support/jim-reference.txt`. `catch {package require binary}` is a no-op where `binary` is built in. The `AS_PARSED`/`AS_INTENDED` rows distinguish the two groupings only because they evaluate to different values for this input. BIG-IP was not run; iRules are taken to share Tcl 8.4 semantics, which this file does not establish.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
catch {package require binary}
set n [binary scan \x81 c type]
puts [list SCANNED $n TYPE $type]
puts [list MASKED [expr {($type & 0xff) == 0x81}]]
puts [list SIGNED [expr {$type == -127}]]
puts [list UNSIGNED [expr {$type == 0x81}]]
puts [list UNPARENTHESISED [expr {$type & 0xff == 0x81}]]
puts [list AS_PARSED [expr {$type & (0xff == 0x81)}]]
puts [list AS_INTENDED [expr {($type & 0xff) == 0x81}]]
binary scan \x00\x80\x80\x00\x00\x00 sI short int
puts [list SHORT $short INT $int]
set code [catch {binary scan \x81 cu u} msg]
puts [list CU $code [expr {$code ? $msg : $u}]]
set code [catch {scan ffffffff %x x} msg]
puts [list SCAN_X $code [expr {$code ? $msg : $x}]]
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 1 {bad field specifier "u"}
SCAN_X 0 -1
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 0 129
SCAN_X 0 4294967295
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 0 129
SCAN_X 0 4294967295
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 0 129
SCAN_X 0 -1
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 0 129
SCAN_X 0 -1
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
SCANNED 1 TYPE -127
MASKED 1
SIGNED 1
UNSIGNED 0
UNPARENTHESISED 0
AS_PARSED 0
AS_INTENDED 1
SHORT -32768 INT -2147483648
CU 0 129
SCAN_X 0 4294967295
```

- Standard error: empty
