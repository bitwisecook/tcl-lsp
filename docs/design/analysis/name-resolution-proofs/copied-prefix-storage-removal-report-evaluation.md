# naming.variable.copied-prefix-storage-removal-report-evaluation

Kind: `native-observation`

## Problem statement

A trace prefix containing a counted zero may have different storage, removal, reporting and execution extents. Reusing one byte equality or retaining the original prefix object can make removal or later callback arguments differ from the actual interpreter. The public object-vector inputs distinguish a raw zero from encoded C080, equal versus unequal counted lengths, and mutation of the caller-owned prefix after registration.

## Question

Does variable trace registration copy a counted raw-zero prefix, and do removal, reporting and callback evaluation use the same extent?

## Conclusion

All five tested C releases copy the full prefix without retaining its input object, report only watch A, and evaluate a callback whose first argument is the full counted A00X. A same-length A00Y prefix removes it; a longer A00YY or encoded C080 prefix leaves it registered. C8.4 uses legacy trace syntax and reports callback operation w; C8.5 through C9.1 use the modern form and report write. Current pinned Jim rejects trace. These observations establish four distinct extents for the exact selected variable trace purpose.

## Scope

Public object-vector trace registration/removal, fresh interpreters, an original nine-byte prefix watch<space>A00X, public prefix-object mutation after registration, ASCII top-level source, and direct callback argv byte observation. Callback argument rendering is observed through its counted public string getter. This does not measure command/execution trace removal, callback bytecode cache, arbitrary object class, variable cell grants or BIG-IP trace behavior.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Registration leaves the input at refCount1; changing it does not change copied reporting or callback A00X. Same-length raw-zero removal leaves0 registrations; longer and C080 negatives each leave1.

### jim

Status: `unsupported`. Channel: Counted Tcl_NewStringObj input arrays passed to Tcl_EvalObjv; ASCII NUL-terminated C source passed to Tcl_Eval for firing and reporting. Jim uses ASCII Jim_Eval only for trace availability. Version: jim 0.84-9-g5bac7c9.

Guest Error: invalid command name "trace"; no prefix law is measured.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_storage`: Selected immutable counted extent, independently of registration and cell.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_matches`: Independent length-plus-strncmp removal selector.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_prefix_report`: CString result projection.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `NativeVariableTraceProtocol::variable_callback_source`: Counted assembled script extent.
- [rust/tcl-registry/src/native_variable_trace.rs](../../../../rust/tcl-registry/src/native_variable_trace.rs), `native_variable_trace::tests::variable_prefix_copy_remove_report_and_counted_eval_keep_independent_extents` (linked): Check same-length strncmp match, unequal length and C080 negative, copied byte extent and independent report/callback extents.
- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls` (linked): Register original object, mutate input, fire counted copied argument and compare all three removal outcomes.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::variable_trace_prefix_copy_and_purpose_extents_match_public_native_controls` (linked): Copied prefix survives original representation invalidation, reports CString and fires counted argument with independent removal negatives.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `variable_bindings::tests::original_variable_trace_copies_producers_and_removes_by_selected_native_purpose` (linked): Checks compiler copied original prefix lineage and selected same-length native removal purpose; Rust lifetime ownership remains an independent obligation rather than a native trace callback guarantee.

## Evidence

- [rust/tcl-registry/tests/data/native_variable_trace_prefix/probe.c](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/probe.c)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.4.20/stdout.tsv)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.5.19/stdout.tsv)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/8.6.18/stdout.tsv)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.0.4/stdout.tsv)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/9.1.0/stdout.tsv)
- [rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_variable_trace_prefix/jim/stdout.tsv)
