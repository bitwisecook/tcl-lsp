# LSP diagnostics publication model

How analysis output reaches the editor: tiered scheduling, the conversion to
client-visible diagnostics, and the ordering and suppression rules that keep
them from flickering or regressing between edits.

## Decision rules / contracts

1. Publish fast baseline diagnostics first; enrich with deep results asynchronously.
2. Suppression and code-family policy must remain centralized and deterministic.
3. New LSP-facing diagnostic families must map cleanly to existing filtering controls.

### Push vs pull diagnostics

**Push (`textDocument/publishDiagnostics`) is the sole delivery channel.**
The server sends diagnostics to the client after each analysis pass.

`build_server_capabilities` sets `diagnostic_provider: None`
unconditionally, and `initialize` stores
`client_supports_pull_diagnostics = false` regardless of what the client
advertised. Both are deliberate, and they are two halves of one rule:

- Advertising `diagnosticProvider` makes `vscode-languageclient` (and most
  other clients) switch to pull mode and stop honouring push notifications,
  which silently disables the richer push pipeline and renders each
  diagnostic twice (issue #721).
- The `client_supports_pull_diagnostics` suppression ("stop pushing when the
  client pulls") must therefore stay **off**. A pull-capable client — VS Code
  advertises the capability whether or not it will use it — would otherwise
  get neither push (suppressed) nor pull (unadvertised): zero diagnostics.

The `textDocument/diagnostic` and `workspace/diagnostic` handlers remain
implemented and answer correctly, backed by `pull_diag_cache`, for clients
that request them directly. Pull is intentionally not exposed as a contributed
editor setting: changing the delivery model requires a different capability
advertised during `initialize`, and cannot be switched safely by a later
configuration refresh without a server restart and client reinitialisation.

Whatever the delivery channel, a pull response and a push notification are
built from the same `finalise_diagnostics` path, so the two cannot disagree
about tags, severity overrides, or encoding abstention.

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

### Diagnostics exclusion by glob (issue #1556)

`tclLsp.diagnostics.exclude` is a second, coarser gate alongside the
`tclLsp.features.diagnostics` master switch: instead of turning
diagnostics off everywhere, it turns them off for files whose path or
name matches one of the configured glob patterns. The list is resolved
per workspace folder from the layered config (project `.tcl-lsp.ini`
overriding editor settings overriding the global `config.ini`), the
same way any other `[diagnostics]` key is. A file with no owning
folder can only match a no-`/` name pattern, since there is no folder
root to relativise a path pattern against.

The check sits beside the master switch: `run_diagnostics_core` tests
it immediately after `toggles.diagnostics_enabled`, and
`full_diagnostics_for` repeats it on the pull path, so push and pull
agree. Either gate short-circuits analysis and publishes an empty
diagnostic set, which is what clears existing squiggles on the next
publish — the file is still opened, indexed, and available to every
other feature. A match logs `[timing] diagnostics excluded 0ms
(uri=..., diags=0)` in place of the usual timing line, so the
exclusion is visible in the same place a slow analysis would be.

### Encoding integrity and abstention (issue #1326)

`W107` and `W109` answer "are the bytes on disk the text we analysed?". They
come from `DecodeReport`, which records byte evidence and the exact replacement
inserted into lossy text. `W305` answers the related Unicode-text question
"does this source render in a different order from the one Tcl executes?". Its
one canonical producer belongs to the analyser, so LSP, CLI, and MCP consumers
receive it automatically. BIG-IP configuration and iApp APL adapters call the
same pure producer because those formats do not run the Tcl analyser.

When the decode report identifies UTF-16, UTF-32, or binary input,
`apply_encoding_abstention` drops every diagnostic except the source-integrity
set. The decision uses the report, not whether W109 was emitted. Disabling W109
therefore hides the explanation but never re-enables claims about mis-decoded
content.
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
  contract and the W107 / W109 producers
- `rust/tcl-compiler/src/analyser/source_integrity.rs` — the canonical W305
  producer and suppression filter for non-Tcl adapters
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
