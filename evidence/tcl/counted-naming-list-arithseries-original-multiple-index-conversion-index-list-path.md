# Tcl counted-source execution evidence

## Question

> A finite arithmetic sequence query needs its exact selected index spelling, public result and observed representation type. Result equality alone cannot establish representation retention or a huge-allocation/host-limit contract.

> For the original nine-element lseq value, what caught result and reported representation type follow single-index, index-list path, separate-index and out-of-range path calls?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four exact original ASCII LF source controls run independently in fresh full Tcl_Init or Jim core/static initialisation under the unchanged counted Native202/210/222 C API driver, with no added outer source envelope. Each available source creates lseq9, catches its exact original lindex spelling, reads ::tcl::unsupported::representation and emits only the extracted public type word with caught result/code. Addresses and complete representation descriptions are not emitted. Complete original source/requests, driver, launch script, actual commands/receipts/stdout/stderr/ELFs and provider source/header/archive/build pins are preserved. Two full pinned tclListObj.c copies and exact TclLindexFlat excerpts are inspected independently; source text does not identify private state during the executed call. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands lseq]]} {
    list UNAVAILABLE
} else {
    set sequence [lseq 9]
    set code [catch {lindex $sequence {0 0}} result]
    set description [::tcl::unsupported::representation $sequence]
    if {![regexp {^value is a ([^ ]+)} $description -> type]} {
        error {unrecognised representation kind}
    }
    list $code $result $type
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
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|554e415641494c41424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
UNAVAILABLE⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|302030206c697374
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 list⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|302030206c697374
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 list⟦EOF⟧
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
