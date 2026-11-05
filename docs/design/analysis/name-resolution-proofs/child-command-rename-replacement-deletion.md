# naming.interpreter.child-command-rename-replacement-deletion

Kind: `native-observation`

## Problem statement

A retained child-table name cannot select the current placement or a newly recreated child command after rename, replacement or deletion.

## Question

What happens to the actual child table and command after the measured rename, replacement, command delete and namespace delete controls?

## Conclusion

All five C controls show replacement or command deletion retires its actual child; a renamed command is removed by interp delete of the original child-table name. A bare root child survives deletion of its creating namespace, while moving the child command into that namespace makes namespace deletion retire it. Recreation preserves a new same-name child and removes the old renamed token. This is finite lifecycle behavior, not a complete teardown callback closure.

## Scope

Exact counted ASCII source passed to Tcl_EvalEx or original Jim_NewStringObj/Jim_EvalObj on a fresh interpreter per source control. C public active-allocation control preserves Tcl_GetSlave actual interpreter through deletion. Finite cases: v1:namespace-rename-delete, v1:replace-child-command, v1:delete-child-command, v1:rename-delete-recreate, v1:namespace-delete-child-command, v2:namespace-renamed-child-deleted-with-namespace, v2:renamed-child-delete-recreate. Result primary/resident fields precede the public getter; retained fields do not independently prove object release/callback closure. No Rust execution or whole lifecycle Normal is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C controls show replacement or command deletion retires its actual child; a renamed command is removed by interp delete of the original child-table name. A bare root child survives deletion of its creating namespace, while moving the child command into that namespace makes namespace deletion retire it. Recreation preserves a new same-name child and removes the old renamed token. This is finite lifecycle behavior, not a complete teardown callback closure.

### jim

Status: `unsupported`. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Version: jim 0.84-9-g5bac7c9.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`.

No appliance observation.

## Consumer bindings

- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_source_controls_match_native_creation_and_retirement_windows` (linked): Compare exact v2 renamed child deletion/recreation source result and guest completion.

## Evidence

- [runtime/rust/tests/data/native_child_command_lifetime/v1](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1)
- [runtime/rust/tests/data/native_child_command_lifetime/v2](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2)
- [runtime/rust/tests/data/native_child_command_lifetime/v1/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture)
- [runtime/rust/tests/data/native_child_command_lifetime/v2/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture)
