# Tcl counted-source execution evidence

## Question

> A factory result stored by an unqualified namespace-source variable command is not necessarily readable through a namespace-qualified variable spelling. Positive publication/dispatch requires a source that reaches the result and target calls.

> With the created result retained through an explicit rooted variable, which original Jim ensemble spelling, callable target/formals/command-query values and exact selector/option rejections are observed?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four exact original ASCII LF sources run separately through the unchanged fresh full-init counted C API driver. The explicit rooted result variable is part of each original positive input, and the standalone selector/option source retains exact caught errors. All original sources/requests/drivers/launchers/ELFs/streams/receipts and required provider/header/archive/build/source pins are retained. The independently authored namespace-qualified-result inputs and all actual compiler/process captures remain under limited-control with their measured original outcomes. Full Jim namespace/helper/ensemble source files remain source-inspection evidence separate from public result/dispatch values. BIG-IP and Rust assertions are not tested.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::N {
        proc go {value} {list DEFAULT $value}
        set ::made [namespace ensemble create]
    }
    list $::made [::N go VALUE] [info args ::N] [lsort [::N -commands]]
}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|4e207b44454641554c542056414c55457d207b737562636d6420617267737d20676f
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
N {DEFAULT VALUE} {subcmd args} go⟦EOF⟧
```
