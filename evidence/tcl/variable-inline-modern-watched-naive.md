# Tcl variable inlining execution

## Question

How do original variable parsing, reaching definitions, callee upvar mutation and read observers affect literal single-use variable inlining in the LSP? The recorded comparisons ask:

> Replacing a variable read by a literal can erase an observer even when the returned scalar bytes remain equal.

> Does the modern read trace remain observable in the original script while the handwritten literal replacement removes it?

This file records one side of the comparison; its result alone does not establish the result of the other script.

## Assumptions

> Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

The captures use fresh native processes. Handwritten replacement scripts are controls, not output from the Rust refactor. BIG-IP was not tested. Other assumptions were not separately recorded.

## Script

```tcl-visible
proc observe {args} {puts OBSERVED}
if {[catch {trace add variable x read observe} msg]} {puts UNAVAILABLE; puts $msg} else {puts 1}
```

## Results

### c84 8.4.20, script file

- Recorded version: 8.4.20
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
1
```

- Standard error: empty.

### c85 8.5.19, script file

- Recorded version: 8.5.19
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
1
```

- Standard error: empty.

### c86 8.6.18, script file

- Recorded version: 8.6.18
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
1
```

- Standard error: empty.

### c90 9.0.4, script file

- Recorded version: 9.0.4
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
1
```

- Standard error: empty.

### c91 9.1.0, script file

- Recorded version: 9.1.0
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
1
```

- Standard error: empty.

### jim 0.84-9-g5bac7c9, script file

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
UNAVAILABLE
invalid command name "trace"
```

- Standard error: empty.
