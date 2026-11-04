# Tcl execution evidence

## Question

> A live TclOO object name does not establish a class suitable for superclass or mixin configuration. Each exact definition operation needs its own public rejection result.

> What public errors do the original superclass and mixin controls report when their operand is an ordinary TclOO object?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames.

## Script

```tcl-visible
set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    oo::object create anobj
    set first [catch {oo::class create BadSuper {superclass anobj}} firstValue]
    set second [catch {oo::class create BadMixin {mixin anobj}} secondValue]
    list $first $firstValue $second $secondValue
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
ORIGINAL|0|31207b6f6e6c79206120636c6173732063616e2062652061207375706572636c6173737d2031207b6d6179206f6e6c79206d697820696e20636c61737365737d
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|31207b6f6e6c79206120636c6173732063616e2062652061207375706572636c6173737d2031207b6d6179206f6e6c79206d697820696e20636c61737365737d
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|31207b6f6e6c79206120636c6173732063616e2062652061207375706572636c6173737d2031207b6d6179206f6e6c79206d697820696e20636c61737365737d
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
