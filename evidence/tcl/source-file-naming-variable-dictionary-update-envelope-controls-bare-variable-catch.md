# Tcl source-file execution evidence

## Question

> A dictionary update with a scalar first mapping and array-valued second destination has both an availability guard and an evaluation envelope. The exact original guarded eval form, a bare eval form and two catch-$script forms require independent public observations before attributing a result to either distinction.

> Do the dict-availability guard and eval versus catch-$script envelope change the public array-second dict update tuple in the four exact recorded source-file controls?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> Four separate ASCII LF scripts compare guarded versus bare bodies and eval versus catch of a script value. Each runs in a fresh process. The processes do not print their versions; the recorded version is associated with a separate version capture for the executable. Raw NUL, non-ASCII, continuation and control escapes, private native identities and general compiler or editor equivalence are not tested.

Other assumptions were not separately recorded.

## Script

```tcl-visible
set original {array set blocked {k OLD};set d {first NEW second OTHER};set ok BEFORE;set entered 0;set c [catch {dict update d first ok second blocked {set entered 1}} r];list $c $ok $entered [catch {set blocked} v] $v}
set outerCode [catch $original outerResult]
puts [list $outerCode $outerResult]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {1 BEFORE 0 1 {can't read "blocked": variable is array}}
```

- Standard error: empty.

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {1 NEW 0 1 {can't read "blocked": variable is array}}
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {1 NEW 0 1 {can't read "blocked": variable is array}}
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {1 NEW 0 1 {can't read "blocked": variable is array}}
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {1 NEW 0 1 {can't read "blocked": variable is array}}
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 {0 NEW 1 0 OTHER}
```

- Standard error: empty.
