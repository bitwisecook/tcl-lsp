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
