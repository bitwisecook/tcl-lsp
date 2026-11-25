# TclOO object definitions use their definition namespace

## Question

Does a standard `oo::objdefine` script run in the receiver's method namespace,
in the calling procedure, or in its own definition activation? This separates
immediate object-definition scripts from the method bodies they store.

## Assumptions

The receiver is created by stock `oo::object` and no custom definition namespace
or worker is installed. The method compares its current namespace with its
receiver's private namespace rather than retaining an implementation-generated
object identifier. Only the recorded releases and entry forms were measured.

## Script

```tcl-visible
namespace eval ::outer {
    proc probe {} {
        set callerLocal visible
        oo::object create ::o
        oo::objdefine ::o {
            puts [list objdefine [namespace current] [info level] [info exists callerLocal]]
            set objectDefinitionWrite written
        }
        puts [list afterObjdefine [info exists objectDefinitionWrite]]
        oo::objdefine ::o method inspect {} {
            list method [expr {[namespace current] eq [info object namespace [self]]}] [info level] [info exists callerLocal]
        }
        puts [::o inspect]
    }
}
::outer::probe
```

## Results

### C Tcl 8.6.18

Invocation: source-built `tmp/tcl8.6.18/unix/tclsh /tmp/oo-object-definition-frame.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
objdefine ::oo::objdefine 2 0
afterObjdefine 0
method 1 2 0
```

- Standard error: empty.

### C Tcl 9.0.4

Invocation: source-built `tmp/tcl9.0.4/unix/tclsh /tmp/oo-object-definition-frame.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
objdefine ::oo::objdefine 2 0
afterObjdefine 0
method 1 2 0
```

- Standard error: empty.
