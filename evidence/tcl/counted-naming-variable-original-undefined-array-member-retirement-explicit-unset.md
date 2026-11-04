# Tcl counted-source execution evidence

## Question

> An unset trace can be registered on an array member without assigning that member a value. A defined-value enumeration cannot by itself answer whether the original trace is called when the array is unset, its procedure returns, or its namespace is deleted.

> What original array/member existence pair and callback log do three independent sources return when an unset trace is registered on an undefined array member and the array is explicitly unset, its procedure frame exits, or its namespace is deleted?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three ASCII LF sources were evaluated independently in fresh fully initialized interpreters through the counted-source C API driver. Defined-value queries, arbitrary trace registration order, private object identity, BIG-IP and executed Rust controls were outside this observation.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set log {}; proc member args {lappend ::log MEMBER}; trace add variable a(x) unset member; set before [list [array exists a] [info exists a(x)]]; unset a; list $before $log
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b3120307d204d454d424552
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{1 0} MEMBER⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b3120307d204d454d424552
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{1 0} MEMBER⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b3120307d204d454d424552
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{1 0} MEMBER⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b3120307d204d454d424552
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{1 0} MEMBER⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b3120307d204d454d424552
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{1 0} MEMBER⟦EOF⟧
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
