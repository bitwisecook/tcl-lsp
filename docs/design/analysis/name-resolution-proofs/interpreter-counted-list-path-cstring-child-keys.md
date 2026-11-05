# naming.interpreter.counted-list-path-cstring-child-keys

Kind: `native-observation`

## Problem statement

A counted interpreter-path operand and its list children can contain raw zero or opaque units. Treating the path as displayed UTF8, as a command namespace or as a single string key would change child selection. These fixed original-object controls compare create, exists, child list leaves, eval and delete; they certify only the measured path/key behavior, not a command implementation or compiler permission.

## Question

How do the tested counted raw-zero, encoded C080, FF and D800 path operands select child keys and return original object/report bytes?

## Conclusion

All five C builds preserve the original create result object, select a CString child key for raw zero, and retain counted encoded C080/FF/D800 child keys. Raw-zero path suffixes can select the same child; children reports the actual shortened child-table key. Jim rejects the named C-style interp API. This does not classify command slots or namespaces, and supplies no callback, compiler, frame or normal-completion capability.

## Scope

Seven fixed paths: counted raw-zero/encoded C080/opaque FF/D800, singleton braced spelling, explicit empty and unmatched brace. Five C releases and current Jim startup/unsupported control. Excludes nested parent paths, options, command-name qualification, hidden commands, aliases, compilation and unknown/custom object getter behavior.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C builds preserve the original create result object, select a CString child key for raw zero, and retain counted encoded C080/FF/D800 child keys. Raw-zero path suffixes can select the same child; children reports the actual shortened child-table key.

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
