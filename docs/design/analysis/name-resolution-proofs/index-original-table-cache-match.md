# naming.index.original-table-cache-match

Kind: `native-observation`

## Problem statement

An abbreviated word can acquire an Index cache tied to one actual table. Reusing that cache for a later exact query can differ from rechecking the bytes or comparing a different equal-content table.

## Question

Does original abbreviated a reuse its cached index for exact same-table lookup and for a distinct equal-content table?

## Conclusion

All five C captures select index0 for abbreviation then same-table exact query. A different table with equal alpha/beta entries returns guest Error and leaves the sentinel index−9; the original Index primary and bytes61 remain. This proves actual table cache correspondence for these calls, not an interchangeable bytes-based table capability.

## Scope

Exact public Tcl_GetIndexFromObj on original String a, distinct static tables first/other, TCL_EXACT and conditional C9 flag64. Tcl_InvalidateStringRep/GetString regenerates from retained mutable static table entries; Tcl_DuplicateObj preserves cache association. Five original output files have complete source/archive/output hashes and successful process status, but no compiler status, header/executable digest or launched runtime version query is recorded. Probe input is C object API, not document source or a guest command selector.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_GetIndexFromObj original object with exact retained static table pointers; object duplication/string invalidation uses public APIs. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried); tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected rows:

```json
[
  {
    "case": "prefix",
    "code": 0,
    "index": 0,
    "type": "index",
    "resident": 1,
    "hex": "61"
  },
  {
    "case": "cached-exact",
    "code": 0,
    "index": 0,
    "type": "index",
    "resident": 1,
    "hex": "61"
  },
  {
    "case": "different-table-exact",
    "code": 1,
    "index": -9,
    "type": "index",
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

## Evidence

- [rust/tcl-cshim/tests/data/native_index_cache/probe.c](../../../../rust/tcl-cshim/tests/data/native_index_cache/probe.c)
- [rust/tcl-cshim/tests/data/native_index_cache/manifest.json](../../../../rust/tcl-cshim/tests/data/native_index_cache/manifest.json)
