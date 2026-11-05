# naming.variable.empty-vector-reset-result-and-error-episode

Kind: `native-observation`

## Problem statement

An empty original argv vector resets the result even without invoking a command. Shared and unshared result headers can take different replacement paths, while the previous public error episode remains a separate object lifetime.

## Question

What does Tcl_EvalObjv with zero argv do to shared/unshared integer result primaries and an existing errorInfo episode?

## Conclusion

All five captured C releases return OK for the empty vector and an empty untyped result. The unshared original result is reused and the retained shared original is replaced; the separately held errorInfo header remains the same. This is the zero-vector evaluation door, not a general command result, native header grant or Jim policy.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Version: tcl8.4 8.4.20.

Observed capture for 8.4.20:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl8.5

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Version: tcl8.5 8.5.19.

Observed capture for 8.5.19:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl8.6

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Version: tcl8.6 8.6.18.

Observed capture for 8.6.18:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl9.0

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Version: tcl9.0 9.0.4.

Observed capture for 9.0.4:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl9.1

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Version: tcl9.1 9.1.0.

Observed capture for 9.1.0:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `empty_public_vectors_reset_original_result_owners_without_dispatch`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `interp::native_object_vector::tests::empty_public_vectors_reset_original_result_owners_without_dispatch` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

## Evidence

- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-manifest.json)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset.c](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset.c)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.4.20.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.5.19.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.6.18.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.0.4.tsv)
- [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.1.0.tsv)
