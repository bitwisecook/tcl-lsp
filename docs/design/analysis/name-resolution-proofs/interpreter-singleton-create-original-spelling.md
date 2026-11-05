# naming.interpreter.singleton-create-original-spelling

Kind: `native-observation`

## Problem statement

The C creator treats fewer than two list elements differently from its later path resolver: singleton braces remain part of the child name, while GetInterp decodes list elements. An explicit empty path also differs from omitting the path. The check bounds the original spelling/result and subsequent addressing behavior; it does not justify auto-naming an explicitly empty operand.

## Question

For singleton braced and explicit empty original path objects, what name is created, which original/alternate paths address it, and is the original create result retained?

## Conclusion

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child. Jim named-path creation is unavailable. No auto-name, command-publication or arbitrary list equivalence follows.

## Scope

Seven fixed paths: counted raw-zero/encoded C080/opaque FF/D800, singleton braced spelling, explicit empty and unmatched brace. Five C releases and current Jim startup/unsupported control. Excludes nested parent paths, options, command-name qualification, hidden commands, aliases, compilation and unknown/custom object getter behavior.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### jim

Status: `unsupported`. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator. Version: jim 0.84-9-g5bac7c9.

Named C-style path API is rejected with the original wrong-argument outcome.

### bigip

Status: `not-tested`.

No appliance observation for this API question.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::interpreter_child_input`: Pure independently selected CString child-key extent; does not parse command/namespace addresses or prove child existence.
- [rust/tcl-vm/src/interp/native_interpreter_paths.rs](../../../../rust/tcl-vm/src/interp/native_interpreter_paths.rs), `Vm::resolve_interp_path_original`: Actual selected original object-list getter and current alive child-table traversal.
- [rust/tcl-vm/src/interp/native_interpreter_paths.rs](../../../../rust/tcl-vm/src/interp/native_interpreter_paths.rs), `interp::native_interpreter_paths::tests::original_interpreter_paths_keep_counted_objects_and_child_cstring_keys` (linked): Original object-vector C path inputs retain byte keys/result identity and singleton/malformed geometry. No Rust execution or full native handler/CPP grant is inferred.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::original_interpreter_paths_preserve_child_keys_and_create_result_objects` (linked): Original counted C path objects preserve create-result allocation, CString child selection, singleton source spelling and malformed-list errors through actual runtime tables; this is a binding, not an executed comparison receipt.

## Evidence

- [rust/tcl-vm/tests/data/native_interpreter_paths](../../../../rust/tcl-vm/tests/data/native_interpreter_paths)
- [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20)
- [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19)
- [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18)
- [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4)
- [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0)
- [rust/tcl-vm/tests/data/native_interpreter_paths/jim](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim)
