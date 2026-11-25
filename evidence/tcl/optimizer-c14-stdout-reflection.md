# A stdout transform observes a rewritten procedure body

## Question

Can a caller distinguish a power-to-shift rewrite of `2**64` in the
`c14-float-format` fixture through `info body p`, even when the computed
values printed by the fixture are identical?

## Assumptions

The wrapper pushes a transform on stdout before sourcing the two files.
The first argument is the unchanged fixture. The second is a temporary
copy changing only `2**64` to `1<<64`. The transform logs the body of
`p` on the first write from each file. The working directory is the
repository root. `LC_ALL` is `C` and `TCL_LIBRARY` selects the
matching pinned release's library. Other environment details were not
recorded. [Original fixture](../../docs/work/name-handling/handover/oo-dispatch-probe/constants/folds/c14-float-format.tcl)

## Script

```tcl-visible
proc observe {method channel args} {
    switch -- $method {
        initialize {return {initialize finalize write}}
        finalize {return}
        write {
            if {!$::reported} {
                set ::reported 1
                puts stderr [info body p]
            }
            return [lindex $args 0]
        }
    }
}
chan push stdout observe
fconfigure stdout -buffering none
foreach script $argv {
    set reported 0
    source $script
}
```

## Results

### Tcl 8.6.18

- Launch argv: `tmp/tcl8.6.18/unix/tclsh`, the temporary observer script, the original fixture and the rewritten file.
- Input channel: script files.
- Exit status: 0.
- Standard output:

```text-visible
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {2**64}] [expr {-0.0}]]⟦SPACE⟧
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {1<<64}] [expr {-0.0}]]⟦SPACE⟧
```

### Tcl 9.0.4

- Launch argv: `tmp/tcl9.0.4/unix/tclsh`, the temporary observer script, the original fixture and the rewritten file.
- Input channel: script files.
- Exit status: 0.
- Standard output:

```text-visible
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {2**64}] [expr {-0.0}]]⟦SPACE⟧
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {1<<64}] [expr {-0.0}]]⟦SPACE⟧
```

### Tcl 9.1.0

- Launch argv: `tmp/tcl9.1.0/unix/tclsh`, the temporary observer script, the original fixture and the rewritten file.
- Input channel: script files.
- Exit status: 0.
- Standard output:

```text-visible
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {2**64}] [expr {-0.0}]]⟦SPACE⟧
 return [list [expr {1/3.0}] [expr {0.1 + 0.2}] [expr {1e21}] [expr {1<<64}] [expr {-0.0}]]⟦SPACE⟧
```
