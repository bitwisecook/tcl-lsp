# naming.diagnostic.typed-subject-reporting

Kind: `implementation-contract`

## Problem statement

W120 package names and W123 unresolved commands can contain opaque native units, while quoted diagnostic wording and suggested fixes are presentation. Parsing that presentation can identify another subject or lose counted bytes; deserializing a reporting payload can also be mistaken for an original source/native issuer. Consumers need to preserve the emitting subject while rejecting unsupported or absent payloads.

## Question

Does diagnostic reporting retain the emitting typed package/source subject independently of presentation, preserve counted bytes and source coordinates, and refuse to turn decoded reporting data into resolution or edit authority?

## Conclusion

The emitting diagnostic supplies its typed subject. The shared versioned readonly projection retains package bytes or original command bytes, policy description, source channel and reporting span without parsing message/fix text. Missing, malformed, future and code-mismatched payloads remain unknown. A decoded description supplies reporting advice only; it recreates neither original source ownership nor a native allocation, lookup/execution or editable-reference grant.

## Scope

Current Compiler-to-Core reporting contract used by protocol adapters. The named tests bind concrete opaque-byte, changed-presentation and absent-payload assertions; no execution of these Rust tests or native-language result is claimed by this record. The structured subject retains its genuine source manufacturer separately from serialized display/coordinates.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This reporting/provenance invariant is a Rust implementation contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `source-owner` (implementation): [rust/tcl-compiler/src/analyser/diagnostic_subject.rs](../../../../rust/tcl-compiler/src/analyser/diagnostic_subject.rs). SHA-256 `b4c829668e1b3f841be184776e79d4e620b33acef2139393912dfff8f1314675`. Current typed diagnostic subject manufacturer or readonly reporting transport. Code provenance is separate from native execution and writable-source authority.
- `reporting-transport` (implementation): [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs). SHA-256 `1168b10e3561f6a7b545dcee386bc278af2caccfe166785e76f2af951236f671`. Current typed diagnostic subject manufacturer or readonly reporting transport. Code provenance is separate from native execution and writable-source authority.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/diagnostic_subject.rs](../../../../rust/tcl-compiler/src/analyser/diagnostic_subject.rs), `SourceUnresolvedCommandSubject`: Retains the genuine emitting source invocation and original lexical name input independently of presentation.
- [rust/tcl-compiler/src/analyser/diagnostic_subject.rs](../../../../rust/tcl-compiler/src/analyser/diagnostic_subject.rs), `DiagnosticSubject`: Carries a typed required-package key or original unresolved command subject.
- [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs), `DiagnosticSubjectData`: Readonly versioned reporting description; decoding supplies no naming issuer or edit grant.
- [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs), `diagnostic_subject_data`: Shared protocol projection preserves the emitting typed subject.
- [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs), `tests::package_transport_ignores_message_and_fixes_but_retains_opaque_bytes` (linked): Changing message/fix presentation preserves the serialized counted package bytes; distinct opaque keys remain distinct.
- [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs), `tests::absent_malformed_future_or_wrong_code_payload_never_recreates_subject` (linked): Absent, malformed, future-version or wrong-code reporting payloads cannot recreate a typed subject.
- [rust/tcl-lsp-core/src/diagnostic_subject.rs](../../../../rust/tcl-lsp-core/src/diagnostic_subject.rs), `tests::unresolved_transport_uses_the_emitting_original_source_owner` (linked): W123 transports original source units/channel and emitting geometry despite a changed diagnostic message.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust tests state diagnostic-subject and reporting assertions. An independent exact-source validation receipt is required to establish their outcome; native guest execution does not verify this reporting invariant.
