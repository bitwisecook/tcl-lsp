# Tcl execution evidence

## Question

> Definition-context self commands have distinct class/object/release purposes; a familiar method name does not determine an accepted worker or argument grammar.

> What public caught results do the exact zero-argument self commands return in class and object definition bodies on each pinned provider?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames.

## Script

```tcl-visible
set available [expr {[llength [info commands oo::class]] > 0}]
if {!$available} {list NOT_APPLICABLE stock-TclOO-unavailable} else {
    oo::class create Cls
    Cls create obj
    set c [catch {oo::define Cls {self}} value]
    set o [catch {oo::objdefine obj {self}} object]
    list $c $value $o $object
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
ORIGINAL|0|31207b77726f6e67202320617267733a2073686f756c64206265202273656c6620617267203f617267202e2e2e3f227d2031207b696e76616c696420636f6d6d616e64206e616d65202273656c66227d
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|30203a3a436c732030203a3a6f626a
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|30203a3a436c732030203a3a6f626a
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
