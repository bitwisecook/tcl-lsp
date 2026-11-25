# Channel callback protocols

## Question

Which words do reflected-channel methods receive, when is the `truncate`
method accepted, and in what order does a write-and-pop transformation call
its methods? These observations constrain registry callback signatures and
release gates, not the existence or execution of a handler in an analysed
program.

## Assumptions

The script ran as a standalone file on 24 November 2026 under each pinned C
Tcl interpreter with its own library, and the repository-pinned Jim
0.84-15-g2f8e155. No packages were loaded. Both channels were reflected
channels, so the script used no filesystem or network I/O. Method counts
include the selector and channel/transform handle. Other inherited
environment details were not recorded. The transform row covers only a
write-and-pop sequence; it is not a universal lifecycle order.

## Script

```tcl-visible
if {![llength [info commands chan]]} {
    puts {chan unavailable}
    exit
}
set calls {}
proc handler {method id args} {
    lappend ::calls [list $method [expr {[llength $args] + 2}]]
    switch -- $method {
        initialize {
            set methods {initialize finalize watch read write seek configure cget cgetall blocking}
            if {[package vcompare [info tclversion] 9.0] >= 0} {lappend methods truncate}
            return $methods
        }
        read {return {}}
        write {return [string length [lindex $args 0]]}
        seek {return 0}
        cget {return value}
        cgetall {return {-example value}}
        default {return {}}
    }
}
set c [chan create {read write} handler]
tell $c
chan configure $c -example
chan configure $c -example value
chan configure $c -blocking 0
if {[package vcompare [info tclversion] 9.0] >= 0} {chan truncate $c 5}
puts -nonewline $c abc
flush $c
read $c 1
close $c
puts [list reflected $calls]
proc truncated {method args} {
    if {$method eq {initialize}} {return {initialize finalize watch read truncate}}
    return {}
}
set rejected [catch {chan create read truncated} c]
if {!$rejected} {close $c}
puts [list truncate-rejected $rejected]
if {[package vcompare [info tclversion] 8.6] >= 0} {
    set transform_calls {}
    proc transform {method id args} {
        lappend ::transform_calls $method
        switch -- $method {
            initialize {return {initialize finalize read write drain flush clear}}
            read - write {return [lindex $args 0]}
            default {return {}}
        }
    }
    set c [chan create write handler]
    chan push $c transform
    puts -nonewline $c abc
    chan pop $c
    close $c
    puts [list transform $transform_calls]
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
reflected {{initialize 3} {seek 4} {cget 3} {configure 4} {blocking 3} {write 3} {read 3} {finalize 2}}
truncate-rejected 1
```

- Standard error: empty.

### C Tcl 8.6.18

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
reflected {{initialize 3} {seek 4} {cget 3} {configure 4} {blocking 3} {write 3} {read 3} {finalize 2}}
truncate-rejected 1
transform {initialize clear write drain flush finalize}
```

- Standard error: empty.

### C Tcl 9.0.4

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
reflected {{initialize 3} {seek 4} {cget 3} {configure 4} {blocking 3} {truncate 3} {write 3} {read 3} {finalize 2}}
truncate-rejected 0
transform {initialize clear write drain flush finalize}
```

- Standard error: empty.

### C Tcl 9.1.0

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
reflected {{initialize 3} {seek 4} {cget 3} {configure 4} {blocking 3} {truncate 3} {write 3} {read 3} {finalize 2}}
truncate-rejected 0
transform {initialize clear write drain flush finalize}
```

- Standard error: empty.

### Jim 0.84-15-g2f8e155

- Invocation: script file argument.
- Exit status: 0
- Standard output:

```text-visible
chan unavailable
```

- Standard error: empty.
