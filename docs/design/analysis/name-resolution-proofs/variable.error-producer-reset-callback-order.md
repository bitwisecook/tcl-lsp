# naming.variable.error-producer-reset-callback-order

Kind: `native-observation`

## Problem statement

The actual error worker retains original info/code operands differently from visible globals, and reset publishes those objects through write traces. Callback-time additional references cannot be inferred from pre-reset values or script-trace behaviour.

## Question

Which original ByteArray info/code objects does the error worker retain, and what do direct native write traces see during reset?

## Conclusion

For the measured direct-worker ff 00 61 operands, C8.5+ retains the info object and validates the same code object into a List. Direct native reset callbacks observe code publication before info and an additional saved-interpreter-state owner. C8.4 uses its separately recorded global path. These windows do not apply to script callbacks or prove source-analysis observer closure.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Version: tcl8.4 8.4.20.

Observed capture for 8.4.20:
default-code	privateinfo=null	privatecode=null	info_same=1	code_same=1	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=list	global1_same=0
error_code=1
explicit-error	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=1	code_refs=2	global0=string	global0_same=0	global1=bytearray	global1_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=1	code_refs=2	global0=string	global0_same=0	global1=bytearray	global1_same=1

### tcl8.5

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Version: tcl8.5 8.5.19.

Observed capture for 8.5.19:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl8.6

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Version: tcl8.6 8.6.18.

Observed capture for 8.6.18:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl9.0

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Version: tcl9.0 9.0.4.

Observed capture for 9.0.4:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl9.1

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Version: tcl9.1 9.1.0.

Observed capture for 9.1.0:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `original_private_error_primaries_and_script_reset_match_native_producers`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::original_private_error_primaries_and_script_reset_match_native_producers` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_error_variables/producers-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/producers-manifest.json)
- [rust/tcl-registry/tests/data/native_error_variables/producers.c](../../../../rust/tcl-registry/tests/data/native_error_variables/producers.c)
- [rust/tcl-registry/tests/data/native_error_variables/8.4.20-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20-producers.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv)
