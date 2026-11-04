# Tcl counted-source execution evidence

## Question

> A final failing source completion cannot locate which precision read failed or report later reads. Root unset, linked unset and an unrelated same-tail variable require their own caught public observations.

> What exact existence and caught read results follow root or linked unset of precision, and how does an unrelated namespace variable with the same tail behave?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three exact original ASCII LF sources run independently in fresh full provider initialisation through the unchanged counted C API driver. Each source explicitly sets precision12; linked unset uses an original upvar before independently caught reads and unrelated namespace storage stays separate. Whole original inputs, actual commands/receipts/streams/ELFs and required provider/header/archive/source/build pins are retained. The independent unguarded original source and all actual provider receipts remain under limited-control with their measured final guest errors; no failing operand is inferred from an uncaught final completion.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set ::tcl_precision 12
proc linked {} {
 upvar #0 ::tcl_precision p
 unset p
 set aliasExists [info exists p]
 set rootExists [info exists ::tcl_precision]
 set aliasCode [catch {set p} aliasValue]
 set rootCode [catch {set ::tcl_precision} rootValue]
 set reportedCode [catch {set ::p} reportedValue]
 list $aliasExists $rootExists $aliasCode $aliasValue $rootCode $rootValue $reportedCode $reportedValue
}
linked
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2030203132
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 0 12⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2030203132
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 0 12⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2030203132
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 0 12⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a70223a206e6f2073756368207661726961626c657d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 1 {can't read "::p": no such variable}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a70223a206e6f2073756368207661726961626c657d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 1 {can't read "::p": no such variable}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|3020302031207b63616e27742072656164202270223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a74636c5f707265636973696f6e223a206e6f2073756368207661726961626c657d2031207b63616e2774207265616420223a3a70223a206e6f2073756368207661726961626c657d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 1 {can't read "::p": no such variable}⟦EOF⟧
```
