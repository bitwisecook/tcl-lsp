# Tcl execution evidence

## Question

> Constant enumeration, global/upvar links and formal shadowing have distinct source/storage roles. Public introspection must retain the exact original controls and selected provider.

> What public caught values does the original TclOO constant-link introspection source return for inspect, retarget and formal-shadow methods?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames. This constructed FrameStack/Namespaces fixture does not execute the original TclOO inspect/retarget/formal-shadow source, prove native constant-link identity or grant a physical procedure/object frame. Exact original public caught values remain independent. Source linkage supplies no executable availability or assertion outcome.

## Script

```tcl-visible
if {![llength [info commands const]] || ![llength [info commands oo::class]]} {
    list NOT_APPLICABLE const-or-stock-TclOO-unavailable
} else {
    const G 9
    oo::class create C {variable X; constructor {} {const X 1}; method inspect {} {global G; upvar #0 ::G U; list [info consts] [info constant G] [info constant U]}; method retarget {} {upvar #0 ::G X; list [info consts] [info constant X]}; method shadow {X} {list $X [info consts]}}
    set o [C new]
    set a [catch {$o inspect} av]
    set b [catch {$o retarget} bv]
    set c [catch {$o shadow formal} cv]
    list $a $av $b $bv $c $cv
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
ORIGINAL|0|4e4f545f4150504c494341424c4520636f6e73742d6f722d73746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|4e4f545f4150504c494341424c4520636f6e73742d6f722d73746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|4e4f545f4150504c494341424c4520636f6e73742d6f722d73746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|30207b58203120317d2030207b7b7d20317d2030207b666f726d616c207b7d7d
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|30207b58203120317d2030207b7b7d20317d2030207b666f726d616c207b7d7d
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|0|4e4f545f4150504c494341424c4520636f6e73742d6f722d73746f636b2d54636c4f4f2d756e617661696c61626c65
```

- Standard error: empty.
