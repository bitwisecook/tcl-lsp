# Command count after eliminating a procedure call

## Question

Does replacing `puts [p]` with the observed result in `c13-number-forms`
and `c14-float-format` preserve the command count a caller can read after
sourcing each file?

## Assumptions

The wrapper sources each argument in a new child interpreter. It leaves
the standard channels unchanged and queries the child after sourcing
finishes. The arguments are the original `c13-number-forms`, its copy
with the final line replaced by `puts {01 1 0x10 16 1.0 1 1 0}`, the
original `c14-float-format`, and its copy with the final line replaced
by `puts {0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0}`.
The procedure bodies are byte-identical within each pair. The working
directory is the repository root. `LC_ALL` is `C` and `TCL_LIBRARY`
selects the matching release's library. Other environment details were
not recorded. [Original fixtures](../../docs/work/name-handling/handover/oo-dispatch-probe/constants/folds)

## Script

```tcl-visible
foreach script $argv {
    set child [interp create]
    $child eval [list source $script]
    puts stderr [$child eval {info cmdcount}]
    interp delete $child
}
```

## Results

### Tcl 8.6.18

- Launch argv: `tmp/tcl8.6.18/unix/tclsh`, the temporary observer script and the four fixture paths in the order above.
- Input channel: script files.
- Exit status: 0
- Standard output:

```text-visible
01 1 0x10 16 1.0 1 1 0
01 1 0x10 16 1.0 1 1 0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
151
141
146
141
```

### Tcl 9.0.4

- Launch argv: `tmp/tcl9.0.4/unix/tclsh`, the temporary observer script and the four fixture paths in the order above.
- Input channel: script files.
- Exit status: 0
- Standard output:

```text-visible
01 1 0x10 16 1.0 1 1 0
01 1 0x10 16 1.0 1 1 0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
135
125
130
125
```

### Tcl 9.1.0

- Launch argv: `tmp/tcl9.1.0/unix/tclsh`, the temporary observer script and the four fixture paths in the order above.
- Input channel: script files.
- Exit status: 0
- Standard output:

```text-visible
01 1 0x10 16 1.0 1 1 0
01 1 0x10 16 1.0 1 1 0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
0.3333333333333333 0.30000000000000004 1e+21 18446744073709551616 -0.0
```

- Standard error:

```text-visible
41
31
36
31
```
