# naming.variable.original-scalar-alias-entry-lifetime

Kind: `native-observation`

## Problem statement

Unsetting a scalar target can leave its entry alive because an actual procedure alias still points to it. Recreating by a fresh global name or by the alias must not fabricate a new entry from displayed spelling.

## Question

Which original scalar entry/name ownership persists across upvar, target removal/recreation, alias unset and frame pop?

## Conclusion

Both measured recreation modes retain the original scalar Var pointer while its procedure alias lives. C8.5 and later retain the original name as the actual table key with refs2; C8.4 uses a distinct string-key table and name refs1. The fresh name has refs1. Unset of the undefined alias reports guest code1. Frame pop removes the visible entry in every capture, and later fresh read finds no entry. No freed Var contents are read after that point.

## Scope

Five exact C processes, 29 raw lines per process; the retained aggregate projects only the 19 alias/child snapshots per release. Full call/procedure outcomes remain in each original inline receipt and match its recorded raw log SHA. The scalar before pointer is compared but its fields are not dereferenced after retirement. Whole strings are ordinary k/arr(k), not opaque byte inputs. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
alias|0|created|parsedVarName|1|-1|1|1|0
call|0
alias|0|linked|parsedVarName|1|-1|1|1|0
call|0
alias|0|unset-target|parsedVarName|1|-1|1|1|0
alias|0|recreated|parsedVarName|1|1|1|1|0
call|0
alias|0|unset-recreated-target|parsedVarName|1|1|1|1|0
call|1
alias|0|unset-alias|parsedVarName|1|1|1|1|0
procedure|0
alias|0|frame-popped|parsedVarName|1|1|0|0|0
alias|0|fresh-read|parsedVarName|1|1|0|0|0
alias|1|created|parsedVarName|1|-1|1|1|0
call|0
alias|1|linked|parsedVarName|1|-1|1|1|0
call|0
alias|1|unset-target|parsedVarName|1|-1|1|1|0
alias|1|recreated|parsedVarName|1|1|1|1|0
call|0
alias|1|unset-recreated-target|parsedVarName|1|1|1|1|0
call|1
alias|1|unset-alias|parsedVarName|1|1|1|1|0
procedure|0
alias|1|frame-popped|parsedVarName|1|1|0|0|0
alias|1|fresh-read|parsedVarName|1|1|0|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
alias|0|created|parsedVarName|2|-1|1|1|1
call|0
alias|0|linked|parsedVarName|2|-1|1|1|1
call|0
alias|0|unset-target|parsedVarName|2|-1|1|1|1
alias|0|recreated|parsedVarName|2|1|1|1|1
call|0
alias|0|unset-recreated-target|parsedVarName|2|1|1|1|1
call|1
alias|0|unset-alias|parsedVarName|2|1|1|1|1
procedure|0
alias|0|frame-popped|parsedVarName|1|1|0|0|0
alias|0|fresh-read|parsedVarName|1|1|0|0|0
alias|1|created|parsedVarName|2|-1|1|1|1
call|0
alias|1|linked|parsedVarName|2|-1|1|1|1
call|0
alias|1|unset-target|parsedVarName|2|-1|1|1|1
alias|1|recreated|parsedVarName|2|1|1|1|1
call|0
alias|1|unset-recreated-target|parsedVarName|2|1|1|1|1
call|1
alias|1|unset-alias|parsedVarName|2|1|1|1|1
procedure|0
alias|1|frame-popped|parsedVarName|1|1|0|0|0
alias|1|fresh-read|parsedVarName|1|1|0|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::element_table_retains_original`: Keeps actual object-key ownership distinct from parser header/entry lifetime.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.

## Evidence

- [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c)
- [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json)
- [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt)
