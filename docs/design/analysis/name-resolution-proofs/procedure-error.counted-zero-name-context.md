# naming.procedure-error.counted-zero-name-context

Kind: `native-observation`

## Problem statement

A counted original name contains p-zero-ignored while its command lookup/presentation can use a shorter native extent. Error context must be measured at that reached operation rather than derived from whole source bytes.

## Question

What procedure compile/call context is published for the original counted p-zero-ignored name?

## Conclusion

C8.4 creation succeeds and first call publishes missing quote, with p in the compilation and invocation context and errorCode NONE. The original input remains the full counted p-zero-ignored vector; no general zero-byte name comparator is inferred.

## Scope

Seven exact C8.4 public Tcl_EvalObjv TCL_EVAL_GLOBAL definition/call controls in fresh interpreters. Original name/formal/body are counted C objects; the result is retained and string-get before errorInfo/errorCode metadata getters, with no intervening Tcl script. Complete source/output/archive/observer digests and process0/empty stderr retained. Only C8.4 runs this purpose; later C, Jim and BIG-IP are not tested. No private primary/refcount, original return-options dictionary or successful prefix execution is sampled.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted Tcl_EvalObjv TCL_EVAL_GLOBAL; result-before-error-global metadata getter sequence. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Original case/definition-code/call-code/name-hex/body-hex/result-hex/errorInfo-hex/errorCode-hex:

```text
3	0	1	700069676e6f726564	7365742076616c75652022	6d697373696e672022	6d697373696e6720220a202020207768696c6520636f6d70696c696e670a227365742076616c756520220a2020202028636f6d70696c696e6720626f6479206f662070726f63202270222c206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227022	4e4f4e45
```
Decoded errorInfo per selected case:

Case 3:
```text
missing "
    while compiling
"set value "
    (compiling body of proc "p", line 1)
    invoked from within
"p"
```

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No original procedure parse-context capture for this provider is attached.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::c84_procedure_parse_failures_match_7_native_original_object_completions` (linked): Compares definition/call completion, exact result/errorInfo/errorCode. The extra no-side-store assertion is Rust coverage beyond the original native observer fields.

## Evidence

- [rust/tcl-vm/tests/data/native_procedure_parse_context/probe.c](../../../../rust/tcl-vm/tests/data/native_procedure_parse_context/probe.c)
- [rust/tcl-vm/tests/data/native_procedure_parse_context/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_procedure_parse_context/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_procedure_parse_context/manifest.json](../../../../rust/tcl-vm/tests/data/native_procedure_parse_context/manifest.json)
