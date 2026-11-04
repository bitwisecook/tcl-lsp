# Tcl counted-source execution evidence

## Question

> A callback selected by array unset can retarget its original source alias to another array. The whole-root and patterned-member operations require separate public observations across releases rather than a private-receiver inference from a shared source name.

> What four public array/member existence flags do original whole-root and patterned-member array-unset sources return when the array callback retargets held from a to b?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two original ASCII LF programs, independently evaluated in fresh fully initialised interpreters using the unchanged counted-source CAPI driver. The original source alias setup, array callback, explicit unpatterned/patterned unset words and four public queries remain byte-identical. Whole source/request/driver/compile/ELF/process/code/result/required pin artifacts are retained without an added source envelope or API substitution. No private header/frame/operand identity, general callback traversal, BIG-IP or executed Rust assertion is supplied. The shared software NativeArrayUnsetMemberLookup selector requires the actual C family, release and distinct-array storage purpose: C8.4/C8.5 selects OriginalName and C8.6/C9.0/C9.1 selects SelectedArray; missing release, mixed dictionary storage, Jim and hosted contexts refuse this C rule. Runtime and VM byte patterned-array-unset adapters consume that selected rule, while whole-root CmdCore unset keeps its separate original-name lookup after the array callback. These are current implementation policies; the public original356 flags remain a distinct observation and do not prove private lookup/table/header identities.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set a(x) OLD; set b(x) FOREIGN; upvar #0 a held; proc redirect args {uplevel #0 {upvar #0 b held}}; trace add variable a array redirect; array unset held; list [array exists a] [info exists a(x)] [array exists b] [info exists b(x)]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|31203120302030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 1 0 0⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|31203120302030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 1 0 0⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|31203120302030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 1 0 0⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|31203120302030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 1 0 0⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|31203120302030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 1 0 0⟦EOF⟧
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
