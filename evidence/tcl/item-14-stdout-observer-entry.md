# Inherited stdout transforms and fresh child interpreters

## Question

Does a fresh Tcl child interpreter's command table settle whether its output calls can change later command and variable lookup? Item 14's three fixtures distinguish missing globals, namespace code executed from the root namespace, and writes to a constant. A separate control checks the qualified scalar name accepted by the Tcl 9 constant handler.

## Assumptions

Each pinned C Tcl release ran one process with `LC_ALL=C` and the repository root as its working directory. Each case evaluates the unchanged fixture in a newly created child. The parent uses unbuffered stdout and installs the transform before creating the child in the `observe` case. The transform modifies the child on its first write. The qualified-name control runs in the parent after the transform has been removed. Other environment details were not recorded. These runs measure C Tcl execution. Analysis of the fixture alone has no evidence about the parent's channel.

## Script

```tcl-visible
proc adjust_stdout {operation handle args} {
    switch -- $operation {
        initialize { return {initialize finalize write} }
        finalize {}
        write {
            global child case wrote
            if {!$wrote} {
                set wrote 1
                switch -- $case {
                    e05 { $child eval {set ::nope observer} }
                    e23 { $child eval {proc ::helper {x} {return observer:$x}} }
                    e20 {
                        $child eval {
                            rename ::set ::ordinary_set
                            proc ::set {args} {
                                if {[llength $args] == 2 && [lindex $args 0] eq "limit"} {
                                    return [lindex $args 1]
                                }
                                tailcall ::ordinary_set {*}$args
                            }
                        }
                    }
                }
            }
            return [lindex $args 0]
        }
    }
}
set base [file join docs work name-handling handover oo-dispatch-probe dynamic]
fconfigure stdout -buffering none
foreach {case fixture} {
    e05 e05-global.tcl
    e23 e23-ns-code.tcl
    e20 e20-const-90.tcl
} {
    foreach mode {plain observe} {
        puts [list CASE $case $mode]
        if {$mode eq "observe"} {
            set wrote 0
            chan push stdout adjust_stdout
        }
        set child [interp create]
        set status [catch {$child eval [list source [file join $base $fixture]]} message]
        interp delete $child
        if {$mode eq "observe"} { chan pop stdout }
        puts [list COMPLETION $status $message]
    }
}
namespace eval ::qualified {}
set creation [catch {const ::qualified::x 8} details]
if {$creation} {
    puts [list QUALIFIED $creation $details]
} else {
    set write [catch {set ::qualified::x 9} details]
    puts [list QUALIFIED $creation $write $details $::qualified::x]
}
```

## Results

### Tcl 8.6.18

- Launch argv: pinned `tmp/tcl8.6.18/unix/tclsh` and a temporary file containing the script above.
- Input channel: script file.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
CASE e05 plain
11
1:can't read "nope": no such variable
COMPLETION 0 {}
CASE e05 observe
11
0:observer
COMPLETION 0 {}
CASE e23 plain
h1
h2
1:invalid command name "helper"
COMPLETION 0 {}
CASE e23 observe
h1
h2
0:observer:3
COMPLETION 0 {}
CASE e20 plain
COMPLETION 1 {invalid command name "const"}
CASE e20 observe
COMPLETION 1 {invalid command name "const"}
QUALIFIED 1 {invalid command name "const"}
```

- Standard error: empty

### Tcl 9.0.4

- Launch argv: pinned `tmp/tcl9.0.4/unix/tclsh` and a temporary file containing the script above.
- Input channel: script file.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
CASE e05 plain
11
1:can't read "nope": no such variable
COMPLETION 0 {}
CASE e05 observe
11
0:observer
COMPLETION 0 {}
CASE e23 plain
h1
h2
1:invalid command name "helper"
COMPLETION 0 {}
CASE e23 observe
h1
h2
0:observer:3
COMPLETION 0 {}
CASE e20 plain
3
1:can't set "limit": variable is a constant
COMPLETION 0 {}
CASE e20 observe
3
0:4
COMPLETION 0 {}
QUALIFIED 0 1 {can't set "::qualified::x": variable is a constant} 8
```

- Standard error: empty

### Tcl 9.1.0

- Launch argv: pinned `tmp/tcl9.1.0/unix/tclsh` and a temporary file containing the script above.
- Input channel: script file.
- Command-line arguments: none.
- Exit status: 0
- Standard output:

```text-visible
CASE e05 plain
11
1:can't read "nope": no such variable
COMPLETION 0 {}
CASE e05 observe
11
0:observer
COMPLETION 0 {}
CASE e23 plain
h1
h2
1:invalid command name "helper"
COMPLETION 0 {}
CASE e23 observe
h1
h2
0:observer:3
COMPLETION 0 {}
CASE e20 plain
3
1:can't set "limit": variable is a constant
COMPLETION 0 {}
CASE e20 observe
3
0:4
COMPLETION 0 {}
QUALIFIED 0 1 {can't set "::qualified::x": variable is a constant} 8
```

- Standard error: empty
