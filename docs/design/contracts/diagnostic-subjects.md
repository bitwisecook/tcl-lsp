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

When adding a diagnostic subject, define its data at the emission owner,
retain every independent naming input needed by its consumers, and extend this
shared transport. Verify that changing the message or fixes does not change
semantic behavior, that equal messages can describe distinct subjects, and
that absent data cannot recover a name from presentation. Test protocol
adapters against the same projection. The broader diagnostic-family contract
is tracked in [#2426](https://github.com/bitwisecook/tcl-lsp/issues/2426).
