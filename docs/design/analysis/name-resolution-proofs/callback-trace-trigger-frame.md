# naming.callback.trace-trigger-frame

Kind: `native-observation`

## Problem statement

A trace prefix is installed in ::A while the operation that fires it runs in ::B, and matching prefix heads also exist at root. Resolving a prefix at installation can bind the wrong callback even when prefix bytes remain intact. Variable, command and execution trace purposes each need an actual triggering-frame observation.

## Question

Which namespace resolves unqualified prefix heads for the exact variable write, command rename and execution enter traces installed in ::A and triggered by ::B procedures?

## Conclusion

All five C providers resolve each of the three callback heads in ::B and report the triggering caller at level1 in ::B. The selected Jim guest rejects trace as unavailable for each case. These results establish the measured triggering lookup purposes; they do not establish arbitrary script equivalence, prefix storage, native argv objects or callback results.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source file through selected native CLI. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### jim

Status: `unsupported`. Channel: ASCII source file through selected native CLI. Version: jim 0.84-9-g5bac7c9.

trace 1 {invalid command name "trace"} {} | command-trace 1 {invalid command name "trace"} {} | execution-trace 1 {invalid command name "trace"} {}

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_callback_lookup_scope](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope)
