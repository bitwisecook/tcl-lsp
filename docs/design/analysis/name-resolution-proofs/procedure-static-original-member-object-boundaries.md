# naming.procedure-static.original-member-object-boundaries

Kind: `native-observation`

## Problem statement

Converting a static declaration list to UTF-8 or rebuilding literal members can merge counted names and lose the initializer object. Value copy and raw-cell reference capture have distinct definition-time meanings.

## Question

What happens for the finite original-list literal, opaque copy/reference and atomic rejection cases under the selected Jim and C Tcl procedure grammars?

## Conclusion

Jim accepts the five counted names and retains each original nested-list literal value. Opaque copy versus reference observes FIRST versus SECOND after the actual source cell replacement. Missing and duplicate declarations fail without installing p. C8.4–9.1 reject the extra static-list operand. These observations do not grant source compilation, callback/release closure or a missing linked target.

## Scope

Five literal names x, xFF, xD800, xD801 and x/raw-zero/tail; actual original nested-list value, original pointer identity before sole result string getter, opaque xFF copy/reference replacement, missing and duplicate rejection. Independent original object-vector and ASCII query channels are retained. No arbitrary wrapper linkage, arrays, release callbacks, object cache layout, source minifier or BIG-IP behaviour is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five literal forms plus copied/reference/missing/duplicate four-operand procedure definitions return code 1 with the recorded wrong-argument diagnostic. No procedure p is installed by the failed definitions. These are static-list grammar availability negatives; body execution is not attempted.

### jim

Status: `observed`. Channel: Counted original native String/list object vectors and retained result pointer before getter; independent ASCII version and command-presence queries. Version: jim 0.84-9-g5bac7c9.

All five listed counted literal names define successfully, and calls return the identical original nested-list initializer object. The opaque copied initializer returns FIRST after source replacement while raw reference returns SECOND, both by original result identity. Missing and duplicate initializers fail and leave p absent.

### bigip

Status: `not-tested`.

No appliance execution is attached.

## Consumer bindings

- [rust/tcl-vm/src/command/native_static_original_tests.rs](../../../../rust/tcl-vm/src/command/native_static_original_tests.rs), `command::native_static_original_tests::original_static_member_objects_match_native_counted_names_and_value_capture` (linked): Recreates original native list members and counted names; compares all recorded declaration codes/messages, literal/copy/reference result identity and bytes, plus atomic missing/duplicate command absence, in six independently selected VM contexts.
- [runtime/rust/src/cmd_proc/native_static_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_static_original_tests.rs), `cmd_proc::native_static_original_tests::original_static_member_objects_match_native_counted_names_and_value_capture` (linked): Separately compares original Runtime object pointers, counted declaration/value bytes and atomic failed publication against every listed provider row. No Rust execution result is inferred.

## Evidence

- [rust/tcl-registry/tests/data/native_procedure_static_original/probe.c](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/probe.c)
- [rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.4.20)
- [rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.5.19)
- [rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/8.6.18)
- [rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.0.4)
- [rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/9.1.0)
- [rust/tcl-registry/tests/data/native_procedure_static_original/jim](../../../../rust/tcl-registry/tests/data/native_procedure_static_original/jim)
