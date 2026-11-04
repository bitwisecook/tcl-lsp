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

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	none	1
stack-before	list	1	2	none	1
stack-saved	2	1
stack-mutated	1	1	1
stack-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	none	1
stack-before	list	1	2	none	1
stack-saved	2	1
stack-mutated	1	1	1
stack-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	none	1
stack-before	list	1	2	none	1
stack-saved	2	1
stack-mutated	1	1	1
stack-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_error_headers/probe.c](../../../../rust/tcl-registry/tests/data/native_error_headers/probe.c). Exact private field conditional boundaries, original child selection and saved-state pointer/count sampling.
- `receipt` (observation): [rust/tcl-registry/tests/data/native_error_headers/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_headers/manifest.json). All five separate compile/process status/stdout/stderr captures; sentinel branches retained.
- `rows` (observation): [rust/tcl-registry/tests/data/native_error_headers/observations.tsv](../../../../rust/tcl-registry/tests/data/native_error_headers/observations.tsv). Exact 38-row projection agrees byte-for-byte with the prefixed original inline streams.
- `authored-consumer645d-697f7e7187ab` (implementation): [runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs). Actual current authored software control source and genuine module declaration, inspected only for the attached finite assertion. No owned-image admission, passed assertion, external Native observation or provider outcome.
- `authored-consumer645d-2db75cd5b415` (implementation): [runtime/rust/src/interp/native_error_headers.rs](../../../../runtime/rust/src/interp/native_error_headers.rs). Actual current authored software control source and genuine module declaration, inspected only for the attached finite assertion. No owned-image admission, passed assertion, external Native observation or provider outcome.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `NativeErrorObjectsProtocol`: Selects independently available C private header storage, separate from public return-options projections.
- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `native_error_objects::tests::original_private_header_fields_match_five_native_constructor_and_save_windows` (linked): Compares all 38 original availability/primary/count/identity projection rows against the release-selected private header recipe; no native launch is implied.
- [runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_error/original_return_boundary_tests.rs), `interp::native_body_artifact::native_error::original_return_boundary_tests::original_return_stack_string_refuses_before_list_backing_and_private_stack_adoption` (linked): Actual private-stack header/episode and original frame survive selected String and checked duplicate/List adoption, with first refusal retained before unused owner cleanup. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.
- [runtime/rust/src/interp/native_error_headers.rs](../../../../runtime/rust/src/interp/native_error_headers.rs), `interp::native_error_headers::original_prepared_stack_tests::original_prepared_stack_unavailable_releases_do_not_issue_or_retire_private_headers` (linked): Actual C84/C85 unsupported stack recipes issue no prepared private owner and perform no Native private-header retirement. This is an inspected Rust software control; it supplies no passed assertion, external Native callback chronology, role parity or new getter/worker authority.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
