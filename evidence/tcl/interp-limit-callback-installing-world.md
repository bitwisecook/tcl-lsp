# Interpreter limit callbacks use the installing world

## Question

When an interpreter installs a command-limit script on a child, does that
script resolve names in the child or in the installing interpreter? Which
activation and namespace does it enter? This determines the registry evaluation context of the callback operand and prevents navigation into the wrong world.

## Assumptions

The commands are the stock C Tcl implementations. The parent and child each
have a different `::world` value. The callback removes the command limit after
recording its entry context, so execution can finish. These runs establish
only the command-limit callback behaviour on the two recorded releases.

## Script

```tcl-visible
set ::world parent
set ::seen absent
interp create child
interp eval child {set ::world child}
interp limit child commands -value 10 -granularity 1 -command {
    set ::seen [list $::world [info level] [namespace current]]
    interp limit child commands -value {}
}
interp eval child {for {set i 0} {$i < 20} {incr i} {set x $i}}
puts $::seen
puts [interp eval child {info exists ::seen}]
```

## Results

### C Tcl 8.6.18

Invocation: source-built `tmp/tcl8.6.18/unix/tclsh /tmp/interp-limit-context.tcl`.

- Exit status: 0.

- Standard output:

```text-visible
parent 0 ::
0
```

- Standard error: empty.

### C Tcl 9.0.4

Invocation: source-built `tmp/tcl9.0.4/unix/tclsh /tmp/interp-limit-context.tcl`.

- Exit status: 0.

- Standard output:

```text-visible
parent 0 ::
0
```

- Standard error: empty.
