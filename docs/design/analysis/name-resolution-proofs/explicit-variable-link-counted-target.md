# naming.tcloo.explicit-variable-link-counted-target

Kind: `native-observation`

## Problem statement

Explicit object variable links decoded the original name or treated its local alias spelling as its namespace key, and rejected array-element spelling before creating the native array shell.

## Question

Which local name, target key and partial namespace state result from the eight original counted explicit my variable operands?

## Conclusion

C8.6/C9.0/C9.1 link local k to the full counted object namespace key k00tail or k00::Q. Encoded C080 and FF names retain their counted spelling. Namespace separators before raw zero reject. The a(k) control rejects while leaving array shell a. A later full-name dynamic write is independent of the clipped local alias. Other providers fail TclOO setup and do not reach LinkVar.

A marked direct VarTable control separates counted-key declaration visibility, alias retention and undefined-value state.

A marked CmdCore pure filtering control binds complete counted keys and selected variable-search units to the shared name-pattern owner.

## Scope

Fresh class/object per original counted argv control. Eight names; link and write controls in v2, plus genuine link/partial-state controls in v1. V1 report and write-final-report recurse through a accidentally shadowing user varname method and cannot corroborate stock varname success. No private declaration, native header/cache, callback, compiler-local or Normal grant.

The body constructs an implementation table directly; it observes no original provider NUL parsing, TclOO my variable invocation, C pointer/header, declaration instruction or native partial-state publication. The original counted explicit-variable operands and their independent public windows remain unchanged. No software passing outcome is attached.

An authored_tcl recipe and constructed key vector supply no original provider NUL semantics, physical namespace variable cell, TclOO method/alias frame or current native scan/table admission. It observes no original process or passing software outcome. Original eight explicit-variable operands and independently captured public partial-state windows remain unchanged.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

TclOO class setup rejects; the selected method is not attempted.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Eight counted-name controls reached the selected stock method; exact key/local/result/error and partial state rows retained.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs](../../../../rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs), `link_original_c_oo_variable`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [runtime/rust/src/interp/native_variable_names/native_tcloo.rs](../../../../runtime/rust/src/interp/native_variable_names/native_tcloo.rs), `link_original_c_oo_variable`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [runtime/rust/src/namespace.rs](../../../../runtime/rust/src/namespace.rs), `Namespaces::var_home`: Consumes the selected variable-root qualification before locating the exact counted simple key, preserving namespace existence and byte identity.
- [rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.
- [runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs](../../../../runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.
- [runtime/rust/src/namespace.rs](../../../../runtime/rust/src/namespace.rs), `namespace::original_variable_geometry_tests::namespace_variable_roots_keep_qualification_before_zero_and_counted_simple_keys` (linked): Pure selected variable-root geometry: unqualified counted keys retain a separator after raw zero, qualification before zero selects the existing namespace and missing parents decline. The test covers C8.4/C8.5 selected recipes without claiming TclOO availability; the native OO question is C8.6/C9.0/C9.1.

- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `VarTable::mark_namespace_declared`: Retain declared namespace-cell visibility independently of a defined scalar value or active alias, using the complete software counted key.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `VarTable::has_native_namespace_cell`: Report retained namespace-cell presence separately from scalar definition and value loading.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::declared_undefined_namespace_cells_remain_visible_without_becoming_defined` (linked): A direct VarTable fixture uses counted k plus NUL plus tail, retains a scalar alias, then marks the namespace cell declared. Dropping the alias keeps the declared name visible while non_link_names is empty, is_set is false and load_scalar is None; removal withdraws visibility. These bytes are software table keys, not an original native name-input observation.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

- [rust/tcl-cmd-core/src/info.rs](../../../../rust/tcl-cmd-core/src/info.rs), `filter_ordered_names`: Retain each complete original software counted key while applying the independently selected name-glob purpose; unavailable pattern purpose refuses rather than truncating a key.
- [rust/tcl-cmd-core/src/info.rs](../../../../rust/tcl-cmd-core/src/info.rs), `info::tests::variable_name_patterns_keep_original_counted_keys_and_native_scan_units` (linked): Three selected C8.6/C9.0/C9.1 pure authored glob recipes filter ordered counted keys containing modified-NUL bytes, FF or raw NUL. Star retains all exact keys, the complete raw-NUL pattern selects only that key, and prefix k matches none. This does not invoke TclOO my variable or authenticate a native variable inventory.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v1](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v1)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2)
