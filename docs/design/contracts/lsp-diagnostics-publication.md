# KCS: LSP diagnostics publication model

## Symptom

Editor diagnostics flicker, regress between edits, or differ from expected suppression/severity behaviour.

## Operational context

The LSP layer coordinates analysis output publication, including tiered scheduling and conversion to client-visible diagnostics.

## Decision rules / contracts

1. Publish fast baseline diagnostics first; enrich with deep results asynchronously.
2. Suppression and code-family policy must remain centralized and deterministic.
3. New LSP-facing diagnostic families must map cleanly to existing filtering controls.

### Push vs pull diagnostics

The default mode is **push** (`textDocument/publishDiagnostics`): the
server sends diagnostics to the client after each analysis pass. This is
the mode that the test suite and most client configurations rely on.

**Pull diagnostics** (`textDocument/diagnostic`, `workspace/diagnostic`)
are an opt-in alternative enabled by `tclLsp.features.pullDiagnostics`.
When enabled, the server advertises `diagnosticProvider` in
`ServerCapabilities`, which causes `vscode-languageclient` (and other LSP
clients) to switch to pull mode and stop processing push notifications.

Because handler registration and capability advertisement happen at server
startup, `pull_diagnostics_enabled` is in `_RESTART_REQUIRED_TOGGLES`.
Changing it via `didChangeConfiguration` logs a warning but takes effect
only after the server process is restarted.

### Diagnostic tags (issue #1333)

A diagnostic carries a *presentation tag* — the LSP `DiagnosticTag`
vocabulary — in addition to its severity. `Unnecessary` (1) makes an editor
fade the range; `Deprecated` (2) makes it strike the range through. These are
not severities: an unused parameter is faded *and* stays a hint, because it is
very often deliberate (interface conformance, callback signatures) but should
still be visible.

**The mapping is table data.** It lives on the code's own row in
`tcl_core_types::diag_code`'s `diagnostic_codes!` table (`… , tag:
Unnecessary`) and is read back through `DiagCode::lsp_tag`. Consequences that
are load-bearing:

- Tagging a new diagnostic is a one-token edit to its row, never a `match` in
  a consumer, and never a command-name list.
- The *deprecated-command* codes (`W144`, `IRULE1003`, `IRULE2001`,
  `IRULE2002`) get their strikethrough because those codes are tagged, and
  which commands are deprecated is `CommandSpec` / iRules-registry data. So
  marking a newly-deprecated command stays a spec edit and the strikethrough
  follows for free.
- Tags are attached in `finalise_diagnostics`, the one point every publish
  path funnels through (fast tier, deep push, pull provider), so the three
  cannot disagree.

`Unnecessary` means "you wrote this and nothing reads it" — `W211`, `W214`,
`W220`, `O126`. It deliberately does **not** cover `W210` ("read before set")
or `W213` ("may not exist") despite their adjacency in the W21x family: those
describe genuine defects, and fading a defect hides it.

### Encoding integrity and abstention (issue #1326)

`W107` / `W109` / `W305` answer "are the bytes on disk the text we analysed?"
and are published through the same source-text orchestrator as `W111`/`W112`/
`W115`/`W118` (`tcl_lsp_core::source_style::style_diagnostics`), so they honour
the same `# noqa` / `tclLsp.diagnostics.<CODE>` suppression.

When `W109` fires — the file is UTF-16/UTF-32 or binary, not UTF-8 text —
`apply_encoding_abstention` drops every diagnostic except the encoding set.
That is a deliberate abstention, not a degradation: findings derived from
mis-decoded bytes point at positions that do not exist in the file, and one
accurate diagnostic beats 87 confident wrong ones.

**Byte evidence, never a text guess.** `tcl diag`, workspace reads, and an
unchanged file opened by the editor retain a byte report. They can name the
byte offset and malformed sequence. An unsaved LSP buffer has Unicode text but
not the bytes that produced it, so W107/W109 abstain. In particular, a literal
`U+FFFD` is valid Tcl text and must not become a diagnostic merely because it
resembles lossy decoding.

## File-path anchors

- `rust/tcl-lsp-server/src/lib.rs` — `finalise_diagnostics`,
  `apply_diagnostic_tags`, `apply_encoding_abstention`,
  `apply_severity_overrides`, the `lift_*_diagnostics` family
- `rust/tcl-core-types/src/diag_code.rs` — the code table, `DiagTag`,
  `DiagCode::lsp_tag`
- `rust/tcl-lsp-core/src/source_decode.rs` — the byte → text decoding
  contract and the W107 / W109 / W305 producers
- `rust/tcl-lsp-core/src/source_style.rs` — the source-text orchestrator

## Failure modes

- Stale deep diagnostics publishing after newer edits.
- Inconsistent suppression handling across analyser vs compiler-pass findings.
- Client-facing severity drift after adding new diagnostic families.

## Test anchors

- `rust/tcl-lsp-server/tests/e2e/diagnostics.rs` — push diagnostics over the
  wire
- `rust/tcl-lsp-server/tests/e2e/issue1333_diagnostic_tags.rs` — `tags` in the
  raw `publishDiagnostics` payload
- `rust/tcl-lsp-server/tests/e2e/issue1326_encoding.rs` — the encoding codes
  and the abstention, over the wire
- `rust/tcl-lsp-core/src/source_decode/tests.rs` — the decoder's TP/FP/TN
  matrix
- `editors/vscode/src/test/diagnosticTags.test.ts` — the rendered outcome

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [compiler diagnostics integration](../../../docs/design/compiler/diagnostics-integration.md)
- [async tiering contracts](../../../docs/design/compiler/async-diagnostics-tiering.md)
