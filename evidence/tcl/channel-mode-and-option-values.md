# Channel modes and option values

## Question

When are empty reflected-channel creation and posted-event masks accepted,
and can the generic translation and EOF-character validators reject values
without knowing the channel's I/O directions? Does `-blocking` accept numeric
truth values beyond the literal digits zero and one? These results constrain source
literal validation; they do not establish that an analysed channel exists.

## Assumptions

The script ran as a standalone file on 25 November 2026 under each pinned C
Tcl interpreter with its own library. No packages were loaded. The script
creates only reflected channels and performs no filesystem or network I/O.
The four modes are empty, read, write and read/write; Tcl 8.5 skips the empty
mode after rejection. Each caught result is 0 for success and 1 for an error.
Translation columns test `bad`, `bad bad`, `bad bad bad` and the malformed list
`{`. EOF-character columns test `bad`, `é`, `é é`, NUL, `x y z` and `{`.
The non-ASCII character is U+00E9, Latin small letter e with acute. These
observations cover these exact values, not all possible custom drivers.
Other inherited environment details were not recorded.

## Script

```tcl-visible
if {![llength [info commands chan]]} {
    puts {chan unavailable}
    exit
}
proc rc {method id args} {
    if {$method eq "initialize"} {return {initialize finalize watch read write}}
    if {$method eq "write"} {return [string length [lindex $args 0]]}
    return
}
set failed [catch {chan create {} rc} c]
puts [list empty-mode $failed]
if {!$failed} {close $c}
foreach mode {{} read write {read write}} {
    if {[catch {chan create $mode rc} c]} {continue}
    puts [list postevent-empty $mode [catch {chan postevent $c {}}]]
    set results {}
    foreach value {bad {bad bad} {bad bad bad} \{} {
        lappend results [catch {chan configure $c -translation $value}]
    }
    puts [list translation $mode $results]
    set results {}
    foreach value {bad é {é é} \u0000 {x y z} \{} {
        lappend results [catch {chan configure $c -eofchar $value}]
    }
    puts [list eofchar $mode $results]
    close $c
}
```

## Results

### C Tcl 8.4.20

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
chan unavailable
```

- Standard error: empty.

### C Tcl 8.5.19

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
empty-mode 1
postevent-empty read 1
translation read {1 1 1 1}
eofchar read {0 1 1 1 1 1}
postevent-empty write 1
translation write {1 1 1 1}
eofchar write {0 1 1 1 1 1}
postevent-empty {read write} 1
translation {read write} {1 1 1 1}
eofchar {read write} {0 1 1 1 1 1}
```

- Standard error: empty.

### C Tcl 8.6.18

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
empty-mode 0
postevent-empty {} 1
translation {} {0 0 1 1}
eofchar {} {0 1 1 1 1 1}
postevent-empty read 1
translation read {1 1 1 1}
eofchar read {0 1 1 1 1 1}
postevent-empty write 1
translation write {1 1 1 1}
eofchar write {0 1 1 1 1 1}
postevent-empty {read write} 1
translation {read write} {1 1 1 1}
eofchar {read write} {0 1 1 1 1 1}
```

- Standard error: empty.

### C Tcl 9.0.4

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
empty-mode 0
postevent-empty {} 1
translation {} {0 0 1 1}
eofchar {} {1 1 1 1 1 0}
postevent-empty read 1
translation read {1 1 1 1}
eofchar read {1 1 1 1 1 0}
postevent-empty write 1
translation write {1 1 1 1}
eofchar write {1 1 1 1 1 0}
postevent-empty {read write} 1
translation {read write} {1 1 1 1}
eofchar {read write} {1 1 1 1 1 0}
```

- Standard error: empty.

### C Tcl 9.1.0

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
empty-mode 0
postevent-empty {} 1
translation {} {0 0 1 1}
eofchar {} {1 1 1 1 1 0}
postevent-empty read 1
translation read {1 1 1 1}
eofchar read {1 1 1 1 1 0}
postevent-empty write 1
translation write {1 1 1 1}
eofchar write {1 1 1 1 1 0}
postevent-empty {read write} 1
translation {read write} {1 1 1 1}
eofchar {read write} {1 1 1 1 1 0}
```

- Standard error: empty.

## Strict channel booleans

This additional script ran on 25 November 2026 under the same five pinned C Tcl
interpreters, invoked by their `/workspace/toolchains/bin/tclsh*` wrappers with
the script on standard input. It queries and restores standard output's original
blocking mode. No packages were loaded. Each result is the catch status of one
`fconfigure stdout -blocking` setter call, in the order of the literal list.
Other inherited environment details were not recorded.

```tcl-visible
set original [fconfigure stdout -blocking]
set results {}
foreach value {0 1 true FALSE t of 2 -1 0x1 01 { 1} 1.0 o {}} {
    lappend results [catch {fconfigure stdout -blocking $value}]
}
fconfigure stdout -blocking $original
puts [list [info patchlevel] $results]
```

All five invocations exited with status 0 and empty standard error. Standard
output, one line per corresponding reference interpreter:

```text-visible
8.4.20 {0 0 0 0 0 0 1 1 1 1 1 1 1 1}
8.5.19 {0 0 0 0 0 0 1 1 1 1 1 1 1 1}
8.6.18 {0 0 0 0 0 0 1 1 1 1 1 1 1 1}
9.0.4 {0 0 0 0 0 0 1 1 1 1 1 1 1 1}
9.1.0 {0 0 0 0 0 0 1 1 1 1 1 1 1 1}
```

These observations agree with the strict boolean string acceptor, rather than
the numeric truthiness accepted by Tcl expression conditions.
