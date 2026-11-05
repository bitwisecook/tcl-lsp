# naming.refactor.variable-inline-reassignment-read

Kind: `native-observation`

## Problem statement

A first assignment and a single displayed read do not establish that the first value still reaches that read after a later store.

## Question

Does replacing the read with the first literal value preserve the retained reassignment control?

## Conclusion

All six shells return 2 for set x 1; set x 2; puts $x and 1 for the handwritten replacement set x 2; puts 1. Exact reaching-definition ownership is required independently of the literal value and displayed name.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

file reassigned_original: exit 0, stdout '2\n', stderr ''. file reassigned_naive: exit 0, stdout '1\n', stderr ''. stdin reassigned_original: exit 0, stdout '2\n', stderr ''. stdin reassigned_naive: exit 0, stdout '1\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`.

No BIG-IP appliance result is attached for this precise native source question.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement` (linked): Reject a reassigned original scalar rather than selecting the earlier value by reporting name. This source consumer assertion is independent of the recorded native shell outcomes.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file)
- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin)
