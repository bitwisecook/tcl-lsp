# Signedness of `scan` integer conversions and `binary scan` integer fields

## Question

Which `scan` integer conversions store a value truncated to a signed 32-bit integer, and which `binary scan` integer fields store signed values, on each release? Which releases accept the `u` (unsigned) suffix on `binary scan` fields? This decides where W156 (a signed scan value compared with a constant outside its range) applies and which fix it suggests; it does not measure the analyser.

## Assumptions

Each row used a fresh process on 2026-11-19 UTC with the exact script below on standard input, no command-line arguments and `LC_ALL=C`, on a little-endian x86-64 host. Other inherited environment details: Not recorded. `t` and `n` are native-endian fields, so their bytes were chosen for a little-endian host. The Tcl rows are the session's reference `tclsh` builds; the Jim row is the pinned oracle built by `scripts/dev/ensure-jim-oracle.sh`. `catch {package require binary}` is a no-op where `binary` is built in. Only the listed inputs were measured; a conversion is described as truncating when an input outside the signed 32-bit range came back negative.

## Script

```tcl-visible
puts [list VERSION [info patchlevel]]
foreach {input format} {
    4294967295 %d 4294967295 %i ffffffff %x 37777777777 %o 4294967295 %u
    2147483648 %d 80000000 %x
    ffffffff %lx 4294967295 %ld 4294967295 %lu ffffffff %hx
    -1 %u -1 %x
} {
    set code [catch {scan $input $format v} result]
    puts [list $format $input [expr {$code ? "error: $result" : $v}]]
    unset -nocomplain v
}
catch {package require binary}
foreach {field bytes} {c \x81 s \x00\x80 S \x80\x00 i \x00\x00\x00\x80 I \x80\x00\x00\x00 t \x00\x80 n \x00\x00\x00\x80 cu \x81 su \x00\x80 iu \x00\x00\x00\x80} {
    set code [catch {binary scan $bytes $field v} result]
    puts [list binary $field [expr {$code ? "error: $result" : $v}]]
    unset -nocomplain v
}
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
%d 4294967295 -1
%i 4294967295 -1
%x ffffffff -1
%o 37777777777 -1
%u 4294967295 -1
%d 2147483648 -2147483648
%x 80000000 -2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 4294967295
%hx ffffffff -1
%u -1 -1
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t {error: bad field specifier "t"}
binary n {error: bad field specifier "n"}
binary cu {error: bad field specifier "u"}
binary su {error: bad field specifier "u"}
binary iu {error: bad field specifier "u"}
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
%d 4294967295 4294967295
%i 4294967295 4294967295
%x ffffffff 4294967295
%o 37777777777 4294967295
%u 4294967295 4294967295
%d 2147483648 2147483648
%x 80000000 2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 4294967295
%hx ffffffff 4294967295
%u -1 18446744073709551615
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t -32768
binary n -2147483648
binary cu 129
binary su 32768
binary iu 2147483648
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
%d 4294967295 4294967295
%i 4294967295 4294967295
%x ffffffff 4294967295
%o 37777777777 4294967295
%u 4294967295 4294967295
%d 2147483648 2147483648
%x 80000000 2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 4294967295
%hx ffffffff 4294967295
%u -1 18446744073709551615
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t -32768
binary n -2147483648
binary cu 129
binary su 32768
binary iu 2147483648
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
%d 4294967295 -1
%i 4294967295 -1
%x ffffffff -1
%o 37777777777 -1
%u 4294967295 18446744073709551615
%d 2147483648 -2147483648
%x 80000000 -2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 4294967295
%hx ffffffff -1
%u -1 18446744073709551615
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t -32768
binary n -2147483648
binary cu 129
binary su 32768
binary iu 2147483648
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
%d 4294967295 -1
%i 4294967295 -1
%x ffffffff -1
%o 37777777777 -1
%u 4294967295 18446744073709551615
%d 2147483648 -2147483648
%x 80000000 -2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 4294967295
%hx ffffffff -1
%u -1 18446744073709551615
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t -32768
binary n -2147483648
binary cu 129
binary su 32768
binary iu 2147483648
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
%d 4294967295 4294967295
%i 4294967295 4294967295
%x ffffffff 4294967295
%o 37777777777 4294967295
%u 4294967295 4294967295
%d 2147483648 2147483648
%x 80000000 2147483648
%lx ffffffff 4294967295
%ld 4294967295 4294967295
%lu 4294967295 {error: unsigned wide not supported}
%hx ffffffff 4294967295
%u -1 -1
%x -1 -1
binary c -127
binary s -32768
binary S -32768
binary i -2147483648
binary I -2147483648
binary t -32768
binary n -2147483648
binary cu 129
binary su 32768
binary iu 2147483648
```

- Standard error: empty
