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

Status: `observed`. Version: 8.4.20. Build: Captured library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Dialect: Tcl.

Observed capture for 8.4.20:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Captured library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Dialect: Tcl.

Observed capture for 8.5.19:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Captured library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Dialect: Tcl.

Observed capture for 8.6.18:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Captured library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Dialect: Tcl.

Observed capture for 9.0.4:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Captured library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by empty-vector-reset.c. Dialect: Tcl.

Observed capture for 9.1.0:
0	0	1	1	1	none	1	0
1	0	0	1	1	none	1	0
episode	1	0	1	2

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-manifest.json). SHA-256 `b3a659666e6cbe6bbb60152b5e29442088dddea7748258fe51ad0de43891b71e`. Exact original build and capture receipt, including all observed engine versions and source/library/output hashes.
- `probe` (input): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset.c](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset.c). SHA-256 `bf7c221f896cdc45ee564180fbf9d7c3237cd14d452be42737860d241e843d58`. Original API calls, reporting order, object retention and internal root-cell observation windows.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.4.20.tsv). SHA-256 `69480e2f7e563a1b67ab0d916d6ae5ef99424c91b744d6b42c113d26e93ab1c0`. Exact native result/state/callback rows for 8.4.20
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.5.19.tsv). SHA-256 `69480e2f7e563a1b67ab0d916d6ae5ef99424c91b744d6b42c113d26e93ab1c0`. Exact native result/state/callback rows for 8.5.19
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-8.6.18.tsv). SHA-256 `69480e2f7e563a1b67ab0d916d6ae5ef99424c91b744d6b42c113d26e93ab1c0`. Exact native result/state/callback rows for 8.6.18
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.0.4.tsv). SHA-256 `69480e2f7e563a1b67ab0d916d6ae5ef99424c91b744d6b42c113d26e93ab1c0`. Exact native result/state/callback rows for 9.0.4
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset-9.1.0.tsv). SHA-256 `69480e2f7e563a1b67ab0d916d6ae5ef99424c91b744d6b42c113d26e93ab1c0`. Exact native result/state/callback rows for 9.1.0

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `empty_public_vectors_reset_original_result_owners_without_dispatch`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `interp::native_object_vector::tests::empty_public_vectors_reset_original_result_owners_without_dispatch` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-std=c99",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/tcl9.0.4/generic",
  "-I/path/tcl9.0.4/unix",
  "rust/tcl-registry/tests/data/native_error_variables/empty-vector-reset.c",
  "/path/tcl9.0.4/unix/libtcl9.0.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/naming-error-variable-probe"
]
```

Build against each exact manifest version/header/library and run the produced executable in a fresh process. Require completed exit 0 and exact full stdout equality with the corresponding evidence TSV (reset episodes excludes only the aggregate version prefix/header). Match retained source/library/header SHA before interpreting any difference. Original captures are retained; a new execution/build availability is not claimed.
