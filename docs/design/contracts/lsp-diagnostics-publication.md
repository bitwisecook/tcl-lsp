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
built by the same `lifted_report` — `published_report`, the document's
report under the policy step, rendered by `lift_report`, the LSP adapter
([diagnostic-policy.md](../compiler/diagnostic-policy.md) § Adapters) — and
the deep push and the pull assemble their findings with the same
`published_findings`, so the two cannot disagree about tags, severity
overrides, or encoding abstention.

### Current completed source reports

`tcl-lsp.currentDiagnostics` is a read-only workspace command over the push
publisher's completion ledger. It accepts one `{uri, version, source}` object
for an open, client-versioned document. It neither schedules a pass nor changes
the diagnostic delivery model.

A `complete` response retains the exact URI, client version and whole source,
the effective dialect retained by the completed pass, server revision, input epoch,
actual final report ID and raw published findings.
The cache/mailbox producer transports its own freshly issued report ID together
with that exact publication acknowledgement; later mutable cache reads cannot
issue publication identity. The query independently requires the cached report
to retain that issued ID.
The publisher issues it only after the final analyser, compiler and applicable
project pass succeeded and the existing notification delivery was acknowledged.
The Compiler pass's checked producer preserves unavailable unit preparation or
solve ownership as `None`; its compatibility empty fallback cannot issue a
completed report.
The query rechecks the current document, full resolved analysis input, resource
configuration epoch, diagnostic scheduling generation, class-factory generation
and report ID. The captured provider Registry snapshot must additionally have
the same complete structural semantic key as the completed analysis's actual
Registry; a foreign metadata store cannot borrow source/input currency. It uses
the configured report before editor display suppression.
The legacy `textDocument/diagnostic` and `workspace/diagnostic` cache has two
explicit producer purposes. A publisher entry retains its original pass and
requires its actual delivered ledger and issued report ID. A direct response
retains the actual checked provider result and its original source/input owner;
it never acquires a push acknowledgement or changes publisher completion. Both
consume one shared analyser/compiler/project/refinement/lift computation, with
unavailable, cancelled or degraded results distinct from successful empty
findings. A direct disabled/excluded outcome records selected policy only,
independently from a delivered suppression receipt.

Each cache entry retains its original source, optional version, server revision,
editing input, actual immutable diagnostic configuration, SourceFile/provider
admission, epoch and generations. The configuration snapshot retains all actual
`AnalyserConfig` fields: options such as declared target ranges are not part of
editing input identity. Final source/input/report validation and cache commit
use one guarded batch and current provider lease. Workspace versions belong to
the reports, never to a later unrelated document snapshot. Closed, missing,
unsupported or unavailable ownership refuses rather than issuing a current
empty report. Unsupported model-provider completion remains separate work.
Neither this compatibility handler nor direct-cache provenance advertises a
standard diagnostic provider; diagnostics remain push-only. The public
source-expectation command still requires a client version and accepts only the
publisher purpose, so direct findings or policy cannot issue its completion.

A completed empty report is different from an unavailable report. The pass
retains its original SourceFile publication admission. The query joins the
actual current SourceFile with that retained receipt and checks the selected
provider after its final await. A newly published equal-path binding cannot
make an earlier pass current.

`suppressed` is a separate receipt for a disabled or excluded provider, issued
only after its actual clearing report was delivered. It retains source, version,
selected resource-policy epoch and report currency, independently of unavailable
analysis preparation. A dedicated configuration-clear consumer may
accept that policy outcome; it cannot satisfy a clean-analysis query.

`unavailable` reports a pending or stale generation, missing context, cancellation,
provider failure, unsupported provider or closed document. Fast-tier output, old cache entries, cleared Problems collections,
log timing and elapsed waits cannot issue completion. Encoding abstention also
cannot issue a complete source report. Clients must preserve this distinction;
a timeout cannot become an empty result. Completion concerns the configured
analysis report and grants no runtime execution, Native identity, semantic
equivalence or absence of unknown effects. Diagnostic policy and applicability
remain independently authoritative.

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
- The tag is decided once, by the policy step (`Outcome::Shown::tag`, from
  `DiagCode::lsp_tag`), and put on the wire by `lift_report`; `lifted_report`
  is the one call every publish path makes (fast tier, deep push, pull
  provider, F5 model report), so they cannot disagree.
