# naming.variable.original-element-unset-callback-roles

Kind: `native-observation`

## Problem statement

Whole-array removal can unlink the visible table before element callbacks while aliases retain old Var nodes. Callback activation, undefined flags and original index key ownership need separate observations; one global reference count cannot stand for all of them.

## Question

What key/old-Var/current-table states are reached at root unset, j then k element callbacks and completion?

## Conclusion

All five captures remove both current table members before the root callback and invoke j then k. Both old Var nodes are dead-hash after their respective element callbacks while aliases retain them. C8.4 keeps each original separate key at refs1 and clears that element definition after its callback; later C table keys remain refs2 through callbacks, with that element already undefined during its callback. The active element adds one old-Var role while its own callback runs. After completion keys return refs1 and both old nodes remain alias-owned but undefined. These snapshots are not a universal trace order or element lifetime rule.

## Scope

Thirty exact raw lines across C5. Ten fields independently capture original k/j key refs, old dead-hash/defined flags, old Var refs and current table presence. Each original separate index has one external construction reference; only object-key C8.5+ tables add one. Both old nodes stay alive through actual global aliases. No result/options/error/global observer or Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original separate Tcl_ObjSetVar2 root/index operands, Tcl_UpVar aliases and Tcl_TraceVar2/Tcl_UnsetVar; private live Var/hash-entry callback observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
window|before|1|1|0|0|1|1|1|1|1|1
window|root|1|1|0|0|1|1|1|1|0|0
window|element1-j|1|1|0|1|1|1|1|2|0|0
window|element2-k|1|1|1|1|1|0|2|1|0|0
unset|0
window|after|1|1|1|1|0|0|1|1|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original separate Tcl_ObjSetVar2 root/index operands, Tcl_UpVar aliases and Tcl_TraceVar2/Tcl_UnsetVar; private live Var/hash-entry callback observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
window|before|2|2|0|0|1|1|2|2|1|1
window|root|2|2|0|0|1|1|2|2|0|0
window|element1-j|2|2|0|1|1|0|2|3|0|0
window|element2-k|2|2|1|1|0|0|3|2|0|0
unset|0
window|after|1|1|1|1|0|0|1|1|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original separate Tcl_ObjSetVar2 root/index operands, Tcl_UpVar aliases and Tcl_TraceVar2/Tcl_UnsetVar; private live Var/hash-entry callback observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
window|before|2|2|0|0|1|1|2|2|1|1
window|root|2|2|0|0|1|1|2|2|0|0
window|element1-j|2|2|0|1|1|0|2|3|0|0
window|element2-k|2|2|1|1|0|0|3|2|0|0
unset|0
window|after|1|1|1|1|0|0|1|1|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original separate Tcl_ObjSetVar2 root/index operands, Tcl_UpVar aliases and Tcl_TraceVar2/Tcl_UnsetVar; private live Var/hash-entry callback observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
window|before|2|2|0|0|1|1|2|2|1|1
window|root|2|2|0|0|1|1|2|2|0|0
window|element1-j|2|2|0|1|1|0|2|3|0|0
window|element2-k|2|2|1|1|0|0|3|2|0|0
unset|0
window|after|1|1|1|1|0|0|1|1|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original separate Tcl_ObjSetVar2 root/index operands, Tcl_UpVar aliases and Tcl_TraceVar2/Tcl_UnsetVar; private live Var/hash-entry callback observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
window|before|2|2|0|0|1|1|2|2|1|1
window|root|2|2|0|0|1|1|2|2|0|0
window|element1-j|2|2|0|1|1|0|2|3|0|0
window|element2-k|2|2|1|1|0|0|3|2|0|0
unset|0
window|after|1|1|1|1|0|0|1|1|0|0
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.c). Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.provenance.json). Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt). Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::element_unset_preserves_definition_during_trace`: Selects actual element definition timing independently from callback activation and table-key ownership.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::element_table_and_var_roles_match_all_25_actual_callback_windows` (linked): Compares the 25 callback state snapshots; original separate keys and retained old nodes remain independently owned.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::cpp_table_and_var_roles_match_all_25_original_callback_windows` (linked): Compares the 25 callback state snapshots; original separate keys and retained old nodes remain independently owned.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.
