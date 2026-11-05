# naming.variable.catch-result-alias-continuity

Kind: `native-observation`

## Problem statement

A caught command can replace a procedure formal cell with a global alias before catch stores its result. Retaining the original formal identity solely because its written name is unchanged would direct later reads or renames at the wrong cell. A known error-only body and a separate catch result destination distinguish catch storage from the callback's prior alias mutation.

## Question

After a caught callback unsets formal name and links it to ::destination, does catch store BOOM through that surviving alias, and what do the plain and separate-result controls produce on each tested engine?

## Conclusion

All six selected native shells return LINKED = BOOM BOOM, PLAIN = BOOM and SEPARATE_RESULT = OLD BOOM OLD. The linked catch result reaches the global destination after callback alias replacement; the separate-result control confirms the alias mutation independently. These finite ASCII source observations do not preserve an arbitrary incoming formal identity across an unknown caught command and do not establish physical variable, object, compiler or Normal capabilities.

## Scope

Three exact caught ASCII source controls supplied as native shell stdin on Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim 0.84-9-g5bac7c9. Actual info patchlevel, selected executable/header/Makefile/source-owner hashes and exact streams are retained. No raw NUL, opaque byte name, arbitrary object representation, native compiler callback, resolver, trace or BIG-IP appliance is tested. Static library digests and configure/compiler command histories are not recorded in this capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII file bytes passed to shell stdin. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_catch`: Store reached caught results through the actual surviving receiver rather than original formal-name metadata.
- [rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs](../../../../rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs), `caught_callback_result_follows_replaced_formal_alias_native_controls`: Compare exact source-triggered alias continuity and result-destination controls with scoped native shell outputs.
- [rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs](../../../../rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs), `command::native_catch_formal_alias_tests::caught_callback_result_follows_replaced_formal_alias_native_controls` (linked): Evaluate the exact retained ASCII source bytes under six independently selected VM core fixtures and compare all three caught result rows against matching original shell stdout. VM reported-version presentation is excluded from this semantic comparison.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::caught_caller_rebinding_cannot_retain_the_original_formal_symbol` (linked): Withdraw source formal identity after exact caller unset/upvar rebinding and incomplete rename coverage, independently of the native caught result rows.

## Evidence

- [rust/tcl-vm/tests/data](../../../../rust/tcl-vm/tests/data)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0)
- [rust/tcl-vm/tests/data/native_catch_formal_alias/jim](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim)
