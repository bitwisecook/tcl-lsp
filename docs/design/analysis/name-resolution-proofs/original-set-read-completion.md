# naming.variable.original-set-read-completion

Kind: `implementation-contract`

## Problem statement

A selected Set read can retain an Error alternative after the exact current receiver has an independently sealed normal-read receipt. Joining that residual branch after ordered native assignment withdraws earlier represented stores.

## Question

When may a selected original Set read discard its intrinsic missing-value error alternative?

## Conclusion

The actual selected Set AfterRead result descriptor and exact one-argument original operand select the current receiver. The existing value-free read issuer checks defined scalar or indexed contents, current identity and membership, selected policy and closed read observers. The driver captures that receipt before the handler, rechecks it after argument and preparation effects, and consumes it only through the existing selected implementation and quiet same-site completion boundary. Known bytes do not establish success.

## Scope

Native source execution with actual original effective operands and independently selected Set handler. Undefined, array-root, unknown, stale or observed receivers, unknown argument cardinality and substituted custom handlers retain their existing alternatives. Native compiler admission, object identity, prior-result release effects and value-byte correspondence remain independent. No Rust or native execution result is claimed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/native_result.rs](../../../../rust/tcl-compiler/src/command_binding/native_result.rs), `capture_original_read_completion`: Capture existing value-free normal read authority from the actual original Set receiver and selected AfterRead result descriptor.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `walk_source_target_after_observers`: Recheck the read receipt before selecting the intrinsic OK route and retain the existing exact-site quiet handler completion only after actual dispatch.
- [rust/tcl-compiler/src/command_binding/native_result.rs](../../../../rust/tcl-compiler/src/command_binding/native_result.rs), `command_binding::native_result::tests::original_set_read_completion_requires_a_current_defined_quiet_receiver` (linked): Actual native entries retain a defined quiet Set read; undefined, array-root, read-observed and replaced-handler cases cannot acquire its completion.
- [rust/tcl-compiler/src/command_binding/native_list_assignment.rs](../../../../rust/tcl-compiler/src/command_binding/native_list_assignment.rs), `command_binding::native_list_assignment::tests::original_native_assignment_projects_each_store_before_the_next_target` (linked): Genuine C85+ inline assignment retains earlier stores through a later target Set read, including compound index and handler replacement during target evaluation.
