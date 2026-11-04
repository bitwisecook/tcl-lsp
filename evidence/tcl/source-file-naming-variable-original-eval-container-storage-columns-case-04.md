# Tcl source-file execution evidence

## Question

> The retained storage expectations describe ten exact public array/scalar, alias and dictionary-mapping specimens. Complete isolated native process receipts are required to distinguish these fixed output columns from an independently failing Runtime implementation or a different source envelope.

> What do the six pinned providers return for all ten original variable-container specimens in their exact isolated puts-[eval]-source-file envelope, and do these captured rows match the existing expectation columns byte for byte?

This is one source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

> Ten ASCII LF programs use puts with an eval script and run in separate fresh processes. The process does not print its own version; the recorded version is associated with a separate version capture for the executable. Raw NUL, non-ASCII, continuation and control escapes are not exercised. Any dictionary absence applies only to the captured guarded stock Tcl 8.4 branch. Private frame/object identity, general storage equivalence and Rust assertion outcomes are not established.

Other assumptions were not separately recorded.

## Script

```tcl-visible
puts [eval {array set a {k OLD};upvar 0 a root;set c [catch {set root NEW} r];list $c $r [array exists a]}]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
1 {can't set "root": variable is array} 1
```

- Standard error: empty.

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
1 {can't set "root": variable is array} 1
```

- Standard error: empty.

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
1 {can't set "root": variable is array} 1
```

- Standard error: empty.

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
1 {can't set "root": variable is array} 1
```

- Standard error: empty.

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
1 {can't set "root": variable is array} 1
```

- Standard error: empty.

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: source file passed to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
0 NEW 0
```

- Standard error: empty.
