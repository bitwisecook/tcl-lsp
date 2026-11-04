# naming.error.original-private-return-options-header

Kind: `native-observation`

## Problem statement

A public return-options dictionary projection cannot prove ownership of the interpreter private returnOpts header. Saving interpreter state can retain that exact container without separately retaining its original child.

## Question

What private return-options container/child references and header identity survive original return, SaveInterpState, mutation and RestoreInterpState?

## Conclusion

C8.5–9.1 begin with no private options object, then observe Dictionary header refs1 and original untyped custom child refs1. Save raises only container refs to2; a new return replaces the current header while the saved old container/child remain1. Restore reinstalls the original header at refs1 with the same child. C8.4 emits a compiled field-absence sentinel and performs no return-options operation. This is private header ownership for these exact producer/save windows, not a public result/options overlay or callback proof.

## Scope

Five private C Interp header probes. Ordinary ASCII Tcl_EvalEx return/error scripts produce the stored original headers; private pointers are sampled while interpreter/saved state owns them. Tcl_DictObjGet or Tcl_ListObjGetElements selects the original child before saved-state counts. No public return-options/error-global observer participates. Original private-header/archive/source hashes and separate successful compiler/process streams are retained; binary/configuration/compiler/queried runtime version are unrecorded. Field-absence rows are compile-time sentinels, not guest API rejections. No Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-options	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-options	absent
options-before	dict	1	none	1
options-saved	2	1
options-mutated	1	1	1
options-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-options	absent
options-before	dict	1	none	1
options-saved	2	1
options-mutated	1	1	1
options-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-options	absent
options-before	dict	1	none	1
options-saved	2	1
options-mutated	1	1	1
options-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-options	absent
options-before	dict	1	none	1
options-saved	2	1
options-mutated	1	1	1
options-restored	1	1	1
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

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `NativeErrorObjectsProtocol`: Selects independently available C private header storage, separate from public return-options projections.
- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `native_error_objects::tests::original_private_header_fields_match_five_native_constructor_and_save_windows` (linked): Compares all 38 original availability/primary/count/identity projection rows against the release-selected private header recipe; no native launch is implied.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
