# naming.variable.legacy-trace-operation-cstring

Kind: `native-observation`

## Problem statement

The legacy operation object has a counted native string but its letters are scanned as a CString. A raw zero and encoded zero therefore select different operation sets and errors. Treating the display or full count as the operation word changes registration and diagnostics. This question is confined to the measured legacy Tcl trace forms, not copied callback-prefix storage.

## Question

How do legacy trace variable operation words distinguish plain w, w plus raw zero, encoded zero, raw ff and a single binary zero?

## Conclusion

C8.4-C8.6 accepts plain w and w plus raw zero as write; encoded zero and raw ff fail with their exact original bytes in the legacy message. A single binary zero is counted nonempty, then delegates an empty operation list and reports the modern empty-list error. C9 rejects the retired variable/vinfo forms; Jim rejects trace. These results do not prove errorCode, callback execution or prefix-copy semantics.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

Plain and raw-zero suffix w register write; C080 and ff fail byte-exactly; initial raw zero yields modern empty operation-list error.

### tcl9.0, tcl9.1

Status: `unsupported`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Legacy variable and vinfo are rejected as retired options.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated source passed to Jim_Eval. Version: jim 0.84-9-g5bac7c9.

The actual trace command is unavailable: invalid command name trace.

### bigip

Status: `not-tested`.

No appliance observation for this exact question.

## Consumer bindings

- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `parse_legacy_variable_ops_original`: Materialise the original native operation string, then apply the counted-empty and CString letter purposes.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `legacy_variable`: Use the actual original operation object instead of its display string.
- [runtime/rust/src/cmd_trace.rs](../../../../runtime/rust/src/cmd_trace.rs), `legacy_var_add_remove`: Use the independently selected original string getter before shared legacy parsing.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::legacy_operation_objects_match_native_counted_and_cstring_controls` (linked): Compare the measured operation/result bytes and stored legacy trace report for original counted inputs; keep binary zero distinct from C080 and opaque ff.
- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `trace::tests::legacy_operation_extent_preserves_counted_empty_and_opaque_errors` (linked): Compare the measured operation/result bytes and stored legacy trace report for original counted inputs; keep binary zero distinct from C080 and opaque ff.
- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::legacy_operation_objects_match_native_counted_and_cstring_controls` (linked): Compare the observed counted raw-zero, C080 and ff operation errors plus actual stored legacy report on C8.4-C8.6; original operations use selected native getters.

## Evidence

- [rust/tcl-vm/tests/data/native_private_legacy_trace/v2](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2)
- [rust/tcl-vm/tests/data/native_private_legacy_trace/v3](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3)
