# naming.variable.trace-subject-counted-zero-address

Kind: `native-observation`

## Problem statement

Trace subject names pass through a CString registration purpose, while ordinary variable names and prefix values have separate counted purposes. A raw counted zero can suppress an apparent array suffix; a binary zero object can materialize encoded C080 instead. Conflating these inputs can attach a trace to the wrong root or donate prefix semantics to a subject address.

## Question

How do raw counted v00tail(k), numeric escaped zero and binary00-derived subject values register, report, trigger and remove variable traces, with plain v and a distinct raw-zero suffix as controls?

## Conclusion

All C5 register the raw counted-zero subject on scalar v; plain v triggers that trace and an alternate raw-zero suffix selects/removes it. Numeric escaped zero and binary00-derived values materialize C080 on all C5 and register the distinct element k; plain v does not trigger them and raw-alternate removal leaves them intact. C8.4 differs on the subsequent raw original Set error, which is retained independently. Jim rejects trace registration; its name materialization rows do not establish trace address semantics.

## Scope

Original explicit-count string subjects and independent numeric-escape/binary source-produced subjects, whose reached string bytes are observed before public trace object-vector invocation. C8.4 legacy trace variable/vinfo/vdelete and C8.5+ add/info/remove forms. ASCII setup source has explicit C EvalEx length (Jim Eval uses NUL-terminated source). Callback argument hex and caught guest results retained. No arbitrary object updater, Native variable/header, copied prefix or full read/write-normal grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Version: tcl8.4 8.4.20.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set after plain v errors variable is not array.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set returns VALUE without firing that scalar trace.

### jim

Status: `unsupported`. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Version: jim 0.84-9-g5bac7c9.

All three subject values materialize hex76007461696c286b29, but each trace registration returns guest error invalid command name trace; no subject-address/trigger behavior is claimed.

### bigip

Status: `not-tested`.

No appliance observation for this exact question.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::trace_registration_input`: Pure selected trace subject extent, independent of variable-root and prefix purposes.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::trace_query_input`: Pure selected trace info/removal subject extent without a physical receiver grant.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::trace_subject_registration_keeps_escaped_zero_distinct_from_raw_zero` (linked): Pure selected trace registration/query CString extent distinguishes raw-zero subject from C080 subject while ordinary variable-root extent remains independently selected; Jim trace purpose unavailable.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `variable_bindings::tests::original_trace_subject_keeps_cstring_purpose_separate_from_counted_runtime_lookup` (linked): The selected trace-subject CString purpose clips raw counted zero independently of ordinary counted C9 variable root/index; Jim trace registration remains unavailable. This is a Rust correspondence assertion, not physical registration or current-value proof.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::original_trace_subject_extent_preserves_counted_original_and_independent_array_form` (linked): Selected Registry trace projection keeps complete original subject bytes while its separate CString address extent distinguishes raw-zero from C080 and preserves independent array root/index form. This Rust correspondence binding supplies no physical registration, storage or Normal claim.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/trace-subject-probe.c](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/trace-subject-probe.c)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim)
