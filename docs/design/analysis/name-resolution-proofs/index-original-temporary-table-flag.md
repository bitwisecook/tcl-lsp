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

### tcl8.4, tcl8.5, tcl8.6

Status: `not-tested`. Channel: C conditional branch did not execute the temporary-table lookup. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried).

Original probe compiled out the temporary-table branch; no flag64 call or guest rejection was observed.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_GetIndexFromObj original object with exact retained static table pointers; object duplication/string invalidation uses public APIs. Version: tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

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

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `NativeIndexLookupProtocol::cached_index_with_flags`: Checks independently retained table/stride/cache/flag correspondence.
- [rust/tcl-cshim/src/index_table.rs](../../../../rust/tcl-cshim/src/index_table.rs), `StaticIndexTable`: Retains the independently supplied table entry reader and native storage lifetime contract.
- [runtime/rust/src/interp/native_index_lookup.rs](../../../../runtime/rust/src/interp/native_index_lookup.rs), `interp::native_index_lookup::tests::original_index_flags_and_offsets_match_all_native_31_windows` (linked): Independent shared flag/stride/cache controls exercise temporary-table no-install behavior; this test uses its own 31-window corpus rather than claiming to execute the shim probe.

## Evidence

- [rust/tcl-cshim/tests/data/native_index_cache/probe.c](../../../../rust/tcl-cshim/tests/data/native_index_cache/probe.c)
- [rust/tcl-cshim/tests/data/native_index_cache/manifest.json](../../../../rust/tcl-cshim/tests/data/native_index_cache/manifest.json)
