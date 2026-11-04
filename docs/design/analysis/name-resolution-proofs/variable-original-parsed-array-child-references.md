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

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|1|-1
parts|duplicated|2|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|-1
parts|duplicated|3|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|-1
parts|duplicated|3|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|2
parts|duplicated|3|3
parts|unset|1|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|2
parts|duplicated|3|3
parts|unset|1|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c). Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json). Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt). Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::element_table_retains_original`: Keeps actual object-key ownership distinct from parser header/entry lifetime.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
