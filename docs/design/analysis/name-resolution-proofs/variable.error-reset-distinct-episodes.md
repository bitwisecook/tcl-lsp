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

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Version: tcl8.4 8.4.20.

Observed capture for 8.4.20:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.4.20	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl8.5

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Version: tcl8.5 8.5.19.

Observed capture for 8.5.19:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.5.19	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl8.6

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Version: tcl8.6 8.6.18.

Observed capture for 8.6.18:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 8.6.18	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl9.0

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Version: tcl9.0 9.0.4.

Observed capture for 9.0.4:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 9.0.4	0	1	1	2	0	1	2	1	1	1	1	1	1

### tcl9.1

Status: `observed`. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by reset-episodes.c. Version: tcl9.1 9.1.0.

Observed capture for 9.1.0:
version	definition	first_error	same_reset_header	reset_refs	second_definition	same_definition_header	definition_refs	second_error	fresh_error_header	message_X	clean_episode	third_error	third_clean_episode; 9.1.0	0	1	1	2	0	1	2	1	1	1	1	1	1

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `public_error_resets_preserve_global_headers_and_start_fresh_episodes`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::public_error_resets_preserve_global_headers_and_start_fresh_episodes` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_error_variables/reset-episodes-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes-manifest.json)
- [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.c](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.c)
- [rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv)
