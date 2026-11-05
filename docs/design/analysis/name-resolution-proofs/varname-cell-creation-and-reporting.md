# naming.tcloo.varname-cell-creation-and-reporting

Kind: `native-observation`

## Problem statement

Varname returned a constructed display path without performing the native cell creation and followed-target lookup; String reconstruction also lost raw original operands.

## Question

What followed variable name and namespace creation does stock my varname expose for these eight original counted operands?

## Conclusion

The separately pinned v2 stock controls create and report namespace::k for raw00 relative names on all supported releases, distinct from the full-counted key created by explicit LinkVar. Encoded C080/FF survive reporting. a(k) succeeds and reports an element; relative N::k fails missing parent. Absolute raw00 reports ::global. Exact created keys and error/result/options remain labelled. V1 user-method recursion does not establish a stock varname outcome.

## Scope

Only separately pinned v2 report/write controls on fresh C8.6/C9.0/C9.1 objects, with original input and before/after namespace rows; unavailable setup on C8.4/C8.5/Jim. Generated object namespace numbers are actual presentation, not cross-control allocation identity. No private mapping, native object/header/cache or successful unrelated operation claim.

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

- [rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs](../../../../rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs), `original_c_oo_varname`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [runtime/rust/src/interp/native_variable_names/native_tcloo.rs](../../../../runtime/rust/src/interp/native_variable_names/native_tcloo.rs), `original_c_oo_varname`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.
- [runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs](../../../../runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0)
- [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim)
