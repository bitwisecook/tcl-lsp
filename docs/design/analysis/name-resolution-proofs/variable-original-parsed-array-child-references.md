# naming.variable.original-parsed-array-child-references

Kind: `native-observation`

## Problem statement

A combined parsed name owns child storage that a duplicate may share. Table removal and duplicate destruction change child counts independently of the parent name or lookup result.

## Question

What child header references remain after original arr(k) store, name duplication and duplicate/table removal?

## Conclusion

C8.4 reports root child refs1,2,1. C8.5/8.6 report root2,3,1. C9 reports root and index2,3,1 independently. Pre-C9 index fields are deliberately unobserved and reported−1; those sentinels do not mean no index bytes exist. These three windows do not supply arbitrary alias allocation or table membership proof.

## Scope

Five exact C processes, 29 raw lines per process; the retained aggregate projects only the 19 alias/child snapshots per release. Full call/procedure outcomes remain in each original inline receipt and match its recorded raw log SHA. The scalar before pointer is compared but its fields are not dereferenced after retirement. Whole strings are ordinary k/arr(k), not opaque byte inputs. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
parts|stored|1|-1
parts|duplicated|2|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
parts|stored|2|-1
parts|duplicated|3|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer. Version: tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected observer rows, in original column order:

```text
parts|stored|2|2
parts|duplicated|3|3
parts|unset|1|1
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
