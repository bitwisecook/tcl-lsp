# Fresh procedure-local variable existence

## Question

When a global `::b` exists but a fresh procedure has no local binding for `b`,
does `info exists b` inside that procedure see the global variable or report
the local variable absent? In the same procedure, is `Params(key)` present
when that local array element has not been created?

This distinguishes the name lookup the LSP, compiler and VM need to model for
this exact unlinked procedure case. The observations do not establish that any
of those implementations already behaves correctly, or answer a broader
question about linked variables or other execution contexts.

## Assumptions

- No additional assumptions were recorded in the capture.
- The capture does not record whether the script was supplied as a file or on
  standard input. The C Tcl labels do not record exact patch levels.
- No BIG-IP, `global`, `upvar` or TclOO execution is recorded for this script.

## Script

```tcl-visible
set ::b PRESENT; proc f {} {list [info exists b] [info exists Params(key)]}; puts [f]
```

## Results

### Tcl 8.4

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.

### Tcl 8.5

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.

### Tcl 8.6

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.

### Tcl 9.0

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.

### Tcl 9.1

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.

### Jim Tcl 0.84

- Exit status: `0`
- Standard output:

```text-visible
0 0
```

- Standard error: empty.
