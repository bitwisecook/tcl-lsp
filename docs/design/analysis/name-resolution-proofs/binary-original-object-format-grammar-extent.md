# naming.binary.original-object-format-grammar-extent

Kind: `implementation-contract`

## Problem statement

Counted binary format storage can contain literal zero, modified zero and later invalid fields. A bulk scan that validates the entire grammar before assigning reached fields loses observable partial assignment and may let a later grammar error replace an earlier assignment refusal. The selected diagnostic unit and error-code store also differ between software C and Jim policies.

## Question

How do the selected binary format grammar and shared reached-field scanner preserve original extent, provider-specific diagnostic units and assignment-before-later-error ordering without manufacturing native execution authority?

## Conclusion

NativeBinaryFormatGrammar retains its independently selected String protocol, uses the shared literal-zero C-string extent and preserves modified-zero bytes as an actual format field. Bad-field diagnostics use the selected C unit encoder or Jim raw first byte; the selected C policy requests NONE publication while the Jim policy preserves the existing error-code state. scan_values_using emits each reached field through the caller before processing the next item. A caller emission refusal returns immediately and prevents later grammar diagnostics. VM binary adapters preserve reached scan assignment, counted format storage and its reference count while keeping selected operation currency independent. These are software contracts and inspected assertion definitions; no external C, Jim or BIG-IP result is supplied.

## Scope

Four annotated Rust source test definitions across Registry grammar selection, the shared binary scanner and VM adapters. Selected C8.4–9.1 and Jim fixtures exercise software policy/storage; they do not execute those external interpreters. Reached result bytes and assignment/refusal ordering have independent owners. A linked source definition is a coverage obligation, not a passing assertion or a native header/cache, handler/frame or completed execution receipt.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This implementation question has no external provider execution receipt. Rust fixture selection and test definitions establish software coverage obligations only.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_binary_value.rs](../../../../rust/tcl-registry/src/native_binary_value.rs), `NativeBinaryFormatGrammar`: Retain independently selected format extent and bad-field diagnostic policy without issuing engine entry.
- [rust/tcl-cmd-core/src/binary.rs](../../../../rust/tcl-cmd-core/src/binary.rs), `scan_values_using`: Emit each reached field through the caller before processing later grammar; preserve the first emission failure.
- [rust/tcl-vm/src/cmd_binary.rs](../../../../rust/tcl-vm/src/cmd_binary.rs), `binary_scan`: Compose the selected grammar and reached-field scanner with original VM assignment and completion owners.
- [rust/tcl-cmd-core/src/binary.rs](../../../../rust/tcl-cmd-core/src/binary.rs), `original_scan_field_is_emitted_before_later_grammar_error` (linked): Shared scanner emits A from a1 before the subsequent invalid ? field; the bulk helper retains its separate error result.
- [rust/tcl-cmd-core/src/binary.rs](../../../../rust/tcl-cmd-core/src/binary.rs), `original_scan_refused_emission_stops_before_next_field` (linked): The first assignment refusal stops scanning after one emission without reaching the later bad-field callback.
- [rust/tcl-registry/src/native_binary_value.rs](../../../../rust/tcl-registry/src/native_binary_value.rs), `original_binary_grammar_distinguishes_literal_and_modified_zero_units` (linked): Selected software release grammars truncate literal zero, retain modified-zero format bytes and preserve C/Jim diagnostic and error-code policy differences.
- [rust/tcl-vm/src/cmd_binary.rs](../../../../rust/tcl-vm/src/cmd_binary.rs), `original_binary_counted_formats_keep_literal_zero_extent_and_reached_scan_assignment` (linked): Software VM format/scan controls retain counted input storage, reached A assignment before modified-zero field rejection, selected error-code policy and unchanged original reference count.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named Rust selectors require a coherent built workspace and independently captured execution receipt before any PASS claim. No native replay or external provider observation is attached to this implementation record.
