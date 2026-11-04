# Tcl variable inlining execution

## Question

How do original variable parsing, reaching definitions, callee upvar mutation and read observers affect literal single-use variable inlining in the LSP? The recorded comparisons ask:

> A callee can change the caller-observed cell through an upvar alias between a store and its apparent single use.

> Does the callee upvar mutation change the original read relative to the handwritten literal replacement?

This file records one side of the comparison; its result alone does not establish the result of the other script.

## Assumptions

> Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

The captures use fresh native processes. Handwritten replacement scripts are controls, not output from the Rust refactor. BIG-IP was not tested. Other assumptions were not separately recorded.

## Script

```tcl-visible
set x 1; proc change {} {upvar #0 x alias; set alias 2}; change; puts $x
```

## Results

### c84 8.4.20, script file

- Recorded version: 8.4.20
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c85 8.5.19, script file

- Recorded version: 8.5.19
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c86 8.6.18, script file

- Recorded version: 8.6.18
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c90 9.0.4, script file

- Recorded version: 9.0.4
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c91 9.1.0, script file

- Recorded version: 9.1.0
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### jim 0.84-9-g5bac7c9, script file

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: script file
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c84 8.4.20, standard input

- Recorded version: 8.4.20
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c85 8.5.19, standard input

- Recorded version: 8.5.19
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c86 8.6.18, standard input

- Recorded version: 8.6.18
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c90 9.0.4, standard input

- Recorded version: 9.0.4
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### c91 9.1.0, standard input

- Recorded version: 9.1.0
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.

### jim 0.84-9-g5bac7c9, standard input

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: standard input
- Exit status: 0
- Standard output:

```text-visible
2
```

- Standard error: empty.
