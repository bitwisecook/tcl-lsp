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

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: header_sha256=f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: header_sha256=72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

Exact selected emitted rows:

```text
initial-stack	absent-field
```
absent-field is a compile-time branch sentinel; it does not observe the omitted runtime API.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: header_sha256=e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: header_sha256=f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: header_sha256=fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Private C Interp observer following public Tcl_EvalEx/Tcl_SaveInterpState/Tcl_RestoreInterpState with ordinary ASCII input.. Dialect: C Tcl.

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

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_error_headers/probe.c](../../../../rust/tcl-registry/tests/data/native_error_headers/probe.c). SHA-256 `bafa27b7e79c9b847bf456e829cddd35f50011ab7724de6c469ea8552ce2a64b`. Exact private field conditional boundaries, original child selection and saved-state pointer/count sampling.
- `receipt` (observation): [rust/tcl-registry/tests/data/native_error_headers/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_headers/manifest.json). SHA-256 `fe5f818e8c364774d335d037beeeb3da22dcfc1354fa786d002a8980b48e8ad0`. All five separate compile/process status/stdout/stderr captures; sentinel branches retained.
- `rows` (observation): [rust/tcl-registry/tests/data/native_error_headers/observations.tsv](../../../../rust/tcl-registry/tests/data/native_error_headers/observations.tsv). SHA-256 `8db9e4d50d705dc72c4b4940ef34b7d1619f955c311d693a0db76042031c0c17`. Exact 38-row projection agrees byte-for-byte with the prefixed original inline streams.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `NativeErrorObjectsProtocol`: Selects independently available C private header storage, separate from public return-options projections.
- [rust/tcl-registry/src/native_error_objects.rs](../../../../rust/tcl-registry/src/native_error_objects.rs), `native_error_objects::tests::original_private_header_fields_match_five_native_constructor_and_save_windows` (linked): Compares all 38 original availability/primary/count/identity projection rows against the release-selected private header recipe; no native launch is implied.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
