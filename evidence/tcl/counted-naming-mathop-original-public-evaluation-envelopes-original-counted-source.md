# Tcl counted-source execution evidence

## Question

> A native arithmetic-worker replacement comparison depends on its exact original evaluation channel. Bare counted completion, a CLI catch/puts envelope and a variable-source catch envelope must retain their own public results and guest stdout before any compiler-selection cause is inferred.

> What public code/result and guest stdout does the original mathop replacement control produce as bare counted source, its exact original CLI envelope and a separate variable-source catch envelope?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three separate ASCII sources cover a bare operation, a CLI catch/result/binary-scan/puts envelope, and a script variable passed to catch. Each runs in a fresh fully initialized provider through the counted-source C API driver without an added source envelope. Guest and driver output retain their recorded buffering order.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands ::tcl::mathop::+]]} {set marker UNAVAILABLE} else {rename ::tcl::mathop::+ original; proc ::tcl::mathop::+ args {return OVERRIDE}; list [original 2 3] [::tcl::mathop::+ 2 3]}⟦EOF⟧
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|35204f56455252494445
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
5 OVERRIDE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|35204f56455252494445
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
5 OVERRIDE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|35204f56455252494445
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
5 OVERRIDE⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|35204f56455252494445
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
5 OVERRIDE⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```
