# naming.trace.original-type-wrong-arity-inputs

Kind: `native-observation`

## Problem statement

An abbreviated trace type may contain raw zero plus opaque bytes. String reconstruction can panic or print the original abbreviation, while encoded zero can follow a different native lookup path. Source spelling and direct counted object argv must remain distinct.

## Question

For these exact original argv and counted-source controls, which trace type operands reach canonical wrong-arity reporting and which fail native type selection?

## Conclusion

All five C providers select var/com/exec prefixes for direct ASCII, direct raw00FF, ordinary ASCII source and counted source raw00FF, then return guest error1 with canonical variable/command/execution usage headers. Direct C080FF fails type lookup instead. Jim reports trace unavailable in all30 controls. C8.5+ return-options bytes are retained separately; C8.4 and Jim C-return-options API absence is explicit. The thirty raw-zero direct C result controls bind the VM regression; this supplies no Rust pass, trace registration/callback effect or whole options equivalence claim.

## Scope

Thirty finite controls per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9:3type prefixes×2members×5independent input modes. Six builds/processes successful;180 controls and546 protocol rows including version/input/result/options metadata. Direct object argv versus counted source remain separate; no Document decoding or trace callback/registration behavior measured.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands. Version: tcl8.4 8.4.20.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guest code 1 in all thirty. Exact result bytes and explicit no-Tcl_GetReturnOptions markers are retained; return-options bytes were not captured.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guestcode1 in all30; exact result/options bytes retained.

### jim

Status: `unsupported`. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands. Version: jim 0.84-9-g5bac7c9.

All30 guest controls report invalid command trace; no C-return-options API used.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `TraceKind::canonical_name`: Already-selected native type reporting only; actual lookup remains independent.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_add_remove`: Use canonical selected type for add/remove usage headers without String decoding.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_info`: Use canonical selected type for info usage headers without String decoding.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `NativeIndexLookupProtocol::selected_input`: Pure extent of independently selected C Index input, shared by physical lookup; no materialization/cache/registration grant.
- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `resolve_type_bytes`: Pure actual trace table matcher over already selected bytes; original Index lookup remains independent.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::trace_type_wrong_arity_uses_selected_member_without_decoding_original_operand` (linked): Compares only30 original direct raw00FF C code/result controls retained in this capture; no option/callback or executed Rust parity claim.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `native_index_lookup::tests::selected_trace_type_bytes_keep_raw_and_encoded_zero_distinct` (linked): Implementation comparator preserves observed raw00FF versus encoded C080FF selector distinction for independently selected C5 recipe and refuses Jim Index recipe. It grants no getter/cache/registration or Rust pass.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_trace_selected_type](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0)
- [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim)
