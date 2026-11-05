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

Status: `observed`. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected emitted rows:

```text
initial-options	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected emitted rows:

```text
initial-options	absent
options-before	dict	1	none	1
options-saved	2	1
options-mutated	1	1	1
options-restored	1	1	1
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `NativeErrorObjectsProtocol`: Selects independently available C private header storage, separate from public return-options projections.
- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `native_error_objects::tests::original_private_header_fields_match_five_native_constructor_and_save_windows` (linked): Compares all 38 original availability/primary/count/identity projection rows against the release-selected private header recipe; no native launch is implied.

## Evidence

- [rust/tcl-registry/tests/data/native_error_headers/probe.c](../../../../rust/tcl-registry/tests/data/native_error_headers/probe.c)
- [rust/tcl-registry/tests/data/native_error_headers/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_headers/manifest.json)
- [rust/tcl-registry/tests/data/native_error_headers/observations.tsv](../../../../rust/tcl-registry/tests/data/native_error_headers/observations.tsv)
