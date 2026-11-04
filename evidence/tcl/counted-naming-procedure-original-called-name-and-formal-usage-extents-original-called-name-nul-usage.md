# Tcl counted-source execution evidence

## Question

> Original called-name and formal name values can contain counted NUL independently of how a wrong-argument usage is assembled and rendered. Required, default and reference-shaped formals require separate public definition/call outcomes rather than a single diagnostic extent inferred from name storage.

> What exact original name/formal lengths, definition completion/results and caught wrong-argument usage do four unchanged NUL-producing called-name, required-formal, default-formal and reference-formal sources return on the six pinned interpreters?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Four byte-identical ASCII LF sources, each independently evaluated in a fresh fully initialised interpreter using the unchanged counted-source driver. binary format H* produces literal NUL at evaluation, with no literal NUL/non-ASCII source or added source envelope. Whole public result fields retain original counted lengths, definition completion/result and conditional caught call completion/message; no definition error is replaced with an availability branch. Complete driver/request/source/compiler/ELF/process/stdout/stderr/version and required provider pins are retained. Source anchors are independently hashed read-only channels. No opaque source AST, physical object/refcount, private lookup/frame/formal cell, arbitrary UTF-8/FF name, instruction/admission or executed Rust assertion is supplied. The shared formal_parameter_usage_bytes implementation preserves counted declaration/storage names separately from displayed usage. Jim required and reference formals use a CString extent per required name and retain later formals; a default formal keeps its counted suffix until the separate final message formatter. Runtime and VM use that suffix with the selected called-name and final-message owners. Their linked controls compare all twenty-four unchanged original public completion/results independently in each backend. Explicit Jim binary capability supplies only the original byte-producing source dependency. The pure suffix control and comparison definitions supply no native formal object/header/table/cache/compiler/activation identity or current executed Rust result. No new original process, compiled opcode, physical formal/header/table identity, private CString stage, source-character channel or software passing outcome is observed by this binding. The C original definitions/invocations retain their actual APIs rather than an unavailable surrogate. Selected compiler and explicit Jim binary adapter are software fixture premises, not general Native entry or library authority.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set r2286_name [binary format H* 72323238365f63616c6c0068656164]
set r2286_formal x
set r2286_parameters [list $r2286_formal]
set r2286_definition_code [catch {proc $r2286_name $r2286_parameters {return VALUE}} r2286_definition]
if {$r2286_definition_code == 0} {
    set r2286_call_code [catch {$r2286_name} r2286_message]
} else {
    set r2286_call_code NOT_CALLED
    set r2286_message {}
}
list [string length $r2286_name] [string length $r2286_formal] $r2286_definition_code $r2286_definition $r2286_call_code $r2286_message
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|313520312030207b7d2031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6cc080686561642078227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 {} 1 {wrong # args: should be "r2286_call⟦BYTE:C0⟧⟦BYTE:80⟧head x"}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|313520312030207b7d2031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6cc080686561642078227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 {} 1 {wrong # args: should be "r2286_call⟦BYTE:C0⟧⟦BYTE:80⟧head x"}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|313520312030207b7d2031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6cc080686561642078227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 {} 1 {wrong # args: should be "r2286_call⟦BYTE:C0⟧⟦BYTE:80⟧head x"}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|313520312030207b7d2031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6cc080686561642078227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 {} 1 {wrong # args: should be "r2286_call⟦BYTE:C0⟧⟦BYTE:80⟧head x"}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|313520312030207b7d2031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6cc080686561642078227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 {} 1 {wrong # args: should be "r2286_call⟦BYTE:C0⟧⟦BYTE:80⟧head x"}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|3135203120302072323238365f63616c6c00686561642031207b77726f6e67202320617267733a2073686f756c64206265202272323238365f63616c6c227d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
15 1 0 r2286_call⟦U+0000⟧head 1 {wrong # args: should be "r2286_call"}⟦EOF⟧
```