- **A code outside the table can never be tagged.** A finding carries a
  `DiagCode`, and a producer that holds its code as a string converts
  fallibly (`TryFrom<&ConfigDiagnostic> for Finding`), so a spelling the
  catalogue lacks never reaches the report — a family that published free
  strings would be silently untaggable, and equally invisible to
  `is_optimisation`, `refined_by_workspace`, the KCS code-table gates and the
  per-code settings surface. Every family we publish therefore has rows in
  `diagnostic_codes!`, including the `XC###` iRule → F5 Distributed Cloud
  translatability family (section `xc`), whose producer carries a typed
  `DiagCode` end to end rather than a string (issue #2121).

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

`SpecTcl` pack-load notices publish outside this pipeline
(`publish_spec_pack_notices` calls `client.publish_diagnostics`
directly), so they carry their own copy of the gate: an excluded URI is
dropped from the batch before the stale-set computation, which both
suppresses new notices and clears previously published ones on the next
pack reload — and adding an exclusion *is* a config apply, which
triggers that reload.

### Encoding integrity and abstention (issue #1326)

`W107` and `W109` answer "are the bytes on disk the text we analysed?". They
come from `DecodeReport`, which records byte evidence and the exact replacement
inserted into lossy text. `W305` answers the related Unicode-text question
"does this source render in a different order from the one Tcl executes?". Its
one canonical producer belongs to the analyser, so LSP, CLI, and MCP consumers
receive it automatically. BIG-IP configuration and iApp APL adapters call the
same pure producer because those formats do not run the Tcl analyser.

When the decode report identifies UTF-16, UTF-32, or binary input, the
policy step's abstention (`DocumentGates::abstain`, from `should_abstain`)
suppresses every finding except the source-integrity set and W305, with the
reason `encoding-abstention`. The decision uses the report, not whether W109
was emitted. Disabling W109
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

- `rust/tcl-lsp-server/src/lib.rs` — `lifted_report`, `published_report`,
  `published_findings`, `lift_report`, `lift_span`, `lsp_severity`
- `rust/tcl-lsp-core/src/diagnostic_policy.rs` — `apply`, `Outcome`,
  `DocumentGates`: the tag, the severity override and the abstention,
  decided once for every surface
- `rust/tcl-core-types/src/diag_code.rs` — the code table, `DiagTag`,
  `DiagCode::lsp_tag`
- `rust/tcl-lsp-core/src/source_decode.rs` — the byte → text decoding
  contract and the W107 / W109 producers
- `rust/tcl-compiler/src/analyser/source_integrity.rs` — the canonical W305
  producer, which filters nothing
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

- [Design doc index](../README.md)
- [compiler diagnostics integration](../compiler/diagnostics-integration.md)
- [async tiering contracts](../compiler/async-diagnostics-tiering.md)

## Editor source actions

Read-only explain and diff commands retain the source document, URI, version,
whole text, selected range and selected text while preparing a model request.
They recheck the captured document bytes after asynchronous model selection and
before sending. A document-backed source uses a dialect-neutral prompt; it does
not borrow the active editor's prompt or treat a configured hint as resolved
analysis context. Inline source has no document identity.


AI source actions retain the actual `TextDocument`, URI components, full source,
version, language, resource-scoped dialect hint, selected extent and source
channel. Source-only discussion can project text; replacing an existing document
requires the original whole-document snapshot. The Node and browser Apply fixes
commands consume a bounded, one-use receipt referring to that retained snapshot.
They check document and analysis currency across editor awaits, require the edit
API's success result and exact replacement/version correspondence, and return a
boolean that immediate callers must check before reporting an applied change.
Creating a new authoring document is a separate operation.

The active client transport issues analysis-context snapshots for its actual
session generation. A stable opaque session token survives provider-wrapper
allocation and changes on lifecycle transitions. Temporary dialect overrides are
serialised, block context capture while pending and retain their acknowledged
owner until cleared or superseded. Operation ownership is independent of analysis
configuration currency, so invalidation can require cleanup without granting a
write context. A superseded or stale lease cannot clear a newer override; a
cancelled acquisition retains responsibility for an acknowledged predecessor.
Only a successful current restore can return the context used to publish a later
source action. These client receipts do not establish the server's independent
source/result applicability or optimiser transformation authority.


### Negative editor diagnostic evidence

The issue969, issue976 and issue977 I230-negative callers wait through the active
extension's existing `waitForCurrentDiagnostics` export before evaluating their
unchanged absence assertions. They retain the actual document returned by
activation and supply its whole source as the independent expected source. The
cross-file issue977 caller also completes its original caller document through
the same owner before opening the library, retaining the original ordering. The
production owner captures actual open-document identity/version/session and
requires an exact complete server response under its existing resource/input,
provider, report and publication checks. No current URI collection sample,
expected count, successive equal diagnostic codes or delivered notification
alone proves diagnostic absence. Complete empty findings remain distinct from
suppressed, unavailable, malformed, stale or provider-error results, which fail
the test rather than becoming an empty array.



The native e2e imported-constant negative W120 control uses the same current
publisher owner through `Lsp::current_complete_diagnostics`, retaining the exact
reader text and client version sent by its original `open_ready` call. Pending
or stale results may retry only within its original single load-scaled 20-second
deadline. Exact URI/version/full-source echo, report identity, revision/input epoch
and a well-formed actual findings array are required before absence is asserted.
Suppressed, provider-error, unsupported, malformed or missing results fail rather
than becoming empty findings. The existing JSON-RPC response waiter supplies one
shared absolute deadline across all queries; repeated pending replies do not
restart or double-scale the budget. This read-only validation route creates no
new endpoint, schedules no analysis and treats notification delivery as distinct
from complete configured diagnostic evidence. Legacy pull helpers remain separate
and supply no completion through a missing-items empty fallback on this route.


### Retained reports in editor requests

`waitForDiagnosticsReport` captures one original open document, whole source,
version and client analysis session. It retains the completed publisher response,
including its actual dialect, revision, input epoch and report ID, separately
from converted display diagnostics. The same complete per-resource `tclLsp`
configuration fingerprint is used by retained diagnostic reports and document
transforms. Unsupported settings cannot establish equality. A source, resource
configuration or session change refuses the original request rather than
recapturing a later context.

Shared contextual model requests, current-report iteration and generic Tcl
validation retain this report through their asynchronous work. Before consuming
it after an await, `currentDiagnosticsReportIsStillPublished` queries the existing
read-only publisher ledger using the original transport and exact original
URI/version/source. Only the same completed dialect, revision, input epoch and
report ID is accepted, with local source/session/settings checked again after
the reply. A newer report does not replace the retained result. Pending, stale,
suppressed, unavailable, malformed and timed-out replies refuse its use.

The report's dialect selects analysis-specific prompt vocabulary. Display-only
diff suppression continues to affect presentation independently; its empty
collection is never used as completed analysis evidence. The array compatibility
adapter preserves its existing retry behavior, while proposal diagnostics keep
their separate hypothetical-source owner. A completed report and its rejoin
supply no permission to edit; source actions still require their existing
checked write or transform owner.
