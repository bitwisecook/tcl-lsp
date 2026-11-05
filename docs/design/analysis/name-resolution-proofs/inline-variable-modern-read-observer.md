# naming.refactor.variable-inline-modern-read-observer

Kind: `native-observation`

## Problem statement

Replacing a variable read by a literal can erase an observer even when the returned scalar bytes remain equal.

## Question

Does the modern read trace remain observable in the original script while the handwritten literal replacement removes it?

## Conclusion

C8.4 through C9.1 emit OBSERVED then 1 for the modern traced read and only 1 for the literal replacement. Jim reports trace unavailable before the read in both controls. The C comparison proves the observer difference for these exact scripts; the Jim result is measured unavailability, not an observer-free read.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

file modern_watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. file modern_watched_naive: exit 0, stdout '1\n', stderr ''. stdin watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. stdin watched_naive: exit 0, stdout '1\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.0, tcl9.1

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

file modern_watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. file modern_watched_naive: exit 0, stdout '1\n', stderr ''. stdin watched_original: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. stdin watched_naive: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### jim

Status: `unsupported`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: jim 0.84-9-g5bac7c9.

file modern_watched_original: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. file modern_watched_naive: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. stdin watched_original: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. stdin watched_naive: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`.

No BIG-IP appliance result is attached for this precise native source question.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement` (linked): Reject an observed original read; literal byte equality supplies no permission to remove a callback. This source consumer assertion is independent of the recorded native shell outcomes.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file)
- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin)
