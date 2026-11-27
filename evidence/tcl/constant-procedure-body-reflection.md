# Procedure body reflection at publication

## Question

Can a Tcl caller observe a constant fold inside either procedure body at the
point `proc` publishes it? The optimiser needs this distinction before
replacing these expressions in source.

## Assumptions

The wrapper installs an execution trace on `::proc` before sourcing either
file. When `p` is published, the trace reads `info body p` and stops the
sourced script before it reaches the call to `p`. The script prints whether
each pair of captured bodies compares equal.

Each comparison starts with the fixture in the repository. The edited c13
copy replaces `[expr {$a + 0}]` with `1`. The edited c14 copy replaces
`[expr {1/3.0}]` with `0.3333333333333333` and leaves `2**64` intact.
Each replacement occurs once. The working directory is the repository root,
`LC_ALL` is `C` and `TCL_LIBRARY` selects the matching source tree's
library. [Original fixtures](../../docs/work/name-handling/handover/oo-dispatch-probe/constants/folds)

## Script

```tcl-visible
proc observe_definition {command code result operation} {
    if {$code != 0 || [lindex $command 1] ne "p"} {
        return
    }
    set ::observed_body [info body p]
    return -code error "stop after procedure publication"
}
trace add execution ::proc leave observe_definition
foreach {fixture original edited} $argv {
    foreach file [list $original $edited] {
        unset -nocomplain ::observed_body
        catch {source $file}
        lappend bodies $::observed_body
    }
    puts "$fixture [string equal [lindex $bodies 0] [lindex $bodies 1]]"
    unset bodies
}
```

## Results

Each interpreter ran the wrapper as a script file. The arguments were the
label `c13`, the original c13 path, the edited c13 path, the label `c14`,
the original c14 path and the edited c14 path. Both body edits were
observable before the first call to the procedure.

### Tcl 8.4.20

- Interpreter: `tmp/tcl8.4.20/unix/tclsh`.
- Exit status: 0.
- Standard error: empty.
- Standard output:

```text-visible
c13 0
c14 0
```

### Tcl 8.5.19

- Interpreter: `tmp/tcl8.5.19/unix/tclsh`.
- Exit status: 0.
- Standard error: empty.
- Standard output:

```text-visible
c13 0
c14 0
```

### Tcl 8.6.18

- Interpreter: `tmp/tcl8.6.18/unix/tclsh`.
- Exit status: 0.
- Standard error: empty.
- Standard output:

```text-visible
c13 0
c14 0
```

### Tcl 9.0.4

- Interpreter: `tmp/tcl9.0.4/unix/tclsh`.
- Exit status: 0.
- Standard error: empty.
- Standard output:

```text-visible
c13 0
c14 0
```

### Tcl 9.1.0

- Interpreter: `tmp/tcl9.1.0/unix/tclsh`.
- Exit status: 0.
- Standard error: empty.
- Standard output:

```text-visible
c13 0
c14 0
```
