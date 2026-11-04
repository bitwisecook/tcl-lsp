# Tcl counted-source execution evidence

## Question

> An array-unset operation can reach a member or root callback that refills the same public array and installs a fresh unset registration. Complete post-callback queries and the next operation are needed to establish the registration’s finite public behaviour.

> What refill value, public trace inventory and final callback log do two original sources return when array unset selects an individual member or whole root whose callback refills it and installs a fresh unset trace?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two ASCII LF sources were evaluated independently in fresh fully initialized interpreters through the counted-source C API driver, without an added source envelope or provider API substitution. Arbitrary trace registration order, private identity, BIG-IP and executed software controls were not tested.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set log {}; proc fresh args {lappend ::log FRESH}; proc old args {lappend ::log OLD; set ::a(x) REFILL; trace add variable ::a(x) unset fresh}; set a(x) START; upvar #0 a(x) saved; trace add variable a(x) unset old; array unset a x; set before [list $a(x) [trace info variable a(x)]]; unset a(x); list $before $log
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b524546494c4c207b7b756e7365742066726573687d7d7d207b4f4c442046524553487d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{REFILL {{unset fresh}}} {OLD FRESH}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b524546494c4c207b7b756e7365742066726573687d7d7d207b4f4c442046524553487d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{REFILL {{unset fresh}}} {OLD FRESH}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b524546494c4c207b7b756e7365742066726573687d7d7d207b4f4c442046524553487d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{REFILL {{unset fresh}}} {OLD FRESH}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b524546494c4c207b7b756e7365742066726573687d7d7d207b4f4c442046524553487d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{REFILL {{unset fresh}}} {OLD FRESH}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b524546494c4c207b7b756e7365742066726573687d7d7d207b4f4c442046524553487d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{REFILL {{unset fresh}}} {OLD FRESH}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "trace"⟦EOF⟧
```
