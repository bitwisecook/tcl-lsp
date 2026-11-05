# naming.procedure-error.body-line-context

Kind: `native-observation`

## Problem statement

A valid first body command followed by a malformed quote can produce a compile-line2 error independently of any successful execution of that prefix. Error location alone cannot certify that the prefix store ran.

## Question

Which compile line and original call context are published for set side READY followed by the malformed quote command?

## Conclusion

C8.4 creation succeeds and first call returns missing quote with compilation body line2 and full original long-name invocation context, errorCode NONE. The observer does not read side; the separate Rust assertion that no prefix store occurs is an implementation obligation beyond these captured fields.

## Scope

Seven exact C8.4 public Tcl_EvalObjv TCL_EVAL_GLOBAL definition/call controls in fresh interpreters. Original name/formal/body are counted C objects; the result is retained and string-get before errorInfo/errorCode metadata getters, with no intervening Tcl script. Complete source/output/archive/observer digests and process0/empty stderr retained. Only C8.4 runs this purpose; later C, Jim and BIG-IP are not tested. No private primary/refcount, original return-options dictionary or successful prefix execution is sampled.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted Tcl_EvalObjv TCL_EVAL_GLOBAL; result-before-error-global metadata getter sequence. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Original case/definition-code/call-code/name-hex/body-hex/result-hex/errorInfo-hex/errorCode-hex:

```text
6	0	1	70707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070c3a9717171717171717171	73657420736964652052454144590a7365742076616c75652022	6d697373696e672022	6d697373696e6720220a202020207768696c6520636f6d70696c696e670a227365742076616c756520220a2020202028636f6d70696c696e6720626f6479206f662070726f632022707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070702e2e2e222c206c696e652032290a20202020696e766f6b65642066726f6d2077697468696e0a2270707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070c3a971717171717171717122	4e4f4e45
```
Decoded errorInfo per selected case:

Case 6:
```text
missing "
    while compiling
"set value "
    (compiling body of proc "ppppppppppppppppppppppppppppppppppppppppppppppppp...", line 2)
    invoked from within
"pppppppppppppppppppppppppppppppppppppppppppppppppéqqqqqqqqq"
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
