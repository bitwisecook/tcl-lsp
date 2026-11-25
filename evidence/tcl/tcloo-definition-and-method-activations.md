# TclOO definition scripts use a definition activation

## Question

Do standard `oo::class create` and `oo::define` script bodies enter the same
activation and namespace as a later method body, or share the calling
procedure's variables? The registry must not label an immediate definition
script as a deferred receiver-method body merely because both configure the
same object system.

## Assumptions

The commands are stock C Tcl implementations using the standard metaclass
and definition namespaces. No custom definition workers, command replacement
or namespace override is installed. The method's second output field compares
its current namespace with its receiver's private namespace instead of retaining
an implementation-generated object name. These runs establish only the two
recorded releases and these entry forms.

## Script

```tcl-visible
namespace eval ::outer {
    proc probe {} {
        set callerLocal visible
        oo::class create ::C {
            puts [list create [namespace current] [info level] [info exists callerLocal]]
            set classBodyWrite written
        }
        puts [list afterCreate [info exists classBodyWrite]]
        oo::define ::C {
            puts [list define [namespace current] [info level] [info exists callerLocal]]
            set defineBodyWrite written
        }
        puts [list afterDefine [info exists defineBodyWrite]]
        oo::define ::C method inspect {} {
            list method [expr {[namespace current] eq [info object namespace [self]]}] [info level] [info exists callerLocal]
        }
        puts [[::C new] inspect]
    }
}
::outer::probe
```

## Results

### C Tcl 8.6.18

Invocation: source-built `tmp/tcl8.6.18/unix/tclsh /tmp/oo-definition-frame-context.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
create ::oo::define 2 0
afterCreate 0
define ::oo::define 2 0
afterDefine 0
method 1 2 0
```

- Standard error: empty.

### C Tcl 9.0.4

Invocation: source-built `tmp/tcl9.0.4/unix/tclsh /tmp/oo-definition-frame-context.tcl`.

- Exit status: 0.
- Standard output:

```text-visible
create ::oo::define 2 0
afterCreate 0
define ::oo::define 2 0
afterDefine 0
method 1 2 0
```

- Standard error: empty.
