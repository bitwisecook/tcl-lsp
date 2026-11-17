# Diagnostic subjects

Diagnostic messages and fix descriptions are presentation. Consumers obtain
names from the emitting owner's typed subject; they do not parse quoted text,
regular expressions, or replacement text to recover semantic information.

`tcl_compiler::analyser::DiagnosticSubject` carries a required package key for
W120 and an original unresolved command subject for W123. Package keys retain
the selected package-name bytes and naming policy. Unresolved commands retain
the original source word, lexical configuration, policy, source channel, and
positioned invocation independently of their reporting label. A subject may be
absent. Absence supplies no replacement naming input.

In-process resolution consumes these typed owners directly. Package and
autoload queries require the original caller scope as well as the original
name. A reporting string, source coordinate, or serialized description cannot
establish an invocation, command allocation, current frame, variable contents,
native handler, successful execution, or writable reference.

`tcl_lsp_core::diagnostic_subject::diagnostic_subject_data` is the common
reporting projection for the LSP `Diagnostic.data` field, CLI diagnostic JSON,
and MCP diagnostic JSON. Its version-one envelope contains the diagnostic code
and a tagged subject. Names are arrays of byte values rather than repaired
Unicode strings. The command description includes the original source channel
and byte extent. Deserialization accepts supported reporting data and never
reconstructs the original semantic owner. Missing, malformed, mismatched-code,
or future-version descriptions remain unknown.

## Compiler-check payloads

Every `tcl_compiler::compiler_checks::Diagnostic` carries one typed
`CheckPayload`. `CheckPayloadKind::accepts` owns which family each code
carries, and `CheckPayload::CODES` lists every code the pipeline emits. The
message and fix descriptions are rendered from the producer's fields; no
consumer reads them back.

| Codes | Family | Fields |
|---|---|---|
| O100 | `ConstantBranch` | `value`: the proved condition value |
| O105, O106 | `Redundancy` | `first_span`: the first computation; `expression`: the repeated expression text |
| S100, S101, S110 | `Representation` | `variable`, `from_type`, `to_type`, `command`, `in_loop` |
| S102 | `Oscillation` | `variable`, `first_type`, `second_type` |
| S103 | `SharedCopy` | `variable`, `shared_with`, `command` |
| T100–T106, W313, IRULE3001–IRULE3004, IRULE3101, IRULE3103 | `Taint` | `variable`, `sink_command`, and the retained source sink invocation when the producer has one |
| W201 | `PathConcatenation` | `variable` |
| IRULE1005 | `Irules::EventWithoutCollect` | `event`, `side`, `collect_commands` |
| IRULE1006 | `Irules::PayloadWithoutCollect` | `protocol`, `side` |
| IRULE1007 | `Irules::CollectWithoutRelease` | `protocol`, `side` |
| IRULE1008 | `Irules::ReleaseWithoutCollect` | `protocol`, `side` |
| IRULE1201 | `Irules::ContextAfterResponse` | `command`, `event` |
| IRULE1202 | `Irules::RepeatedResponse` | `command`, `event` |
| IRULE3102 | `Irules::UnnormalisedGetter` | `command` |
| IRULE4002 | `Irules::GenericStaticName` | `source_name`, `pattern_name` |
| IRULE4004 | `Irules::HoistableAssignment` | `variable` |
| IRULE5002 | `Irules::UnguardedDrop` | `command` |
| IRULE5004 | `Irules::UnguardedDnsReturn` | none beyond the code and span |

Names in these fields are the producer's analysis subjects, not native lookup
keys. `compiler_checks::payload_contract_tests` runs one source per code and
asserts that every emitted diagnostic carries the family its code is assigned.

On the wire, `tcl_lsp_core::check_payload::CheckPayloadData` is the
version-one projection, shared by LSP `Diagnostic.data`, `tcl diag --json`
and the MCP diagnostic object through `Finding::structured_data`. It holds the
code, a `kind`-tagged payload with names as byte arrays, and, when the issuer
retained source context for an action, the process-local `sourceAdvice`
handle. Decoding requires version 1, the expected code, and a payload family
that code is assigned; anything else stays unknown. The source sink of a
taint payload does not travel. Each adapter has a round-trip test over
`check_payload::family_fixtures`, which holds one diagnostic per family with
quoted, spaced, qualified, control-character and non-ASCII names.

## Adding a subject

When adding a diagnostic subject, define its data at the emission owner,
retain every independent naming input needed by its consumers, and extend this
shared transport. Verify that changing the message or fixes does not change
semantic behavior, that equal messages can describe distinct subjects, and
that absent data cannot recover a name from presentation. Test protocol
adapters against the same projection. A new compiler-check code takes a
`CheckPayload` family (or a new one), an entry in `CheckPayload::CODES`, a
fixture in `payload_contract_tests` and, for a new family, a variant in
`CheckPayloadData` and in `family_fixtures`.

The same rule holds for the other reports with rendered text. An optimiser
rewrite receipt authorises the issued edit (code, span, replacement, group,
hint flag), not its message. Class-linearisation errors are
`tcl_syntax::mro::MroError` values with a kind and a class, rendered by
`Display`. A SpecTcl loader notice about newer vocabulary carries
`Notice::newer_vocabulary`, and a sandbox denial is recorded on the loader's
evaluation state rather than recovered from the guest error.

`cargo xtask diagnostic-presentation` (in `xtask-check`) fails when
production Rust or the VS Code adapter calls a text-parsing method on a
`message`, `description`, `title`, `new_text` or `newText` field. A
presentation-only read, such as joining punctuation or normalising an edit's
line endings, carries a `// presentation-ok: <reason>` comment.
