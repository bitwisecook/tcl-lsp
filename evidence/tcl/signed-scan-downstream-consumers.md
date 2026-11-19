# What stock commands do with a signed `binary scan` value

## Question

When a byte or short read by `binary scan` with a signed field (`-127` for `0x81`, `-64` for `0xc0`, `-1` for `0xff`, `-32768` for `0x8000`) reaches a command or operator that expects the unsigned value — hex or unsigned `format`, a dotted-quad `format`, combining bytes with `<<`/`|` or `*`/`+`, a list or string index, a range or repeat length, `incr` — what does each release produce, and does masking with `& 0xff` restore the intended result? This scopes which downstream consumers a signed-scan check could report; it does not measure the analyser, F5 commands or BIG-IP.

## Assumptions

Each row used a fresh process on 2026-11-19 UTC with the exact script below on standard input, no command-line arguments and `LC_ALL=C`. Other inherited environment details: Not recorded. The Tcl rows are the session's reference `tclsh` builds; the Jim row is the pinned oracle built by `scripts/dev/ensure-jim-oracle.sh`. `catch {package require binary}` is a no-op where `binary` is built in. `FORMAT_CHAR` reports the code point `scan %c` reads back from `format %c`, so no raw byte is written to standard output.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
catch {package require binary}
binary scan \x81\xc0\xff\x80\x00 cccS byte hi lo short
puts [list SCANNED $byte $hi $lo $short]
proc try {name script} {
    set code [catch {uplevel 1 $script} result]
    puts [list $name [expr {$code ? "error: $result" : $result}]]
}
try FORMAT_HEX {format %02x $byte}
try FORMAT_HEX_MASKED {format %02x [expr {$byte & 0xff}]}
try FORMAT_DEC {format %d $byte}
try FORMAT_UNSIGNED {format %u $byte}
try FORMAT_CHAR {scan [format %c $byte] %c code; set code}
try FORMAT_DOTTED {format %d.%d.%d.%d 192 $hi $lo 1}
try COMPOSE_SHIFT_OR {expr {($hi << 8) | $lo}}
try COMPOSE_MASKED {expr {(($hi & 0xff) << 8) | ($lo & 0xff)}}
try COMPOSE_ADD {expr {$hi * 256 + $lo}}
try LINDEX {lindex {zero one two} $byte}
try STRING_INDEX {string index abcdef $byte}
try STRING_RANGE_LENGTH {string length [string range abcdef 0 [expr {$short - 1}]]}
try BINARY_FORMAT_BACK {binary scan [binary format c $byte] H2 hex; set hex}
try STRING_REPEAT {string length [string repeat x $byte]}
try LRANGE {llength [lrange {a b c d} 0 $short]}
try INCR_COUNTER {set n 0; incr n $short}
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffffffffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED -127
FORMAT_CHAR 65533
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffffffffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED 18446744073709551489
FORMAT_CHAR 65533
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffffffffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED 18446744073709551489
FORMAT_CHAR 65533
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED 4294967169
FORMAT_CHAR 65533
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED 4294967169
FORMAT_CHAR 65533
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
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
SCANNED -127 -64 -1 -32768
FORMAT_HEX ffffffffffffff81
FORMAT_HEX_MASKED 81
FORMAT_DEC -127
FORMAT_UNSIGNED 18446744073709551489
FORMAT_CHAR 2097025
FORMAT_DOTTED 192.-64.-1.1
COMPOSE_SHIFT_OR -1
COMPOSE_MASKED 49407
COMPOSE_ADD -16385
LINDEX {}
STRING_INDEX {}
STRING_RANGE_LENGTH 0
BINARY_FORMAT_BACK 81
STRING_REPEAT 0
LRANGE 0
INCR_COUNTER -32768
```

- Standard error: empty
