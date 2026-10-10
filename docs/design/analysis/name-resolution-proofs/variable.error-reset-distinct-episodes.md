# naming.variable.error-reset-distinct-episodes

Kind: `native-observation`

## Problem statement

Resetting a result need not destroy a retained public errorInfo object, but a later shorter error must start a new clean episode. A retained object receipt cannot be treated as current episode storage merely because it survived reset and an unrelated successful definition.

## Question

Does errorInfo retain its original header across reset/successful definition, and do later shorter/repeated errors form clean distinct episodes?

## Conclusion

All five captured C releases preserve the retained errorInfo header across reset and the intervening successful definition. The subsequent X error uses a fresh header without FIRST_ERROR_LONG, and the repeated error remains clean. This does not certify arbitrary future cell currency, Jim reset semantics or observer-free error handling.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Captured library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Dialect: Tcl.

Observed capture for 8.4.20:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.4.20	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Captured library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Dialect: Tcl.

Observed capture for 8.5.19:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.5.19	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Captured library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Dialect: Tcl.

Observed capture for 8.6.18:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.6.18	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Captured library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Dialect: Tcl.

Observed capture for 9.0.4:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 9.0.4	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Captured library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Dialect: Tcl.

Observed capture for 9.1.0:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 9.1.0	0	1	1	2	0	1	2	1	1	1	1	1	1

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes-manifest.json). SHA-256 `38b32ca3b24b6ab3f53f4c851e18e9719f30b4fa1ed652b4b4b6441e021d9e18`. Exact original build and capture receipt, including all observed engine versions and source/library/output hashes.
- `probe` (input): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.c](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.c). SHA-256 `ba54cc68a231bff05900bcad3b890882e5971e81dc24208efd45564652845199`. Original API calls, reporting order, object retention and internal root-cell observation windows.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv). SHA-256 `9646fcdfe5c93cbd36f3b4c6ec03f3dffe724a86fedaaa20e6e787ca98a0f4f8`. Exact native result/state/callback rows for 8.4.20
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv). SHA-256 `9646fcdfe5c93cbd36f3b4c6ec03f3dffe724a86fedaaa20e6e787ca98a0f4f8`. Exact native result/state/callback rows for 8.5.19
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv). SHA-256 `9646fcdfe5c93cbd36f3b4c6ec03f3dffe724a86fedaaa20e6e787ca98a0f4f8`. Exact native result/state/callback rows for 8.6.18
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv). SHA-256 `9646fcdfe5c93cbd36f3b4c6ec03f3dffe724a86fedaaa20e6e787ca98a0f4f8`. Exact native result/state/callback rows for 9.0.4
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv). SHA-256 `9646fcdfe5c93cbd36f3b4c6ec03f3dffe724a86fedaaa20e6e787ca98a0f4f8`. Exact native result/state/callback rows for 9.1.0

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `public_error_resets_preserve_global_headers_and_start_fresh_episodes`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::public_error_resets_preserve_global_headers_and_start_fresh_episodes` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

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
  "rust/tcl-registry/tests/data/native_error_variables/reset-episodes.c",
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
