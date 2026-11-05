# naming.variable.error-info-append-original-object

Kind: `native-observation`

## Problem statement

Appending error context can preserve an original result object for an empty append yet convert or replace it for a nonempty append. Bytes alone cannot certify the original ByteArray header or private/public reference ownership.

## Question

How do empty and nonempty Tcl_AddObjErrorInfo appends and reset affect the original ByteArray result and private/global error objects?

## Conclusion

The original ff 00 61 ByteArray, empty append, eight-byte context append and reset have distinct measured object/header windows. The retained rows determine sameness and reference counts for each C release; they do not establish an arbitrary string mutation or physical identity from equal bytes.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Version: tcl8.4 8.4.20.

Observed capture for 8.4.20:
empty-append	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=3	code_refs=-1	global0=bytearray	global0_same=1	global1=string	global1_same=0
nonempty-append	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=2	code_refs=-1	global0=string	global0_same=0	global1=string	global1_same=0
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=string	global1_same=0

### tcl8.5

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Version: tcl8.5 8.5.19.

Observed capture for 8.5.19:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl8.6

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Version: tcl8.6 8.6.18.

Observed capture for 8.6.18:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl9.0

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Version: tcl9.0 9.0.4.

Observed capture for 9.0.4:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl9.1

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Version: tcl9.1 9.1.0.

Observed capture for 9.1.0:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `error_info_byte_append_preserves_seed_primary_until_nonempty_mutation`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::error_info_byte_append_preserves_seed_primary_until_nonempty_mutation` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_error_variables/info-append-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/info-append-manifest.json)
- [rust/tcl-registry/tests/data/native_error_variables/info-append.c](../../../../rust/tcl-registry/tests/data/native_error_variables/info-append.c)
- [rust/tcl-registry/tests/data/native_error_variables/8.4.20-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20-info-append.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.5.19-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-info-append.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/8.6.18-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-info-append.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.0.4-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-info-append.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/9.1.0-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-info-append.tsv)
