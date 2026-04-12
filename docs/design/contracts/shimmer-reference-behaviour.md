# KCS: Shimmer reference behaviour and validation

## What we mean by shimmer

A Tcl object can be used as different semantic types over time (string, list, int, double, etc.). Shimmering is the conversion churn between internal representations when usage changes. In C Tcl, this happens when `Tcl_ConvertToType()` calls `FreeInternalRep` on the old type and `setFromAnyProc` on the new type.

## Practical expectations for this project

- A one-off mismatch at a use site is informative (S100).
- Repeated mismatch in loops is more expensive and should be elevated (S101).
- Oscillation patterns across loop iterations are the strongest signal (S102).

## Mapping to C Tcl 9.0.3 functions

Each detector diagnostic maps to specific C functions that trigger `FreeInternalRep`:

| Detector diagnostic | C function / macro | Source file |
|---|---|---|
| STRING → LIST (S100/S101) | `Tcl_GetListFromObj` → `SetListFromAny` | `tclListObj.c` |
| STRING → INT (S100/S101) | `TclGetIntFromObj` / `Tcl_GetWideIntFromObj` | `tclObj.c` |
| STRING → DOUBLE (S100/S101) | `Tcl_GetDoubleFromObj` | `tclObj.c` |
| STRING → BOOLEAN (S100/S101) | `Tcl_GetBooleanFromObj` | `tclObj.c` |
| STRING → DICT (S100/S101) | `Tcl_DictObjGet` → `SetDictFromAny` | `tclDictObj.c` |
| STRING → INT via `incr` | `TclIncrObj` → `TclGetNumberFromObj` | `tclObj.c` |
| STRING → NUMERIC in `expr` | `TclGetNumberFromObj` in `INST_ADD` etc. | `tclExecute.c` |
| INT/DOUBLE → STRING in `expr` | `Tcl_GetStringFromObj` in `INST_STR_EQ` etc. | `tclExecute.c` |
| LIST ↔ DICT oscillation | Bidirectional `SetListFromAny` / `SetDictFromAny` | `tclListObj.c`, `tclDictObj.c` |
| BOOLEAN → INT promotion | `TclGetIntFromObj` (cheap path) | `tclObj.c` |

### Numeric subtype hierarchy

BOOLEAN → INT promotion is **not** flagged because it matches Tcl 9.0's O(1) conversion path. The `_is_numeric_compatible` function implements: BOOLEAN ⊆ INT ⊆ NUMERIC, DOUBLE ⊆ NUMERIC.

### When shimmering does NOT occur

- Same-type access (fast path in all `Tcl_Get*FromObj` functions)
- String rep generation from intrep (intrep is preserved alongside string rep)
- Pure string objects (`typePtr == NULL`) — first type assignment is not a shimmer
- Shared object duplication (`Tcl_DuplicateObj`) — original intrep is not affected

## Reference validation status

C source analysis completed against Tcl 9.0.3. The detector's `TYPE_HINTS` registry correctly maps Tcl commands to their underlying `Tcl_Get*FromObj` calls.

## Fixture scenarios

See `tests/fixtures/shimmer/` for script-based cases that exercise:

- string → list conversion (`string_scalar_to_list_once.tcl`, `string_list_roundtrip.tcl`),
- numeric/string coercion in expression loops (`numeric_string_loop_thrash.tcl`),
- list/string oscillation pressure in loops (`list_string_loop_toggle.tcl`),
- dict/list oscillation (`dict_list_oscillation.tcl`),
- boolean/int promotion — no false positive (`boolean_int_promotion.tcl`),
- namespace-scoped shimmer variants (`namespace_scalar_vs_list.tcl`, `namespace_array_vs_list.tcl`).

## Cross-links

- Tests: `tests/test_shimmer.py`.
- Conformance plan: `docs/plans/shimmering-conformance-plan.md`.
- Implementation: `core/compiler/shimmer.py`.
