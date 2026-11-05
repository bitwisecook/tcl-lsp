# naming.variable.unset-active-read-callback-retirement

Kind: `native-observation`

## Problem statement

An active scalar read callback unsets and recreates the same written scalar. Its old unset observers, old read registration and later read result have separate purposes; a trace setup rejection cannot answer any entered callback behavior.

## Question

For the exact original ASCII scalar probe, does unsetting during an active read invoke the old unset callback, retire the old registrations, and let the later read observe the recreated value without rerunning the old read callback?

## Conclusion

All five captured C releases produce the exact caught result 7 7 {read unset} {}: the old read and unset callbacks are observed once, both reads return the recreated value 7, and the final trace inventory is empty. Current Jim rejects trace registration with invalid command name "trace" at SETUP, so no OBSERVATION row or callback answer exists.

## Scope

One fixed scalar script per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim fresh CLI process. Only the recorded source setup/completion/result sequence is measured. No physical captured receiver, pointer/header epoch, compiler admission, array-element identity, generic increment/container claim or BIG-IP answer follows. Linked C port controls replay the identical complete original source and compare the full observation output after the independently identified PATCHLEVEL line. Linked Jim controls execute only the identical original source prefix through trace SETUP and compare the stored setup code/message with the original SETUP bytes; the unrelated binary formatter and callback body are not replayed. The original complete native source and stdout remain immutable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### jim

Status: `unsupported`. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process. Version: jim 0.84-9-g5bac7c9.

Trace registration has catch code 1 and exact result invalid command name "trace". There is no OBSERVATION row, so active callback execution and retirement are not answered.

### bigip

Status: `not-tested`.

No native interpreter observation establishes this Rust source-advice invariant.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_variable_trace_requires_name`: Keep actual destroyed-cell unset notification separate from active read/write suppression; the CLI result does not itself establish internal receiver identity.
- [rust/tcl-runtime-api/src/native_variable_trace.rs](../../../../rust/tcl-runtime-api/src/native_variable_trace.rs), `NativeVariableTraceOperation::uses_destroyed_cell_callbacks`: Select reached destroyed-cell unset notification separately from active read/write suppression; actual source capture establishes guest behavior only.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_retires_old_registrations_matches_five_native_source_controls` (linked): Compare all five C complete original scalar-source observation outputs after the separate PATCHLEVEL row. Old unset delivery, old trace retirement and later recreated value stay separate from private native receiver/header conclusions.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_probe_preserves_actual_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP, then compare stored setup code 1 and exact invalid-command message with the captured SETUP hex. No binary formatter replay, callback observation or full-script Rust success is claimed.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_matches_five_original_source_controls` (linked): Compare all five C complete original scalar-source observation outputs after the separate PATCHLEVEL row. Old unset delivery, old trace retirement and later recreated value stay separate from private native receiver/header conclusions.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_preserves_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP, then compare stored setup code 1 and exact invalid-command message with the captured SETUP hex. No binary formatter replay, callback observation or full-script Rust success is claimed.

## Evidence

- [rust/tcl-registry/tests/data/native_active_read_unset_original](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0)
- [rust/tcl-registry/tests/data/native_active_read_unset_original/jim](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/jim)
