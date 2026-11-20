# Signed `binary scan`, `scan` and `format` in the appliance's host tclsh

## Question

On the BIG-IP host itself, what do the stock `tclsh8.4` and `tclsh8.5` binaries return for the same signed `binary scan`, `scan` and `format` cases as the TMM case [bigip-tmm-signed-scan-and-format](bigip-tmm-signed-scan-and-format.md)? This is the control for that case. It does not show what TMM does.

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26. The script was copied over SSH to `/var/tmp` and run as `tclsh8.4 <file>` and `tclsh8.5 <file>` as root, then deleted. The host `tclsh8.4` reports patchlevel 8.4.13, not the 8.4.6 that TMM embeds.

## Script

```tcl-visible
puts [list VERSION [info patchlevel] [array get tcl_platform wordSize]]
foreach {id script} {
    bscan {binary scan \x81\x00\x80 cS b s; list $b $s}
    bscan_cu {binary scan \x81 cu b; set b}
    bscan_masked {binary scan \x81\x00\x80 cS b s; list [expr {$b & 0xff}] [expr {$s & 0xffff}]}
    scan_x {scan ffffffff %x v; set v}
    scan_lx {scan ffffffff %lx v; set v}
    fmt_hex {format %02x -127}
    fmt_hex_masked {format %02x [expr {-127 & 0xff}]}
    fmt_u {format %u -127}
    fmt_c {binary scan [format %c -127] H* h; set h}
    fmt_c_len {string length [format %c -127]}
    intwidth {list [expr {1 << 31}] [expr {1 << 32}] [expr {0x7fffffff + 1}] [expr {-1 >> 1}]}
} {
    set c [catch $script v]
    puts [list $id $c $v]
}
```

## Results

### Tcl 8.4.13 (BIG-IP host /usr/bin/tclsh8.4)

- Exit status: 0
- Standard output:

```text-visible
VERSION 8.4.13 {wordSize 8}
bscan 0 {-127 128}
bscan_cu 1 {bad field specifier "u"}
bscan_masked 0 {129 128}
scan_x 0 -1
scan_lx 0 4294967295
fmt_hex 0 ffffffffffffff81
fmt_hex_masked 0 81
fmt_u 0 18446744073709551489
fmt_c 0 fd
fmt_c_len 0 1
intwidth 0 {2147483648 4294967296 2147483648 -1}
```

- Standard error: Not recorded

### Tcl 8.5.13 (BIG-IP host /usr/bin/tclsh8.5)

- Exit status: 0
- Standard output:

```text-visible
VERSION 8.5.13 {wordSize 8}
bscan 0 {-127 128}
bscan_cu 0 129
bscan_masked 0 {129 128}
scan_x 0 4294967295
scan_lx 0 4294967295
fmt_hex 0 ffffffffffffff81
fmt_hex_masked 0 81
fmt_u 0 18446744073709551489
fmt_c 0 fd
fmt_c_len 0 1
intwidth 0 {2147483648 4294967296 2147483648 -1}
```

- Standard error: Not recorded
