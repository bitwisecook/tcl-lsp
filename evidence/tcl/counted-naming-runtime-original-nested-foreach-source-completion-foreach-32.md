# Tcl counted-source execution evidence

## Question

> A host implementation limit or an expectation of failure does not establish the public completion of nested original foreach source. Exact depth-bounded public controls must retain the selected provider, counted source and completion channel independently of private loop compilation or host-stack assumptions.

> Do the three exact original nested foreach sources at depths 32, 128, 300 complete or raise an error on each pinned provider, and what counted public result do they return?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three exact ASCII LF sources consist of a counted number of identical foreach x {1} openings, one set done 1 command, and matching closing braces. Fresh full Tcl_Init or Jim core/static-extension interpreters evaluate each complete original source directly using the unchanged native202 counted-source driver. No surrounding eval/catch/puts or stack-size override is added. Actual version and exact public counted completion remain separate rows; executable/library/header/Makefile/source and actual compiler command pins are preserved.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
foreach x {1} {
set done 1
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
}
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
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
⟦EOF⟧
```
