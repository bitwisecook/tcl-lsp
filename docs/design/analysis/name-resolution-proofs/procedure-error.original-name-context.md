# naming.procedure-error.original-name-context

Kind: `native-observation`

## Problem statement

A procedure body parse error reports the original invocation name in both compilation and call context. Quoted space and raw FF names can be corrupted if a consumer first decodes/rebuilds their display.

## Question

What completion, errorInfo and errorCode bytes follow first invocation of malformed bodies under simple, spaced and raw-FF original procedure names?

## Conclusion

All three C8.4 definitions succeed, calls fail with missing quote, errorCode NONE and the exact original compilation/invocation name context. Space is list-quoted in the invocation and FF remains counted in the context. These are error publication fields, not a physical name-header proof.

## Scope

Seven exact C8.4 public Tcl_EvalObjv TCL_EVAL_GLOBAL definition/call controls in fresh interpreters. Original name/formal/body are counted C objects; the result is retained and string-get before errorInfo/errorCode metadata getters, with no intervening Tcl script. Complete source/output/archive/observer digests and process0/empty stderr retained. Only C8.4 runs this purpose; later C, Jim and BIG-IP are not tested. No private primary/refcount, original return-options dictionary or successful prefix execution is sampled.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted Tcl_EvalObjv TCL_EVAL_GLOBAL; result-before-error-global metadata getter sequence. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Original case/definition-code/call-code/name-hex/body-hex/result-hex/errorInfo-hex/errorCode-hex:

```text
0	0	1	70	7365742076616c75652022	6d697373696e672022	6d697373696e6720220a202020207768696c6520636f6d70696c696e670a227365742076616c756520220a2020202028636f6d70696c696e6720626f6479206f662070726f63202270222c206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227022	4e4f4e45
1	0	1	702071	7365742076616c75652022	6d697373696e672022	6d697373696e6720220a202020207768696c6520636f6d70696c696e670a227365742076616c756520220a2020202028636f6d70696c696e6720626f6479206f662070726f632022702071222c206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a227b7020717d22	4e4f4e45
2	0	1	70ff	7365742076616c75652022	6d697373696e672022	6d697373696e6720220a202020207768696c6520636f6d70696c696e670a227365742076616c756520220a2020202028636f6d70696c696e6720626f6479206f662070726f63202270ff222c206c696e652031290a20202020696e766f6b65642066726f6d2077697468696e0a2270ff22	4e4f4e45
```
Decoded errorInfo per selected case:

Case 0:
```text
missing "
    while compiling
"set value "
    (compiling body of proc "p", line 1)
    invoked from within
"p"
```

Case 1:
```text
missing "
    while compiling
"set value "
    (compiling body of proc "p q", line 1)
    invoked from within
"{p q}"
```

Case 2:
```text
missing "
    while compiling
"set value "
    (compiling body of proc "p\xff", line 1)
    invoked from within
"p\xff"
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
