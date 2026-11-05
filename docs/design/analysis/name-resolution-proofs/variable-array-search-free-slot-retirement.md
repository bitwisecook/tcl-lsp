# naming.variable.array-search-free-slot-retirement

Kind: `native-observation`

## Problem statement

An array-search primary has no free hook in older C releases. Clearing every primary or retaining every no-free-hook primary before a variable lookup predicts different original-header states; duplication adds a separate retirement window.

## Question

What primary/free-hook/resident states remain on the original array-search handle and its duplicate before and after missing variable reads?

## Conclusion

C8.4 retains array search with no free hook on both missing reads. C8.5/8.6 begin with that primary on both objects and retire both to an untyped primary. C9 begins with a String primary on the original and an untyped duplicate; those states remain after reads. All four snapshots retain resident strings. This measures the selected descriptor retirement and copy windows, not search validity or any arbitrary variable read completion.

## Scope

Twenty original header observations across five C captures. Fields are phase, primary, free-hook-present and string-resident. Search identifier content is not printed; the missing read result/completion is not sampled. One original handle and one duplicate have explicit external references. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
converted|array search|0|1
duplicate|array search|0|1
missing|array search|0|1
duplicate-missing|array search|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
converted|array search|0|1
duplicate|array search|0|1
missing|none|0|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate. Version: tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
converted|string|1|1
duplicate|none|0|1
missing|string|1|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::retires_before_simple_lookup`: Selects whether the actual original free-hook/primary is retired before simple lookup.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::search_free_slots_match_all_20_actual_c_windows` (linked): Compares the 20 retained primary/free-hook/resident windows; host variable read and duplicate ownership remain independent implementation obligations.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::search_free_slots_match_all_20_actual_c_windows` (linked): Compares the 20 retained primary/free-hook/resident windows; host variable read and duplicate ownership remain independent implementation obligations.

## Evidence

- [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.c)
- [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot_provenance.json)
- [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt)
