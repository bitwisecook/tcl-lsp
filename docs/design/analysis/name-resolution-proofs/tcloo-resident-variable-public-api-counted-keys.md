# naming.tcloo.resident-variable-public-api-counted-keys

Kind: `native-observation`

## Problem statement

TclOO declaration bytes, compiled primary selection, dynamic root lookup and reporter results are distinct purposes. The same raw-zero or opaque name may be visible in one public API and absent in another. The captured writer/read/report/getter sequence bounds those differences without equating a report with a stored key or granting a compiled local from name equality.

## Question

What public report leaves and namespace getter outcomes follow the captured static writes, frame observers, static reads and dynamic accesses for the six declaration/primary arrangements?

## Conclusion

All original public outcomes are retained by label and input variant. C8.6 raw-zero reporter results differ between legacy and counted evaluation; C9.0/C9.1 counted reports retain the full raw-zero key. Prefix-only and qualification-after-zero cases retain their distinct errors/access outcomes. Static, dynamic and observer windows remain separately measured; no generic alias, normal-write, callback-free or allocation guarantee follows.

## Scope

Six fixed declaration/primary arrangements (plain, raw zero, qualifier-after-zero, D800, FF, prefix negative), exact original v1..v5 inputs/observer chronology and API windows. C86/C90/C91 supported OO, C84/C85/Jim explicit unavailable OO controls. v3 C86 failed compile is retained separately.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted declaration/method objects and Tcl_EvalObjv calls; ASCII Tcl_Eval/Tcl_EvalEx reporter source; actual ListObjIndex child bytes. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact labelled static/dynamic/report/namespace-getter outcomes are retained. Legacy and counted result boundaries are separate.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_variables.rs](../../../../rust/tcl-vm/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.
- [runtime/rust/src/cmd_oo/native_variables.rs](../../../../runtime/rust/src/cmd_oo/native_variables.rs), `cmd_oo::native_variables::tests::compiled_and_dynamic_declared_variables_match_original_native_controls` (linked): Compares original counted dispatch results with the legacy result projection only for rows actually captured through Tcl_Eval, and independently asserts v5 object-vector bytes without projection. No execution is inferred.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_resident_keys](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v1](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v1)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v2](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v2)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v3](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v3)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v5](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v5)
