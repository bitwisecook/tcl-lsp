# Tcl counted-source execution evidence

## Question

> Array teardown callbacks can change member trace registrations through retained aliases or a same-spelled root. Public callback logs are needed to distinguish those original programs without assuming a universal snapshot rule or private member identity.

> What whole public callback logs do the three original array teardown programs return when a root or member callback changes registrations through retained aliases or the original root spelling?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Three original ASCII LF programs, each in a fresh fully initialised provider interpreter, use the unchanged counted-source CAPI driver without a source catch/eval/puts envelope. Six compile receipts and eighteen whole process receipts/streams preserve exact source, executable, archive, header, build and required-source pins. Original source names and request statements are retained, including anticipated outcomes separately from measured answers. No opaque/Unicode/NUL trace-name input, private member identity, BIG-IP or executed Rust comparison is measured. The linked Runtime comparison consumes all eighteen whole untouched original public completions/results, including Jim unsupported trace. Separate software VarTraceScope ownership uses the selected root and original element binding identity; retained aliases use that selected generation, the original member stays live during root and earlier-member callbacks; at its own retirement horizon the then-current registration group is captured and detached by immutable registration IDs before its unset callbacks. Changes before that horizon can affect the group, while same-spelled replacement-member registrations cannot. Cleanup removes only the captured original IDs. The Rust member-correspondence control does not turn public logs into provider member/cell/header identities, and linked status claims no executed software pass. NativeElementAliasEntry::permits_trace_registration admits only the exact actively retiring original entry for trace-list registration. Ordinary is_live value access remains refused at that point, and registration permission ends with the active retirement receipt. Trace resolver/ensure consumers retain this purpose-specific entry rather than revive retired alias reads or writes. The linked software allocation control proves only this implementation capability boundary; public original343 callback logs do not identify private entries or registration IDs. The definition may discover no references when the all-oracles requirement is absent, so source linkage establishes neither a provider count nor a passing native comparison. It checks public callback completion/bytes, not original private member/cell/header identity, physical refcount lifetime or Jim behavior. The eighteen original whole process windows remain independently measured and unchanged; no fresh outcome is attached.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set a(x) X; set a(y) Y; upvar 0 a(x) xx; upvar 0 a(y) yy; set log {}; proc fresh args {lappend ::log NEW}; proc old args {lappend ::log OLD; trace add variable ::xx unset fresh; trace add variable ::yy unset fresh}; trace add variable a(x) unset old; trace add variable a(y) unset old; unset a; set log
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|4f4c44204e4557204f4c44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD NEW OLD⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4f4c44204e4557204f4c44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD NEW OLD⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4f4c44204e4557204f4c44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD NEW OLD⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|4f4c44204e4557204f4c44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD NEW OLD⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|4f4c44204e4557204f4c44
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
OLD NEW OLD⟦EOF⟧
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
