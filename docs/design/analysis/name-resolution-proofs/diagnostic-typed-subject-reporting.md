# naming.diagnostic.typed-subject-reporting

Kind: `implementation-contract`

## Problem statement

W120 package names and W123 unresolved commands can contain opaque native units, while quoted diagnostic wording and suggested fixes are presentation. Parsing that presentation can identify another subject or lose counted bytes; deserializing a reporting payload can also be mistaken for an original source/native issuer. Consumers need to preserve the emitting subject while rejecting unsupported or absent payloads.

## Question

Does diagnostic reporting retain the emitting typed package/source subject independently of presentation, preserve counted bytes and source coordinates, and refuse to turn decoded reporting data into resolution or edit authority?

## Conclusion

The emitting diagnostic supplies its typed subject. The shared versioned readonly projection retains package bytes or original command bytes, policy description, source channel and reporting span without parsing message/fix text. Missing, malformed, future and code-mismatched payloads remain unknown. A decoded description supplies reporting advice only; it recreates neither original source ownership nor a native allocation, lookup/execution or editable-reference grant.

Typed Report transport preserves original counted subjects independently of message prose. Unavailable source generations remain an explicit input-refusal payload through empty finding sets and protocol publication; a successfully analysed empty document is a different state.

## Scope

Current Compiler-to-Core reporting contract used by protocol adapters. The named tests bind concrete opaque-byte, changed-presentation and absent-payload assertions; no execution of these Rust tests or native-language result is claimed by this record. The structured subject retains its genuine source manufacturer separately from serialized display/coordinates.

These three controls bind Core/Server software data transport and refusal only. Their source walk/status and structured DTO do not grant Native lookup, handler/body/frame entry, runtime name identity, edit eligibility or assertion outcome. The original semantic issuer remains independent of display text and serialized coordinates.

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

- [rust/tcl-lsp-core/src/diagnostic_report.rs](../../../../rust/tcl-lsp-core/src/diagnostic_report.rs), `standalone_findings`: Resolve one complete explicit standalone input before the source walk and retain typed unavailable-generation refusal rather than empty-clean findings.
- [rust/tcl-lsp-core/src/diagnostic_report.rs](../../../../rust/tcl-lsp-core/src/diagnostic_report.rs), `document_report_with_analysis`: Carry actual retained analysis and its typed unavailable status into Report policy without reconstructing source ownership from findings.
- [rust/tcl-lsp-core/src/diagnostic_policy.rs](../../../../rust/tcl-lsp-core/src/diagnostic_policy.rs), `Finding::structured_data`: Project the original typed analyser or compiler subject directly, retaining counted units independently of diagnostic message wording.
- [rust/tcl-lsp-core/src/diagnostic_policy.rs](../../../../rust/tcl-lsp-core/src/diagnostic_policy.rs), `Report::retain_analysis_context`: Keep actual typed analysis-context refusal alongside finding policy; empty finding sets do not establish successful source input.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `lift_report`: Publish the retained original structured subject and explicit unavailable-generation payload; message text is presentation only.
- [rust/tcl-lsp-core/src/diagnostic_report.rs](../../../../rust/tcl-lsp-core/src/diagnostic_report.rs), `diagnostic_report::tests::an_unavailable_overlay_is_not_a_successfully_empty_report` (linked): A typed unavailable overlay produces no source walk and remains explicit Report input refusal, independently of a genuinely successful empty document/report. Empty findings cannot erase unavailable status.
- [rust/tcl-lsp-core/src/diagnostic_policy.rs](../../../../rust/tcl-lsp-core/src/diagnostic_policy.rs), `diagnostic_policy::tests::report_retains_typed_subjects_independently_of_diagnostic_presentation` (linked): Original counted RequiredPackage subject bytes survive changed diagnostic prose and Report policy conversion; the retained original analyser diagnostic and structured DTO share the same semantic subject.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::report_publication_retains_unavailable_generation_status` (linked): The actual protocol publication adapter emits an explicit analysisContextUnavailable payload for retained OverlayMiss even when there are no findings; a successful empty Report remains distinct.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

The linked Rust tests state diagnostic-subject and reporting assertions. An independent exact-source validation receipt is required to establish their outcome; native guest execution does not verify this reporting invariant.
