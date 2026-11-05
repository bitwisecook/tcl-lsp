# naming.formal.whole-object-return-renaming

Kind: `native-observation`

## Problem statement

A sole formal and a whole-word return may look safe to rename, but equal displayed text does not establish the same result object. Formal/body parsing also has independent byte purposes. The comparison retains one original argument object across both calls and samples result identity before a getter; only the listed ordinary native String inputs and formal spellings are suitable for this observation.

## Question

Do the tested non-raw-zero required formal spellings and their renamed a formal return the same retained original argument object and bytes?

## Conclusion

All six tested interpreters return completion0, the identical retained original argument object and exact counted bytes through both forms for ASCII, genuine UTF8 E9/supplementary, encoded C080, raw ff and D800 formal spellings, across six fresh ordinary String arguments. This supplies finite whole-object return observations; it does not prove arbitrary alpha equivalence, unknown object callback/release behavior, frame elimination, source compiler admission or Rust execution.

## Scope

Public counted original object-vector proc declarations, formal-list/body operands and calls. Body original is return ${<exact formal bytes>}; renamed body is return $a. Excludes raw-zero formal rows and introspection/trace rows, and excludes custom or initially non-String argument classes.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### bigip

Status: `not-tested`.

No appliance capture for this exact counted formal/body/argv question.

## Consumer bindings

- [rust/tcl-vm/src/command/native_formal_alpha_tests.rs](../../../../rust/tcl-vm/src/command/native_formal_alpha_tests.rs), `command::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares original counted proc/formal/body/value operands, guest codes, original result identity before getter and bytes for the six independently selected native VM contexts.
- [runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs](../../../../runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs), `cmd_proc::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares the exact original proc/formal/body/value operands and retained result pointer/bytes in the separately selected Runtime interpreter contexts. No execution result is inferred from this binding.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_alpha](../../../../rust/tcl-vm/tests/data/native_formal_alpha)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0)
- [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim)
- [rust/tcl-vm/tests/data/native_formal_alpha/v1](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v1)
