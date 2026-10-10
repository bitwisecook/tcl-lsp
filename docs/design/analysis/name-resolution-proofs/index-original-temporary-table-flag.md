# naming.index.original-temporary-table-flag

Kind: `native-observation`

## Problem statement

A temporary table lookup may return an index without installing a persistent table cache. Older-release compile branches do not exercise that flag and must not be reported as native rejections.

## Question

Does C9 flag64 select the abbreviated entry without installing an Index primary on the original object?

## Conclusion

Both C9 captures return success/index0 while leaving an untyped primary and resident bytes61. Pre-C9 captures do not enter the temporary-table branch, so no older-release flag behavior is observed. Jim/BIG-IP are not tested.

## Scope

Exact public Tcl_GetIndexFromObj on original String a, distinct static tables first/other, TCL_EXACT and conditional C9 flag64. Tcl_InvalidateStringRep/GetString regenerates from retained mutable static table entries; Tcl_DuplicateObj preserves cache association. Five original output files have complete source/archive/output hashes and successful process status, but no compiler status, header/executable digest or launched runtime version query is recorded. Probe input is C object API, not document source or a guest command selector.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit_code=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C conditional branch did not execute the temporary-table lookup. Dialect: C Tcl.

Original probe compiled out the temporary-table branch; no flag64 call or guest rejection was observed.

### tcl8.5

Status: `not-tested`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit_code=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C conditional branch did not execute the temporary-table lookup. Dialect: C Tcl.

Original probe compiled out the temporary-table branch; no flag64 call or guest rejection was observed.

### tcl8.6

Status: `not-tested`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit_code=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C conditional branch did not execute the temporary-table lookup. Dialect: C Tcl.

Original probe compiled out the temporary-table branch; no flag64 call or guest rejection was observed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit_code=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_GetIndexFromObj original object with exact retained static table pointers; object duplication/string invalidation uses public APIs.. Dialect: C Tcl.

Exact selected rows:

```json
[
  {
    "case": "temporary-table",
    "code": 0,
    "index": 0,
    "type": "none",
    "resident": 1,
    "hex": "61"
  }
]
```
The two regeneration rows report authored code/index constants; they are string/header observations, not additional GetIndex calls.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit_code=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_GetIndexFromObj original object with exact retained static table pointers; object duplication/string invalidation uses public APIs.. Dialect: C Tcl.

Exact selected rows:

```json
[
  {
    "case": "temporary-table",
    "code": 0,
    "index": 0,
    "type": "none",
    "resident": 1,
    "hex": "61"
  }
]
```
The two regeneration rows report authored code/index constants; they are string/header observations, not additional GetIndex calls.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-cshim/tests/data/native_index_cache/probe.c](../../../../rust/tcl-cshim/tests/data/native_index_cache/probe.c). SHA-256 `0ecd1cd6e69ec369ac82f8f32712056b940b531ba662a0f575aa4559984782cd`. Exact static table identities, original API flags and before-string header observer.
- `receipt` (provider): [rust/tcl-cshim/tests/data/native_index_cache/manifest.json](../../../../rust/tcl-cshim/tests/data/native_index_cache/manifest.json). SHA-256 `7ba270e4579b1ec50bc3f75efd23a1c95982adef3580aae39bd140898108566e`. Original compile arguments, source/archive/output hashes and process status.
- `rows-tcl8.4` (observation): [rust/tcl-cshim/tests/data/native_index_cache/8.4.20.jsonl](../../../../rust/tcl-cshim/tests/data/native_index_cache/8.4.20.jsonl). SHA-256 `8926b60da95068a6a0281c4a63f840b4d23317ad11265e3d8d552e0ea6ba383d`. Exact original full output; selected case labels ['temporary-table'].
- `rows-tcl8.5` (observation): [rust/tcl-cshim/tests/data/native_index_cache/8.5.19.jsonl](../../../../rust/tcl-cshim/tests/data/native_index_cache/8.5.19.jsonl). SHA-256 `8926b60da95068a6a0281c4a63f840b4d23317ad11265e3d8d552e0ea6ba383d`. Exact original full output; selected case labels ['temporary-table'].
- `rows-tcl8.6` (observation): [rust/tcl-cshim/tests/data/native_index_cache/8.6.18.jsonl](../../../../rust/tcl-cshim/tests/data/native_index_cache/8.6.18.jsonl). SHA-256 `8926b60da95068a6a0281c4a63f840b4d23317ad11265e3d8d552e0ea6ba383d`. Exact original full output; selected case labels ['temporary-table'].
- `rows-tcl9.0` (observation): [rust/tcl-cshim/tests/data/native_index_cache/9.0.4.jsonl](../../../../rust/tcl-cshim/tests/data/native_index_cache/9.0.4.jsonl). SHA-256 `2f535fb519ec211e132537f0792ef3ed202d7d959730adbc844ebccfc6cf9614`. Exact original full output; selected case labels ['temporary-table'].
- `rows-tcl9.1` (observation): [rust/tcl-cshim/tests/data/native_index_cache/9.1.0.jsonl](../../../../rust/tcl-cshim/tests/data/native_index_cache/9.1.0.jsonl). SHA-256 `2f535fb519ec211e132537f0792ef3ed202d7d959730adbc844ebccfc6cf9614`. Exact original full output; selected case labels ['temporary-table'].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `NativeIndexLookupProtocol::cached_index_with_flags`: Checks independently retained table/stride/cache/flag correspondence.
- [rust/tcl-cshim/src/index_table.rs](../../../../rust/tcl-cshim/src/index_table.rs), `StaticIndexTable`: Retains the independently supplied table entry reader and native storage lifetime contract.
- [runtime/rust/src/interp/native_index_lookup.rs](../../../../runtime/rust/src/interp/native_index_lookup.rs), `interp::native_index_lookup::tests::original_index_flags_and_offsets_match_all_native_31_windows` (linked): Independent shared flag/stride/cache controls exercise temporary-table no-install behavior; this test uses its own 31-window corpus rather than claiming to execute the shim probe.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
