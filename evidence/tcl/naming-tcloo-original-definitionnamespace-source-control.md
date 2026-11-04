# Tcl execution evidence

## Question

> Definition-namespace configuration and its public query are separate availability/qualification questions; version labels cannot supply their result.

> What exact public set/query completions does the original TclOO definitionnamespace source report?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames.

## Script

```tcl-visible
set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    namespace eval foodef {}
    oo::class create Meta
    set code [catch {oo::define Meta {definitionnamespace foodef}} value]
    set query [catch {info class definitionnamespace Meta} stored]
    list $code $value $query $stored
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|4e4f545f4150504c494341424c452073746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4e4f545f4150504c494341424c452073746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|31207b696e76616c696420636f6d6d616e64206e616d652022646566696e6974696f6e6e616d657370616365227d2031207b756e6b6e6f776e206f7220616d626967756f757320737562636f6d6d616e642022646566696e6974696f6e6e616d657370616365223a206d7573742062652063616c6c2c20636f6e7374727563746f722c20646566696e6974696f6e2c2064657374727563746f722c2066696c746572732c20666f72776172642c20696e7374616e6365732c206d6574686f64732c206d6574686f64747970652c206d6978696e732c20737562636c61737365732c207375706572636c61737365732c206f72207661726961626c65737d
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|30207b7d2030203a3a666f6f646566
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|30207b7d2030203a3a666f6f646566
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|4e4f545f4150504c494341424c452073746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.
