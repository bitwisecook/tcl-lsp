# Tcl execution evidence

## Question

> Jim flat command lookup and namespace helpers can share written names with C namespace-tree operations while requiring different ownership purposes. Guest completion must be separated from any internal root holder or epoch claim.

> What completion and result does current Jim return for the exact original flat-namespace script combining canonical namespaces, command/procedure enumeration, imports, variable lookup, lexical variables and alias origin?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One original fixed ASCII LF source-file CLI process on the recorded current Jim executable. C Tcl and BIG-IP are not measured for this question. No raw-zero/opaque object-vector input, compilation admission, internal root context liveness or arbitrary namespace behavior is inferred. The original generic receipt input_channel label mentions scalar trace callbacks; the exact probe, command vector and boundary field establish this namespace-script channel instead.

## Script

```tcl-visible
puts "PATCHLEVEL [info patchlevel]"
set code [catch {
proc p {} {return ROOT}; namespace eval n {proc p {} {return INNER}; namespace eval child {proc q {} {return CHILD}}}; namespace eval D {namespace import ::n::p}; list [namespace canonical] [namespace eval n {namespace canonical}] [lsort [namespace eval n {info procs *}]] [namespace eval n {info commands p}] [lsort [namespace eval n {info procs ::n::*}]] [namespace eval n {namespace which -variable absent}] [namespace eval n {set value LOCAL; namespace eval child {set value CHILD}; set value}] [D::p] [namespace origin D::p]
} value]
binary scan $value H* hex
puts "OBSERVATION $code $hex"
```

## Results

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.
- Exit status: 0
- Standard output:

```text-visible
PATCHLEVEL 0.84-9-g5bac7c9
OBSERVATION 0 7b7d206e207b6368696c643a3a7120707d2070207b3a3a6e3a3a6368696c643a3a71203a3a6e3a3a707d203a3a6e3a3a616273656e74204c4f43414c20494e4e4552203a3a6e3a3a70
```

- Standard error: empty.
