# naming.error.original-private-error-stack-header

Kind: `native-observation`

## Problem statement

The interpreter error-stack header is separate from error globals and public return-options. Its initial untyped object, later original List and saved state can own different references and identities.

## Question

What private error-stack container/child state is observed at construction, original error, save, second error and restore?

## Conclusion

C8.6–9.1 begin with an untyped private stack object refs1. Error FIRST produces a two-member List header refs1 with observed second child untyped/refs1. Save raises only the header to2; Error SECOND replaces the current header while the retained original header/child remain1. Restore reinstalls that original header at refs1. C8.4/8.5 only emit compiled field-absence sentinels, so no error-stack producer/save behavior is observed there. This does not establish error-global aliasing or any other error episode.

## Scope

Five private C Interp header probes. Ordinary ASCII Tcl_EvalEx return/error scripts produce the stored original headers; private pointers are sampled while interpreter/saved state owns them. Tcl_DictObjGet or Tcl_ListObjGetElements selects the original child before saved-state counts. No public return-options/error-global observer participates. Original private-header/archive/source hashes and separate successful compiler/process streams are retained; binary/configuration/compiler/queried runtime version are unrecorded. Field-absence rows are compile-time sentinels, not guest API rejections. No Jim/BIG-IP attempt.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried); tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried).

Exact selected emitted rows:

```text
initial-stack	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input. Version: tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected emitted rows:

```text
initial-stack	none	1
stack-before	list	1	2	none	1
stack-saved	2	1
stack-mutated	1	1	1
stack-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `NativeErrorObjectsProtocol`: Selects independently available C private header storage, separate from public return-options projections.
- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `native_error_objects::tests::original_private_header_fields_match_five_native_constructor_and_save_windows` (linked): Compares all 38 original availability/primary/count/identity projection rows against the release-selected private header recipe; no native launch is implied.
- [runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs), `interp::native_body_artifact::native_error::original_return_boundary_tests::original_return_stack_string_refuses_before_list_backing_and_private_stack_adoption` (linked): Actual private-stack header/episode and original frame survive selected String and checked duplicate/List adoption, with first refusal retained before unused owner cleanup. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_error_headers.rs](../../../../runtime/rust/src/interp/native_error_headers.rs), `interp::native_error_headers::original_prepared_stack_tests::original_prepared_stack_unavailable_releases_do_not_issue_or_retire_private_headers` (linked): Actual C84/C85 unsupported stack recipes issue no prepared private owner and perform no Native private-header retirement. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

## Evidence

- [rust/tcl-registry/tests/data/native_error_headers/probe.c](../../../../rust/tcl-registry/tests/data/native_error_headers/probe.c)
- [rust/tcl-registry/tests/data/native_error_headers/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_headers/manifest.json)
- [rust/tcl-registry/tests/data/native_error_headers/observations.tsv](../../../../rust/tcl-registry/tests/data/native_error_headers/observations.tsv)
- [runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs)
- [runtime/rust/src/interp/native_error_headers.rs](../../../../runtime/rust/src/interp/native_error_headers.rs)
