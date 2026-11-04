# Tcl counted-source execution evidence

## Question

> A namespace helper query must establish the actual public procedure-body capability before recording returned declarations or replacement calls. A version-variable existence guard supplies no such helper capability.

> When the original public info-body guard reaches the Jim namespace helper, what formals/body results, global/current helper replacement values and caught unsupported/arity results are observed?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four exact original ASCII LF sources run independently in fresh full provider initialisation under the unchanged counted C API driver, without an added outer source envelope. Public info-body catch is part of the source itself. Whole sources/requests/driver/capture commands/ELFs/stdout/stderr/receipts and required executable/header/archive/source/build pins are retained. Jim public body values and full vendored source assets remain independent channels. The version-variable-gated four-control input and all its actual receipts are losslessly retained under limited-control; its original guard does not reach a helper call. BIG-IP and Rust assertions are not tested. The selected source schema supplies neither an installed scripted helper nor successful forwarding, Native publication/frame/dispatch or Normal completion. Returned public helper bodies and current helper binding remain independent premises.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::N {
        proc {namespace ensemble} args {list LOCAL $args [uplevel 1 namespace current]}
        namespace ensemble create -map {go ::list}
    }
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
ORIGINAL|0|4c4f43414c207b637265617465202d6d6170207b676f203a3a6c6973747d7d203a3a4e
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
LOCAL {create -map {go ::list}} ::N⟦EOF⟧
```
