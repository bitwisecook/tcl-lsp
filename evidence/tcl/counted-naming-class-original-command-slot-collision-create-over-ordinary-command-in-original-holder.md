# Tcl counted-source execution evidence

## Question

> A namespace-relative object creation can encounter an existing ordinary procedure. Reporting a similarly rendered object label cannot establish that the original destination command was free or replaced.

> When original relative TclOO creation targets p occupied by an ordinary procedure in current a:, what caught error, subsequent procedure result and command query are observed?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Two unchanged ASCII LF sources execute separately under full original provider initialisation and counted C API evaluation. The creation collision is a reached caught guest error; the copy-labelled input retains an independent setup failure before copying. All original input/requests/drivers/actual commands/ELFs/whole streams/receipts and required provider/archive/header/build/source pins are retained. Independently reached copy inputs have their own original-copy-holder-and-collision record. BIG-IP and Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass. The fixture selects an actual software core/compiler and compares public original bytes, without launching new original processes or observing private native token/header/cache identity. Constructed-holder reporting and collision results remain separate original questions. No passing software outcome, oo::copy availability, copy permission, activation/frame or compiler instruction admission is attached.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
if {![llength [info commands ::oo::object]]} {
    list NOT_APPLICABLE
} else {
    namespace eval a: {
        proc p {} {return ORIGINAL_PROC}
        set code [catch {::oo::object create p} result]
        list $code $result [p] [info object isa object p]
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
ORIGINAL|0|31207b63616e277420637265617465206f626a656374202270223a20636f6d6d616e6420616c72656164792065786973747320776974682074686174206e616d657d204f524947494e414c5f50524f432030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|31207b63616e277420637265617465206f626a656374202270223a20636f6d6d616e6420616c72656164792065786973747320776974682074686174206e616d657d204f524947494e414c5f50524f432030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|31207b63616e277420637265617465206f626a656374202270223a20636f6d6d616e6420616c72656164792065786973747320776974682074686174206e616d657d204f524947494e414c5f50524f432030
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|4e4f545f4150504c494341424c45
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
NOT_APPLICABLE⟦EOF⟧
```
