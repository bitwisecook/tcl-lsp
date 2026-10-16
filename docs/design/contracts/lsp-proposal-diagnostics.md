# LSP proposal diagnostics

This contract defines how the language server analyses proposed full-document
source for an already-open document, and how the editor keeps that analysis
separate from edits to the original document.

## Protocol

The editor invokes `workspace/executeCommand` with
`tcl-lsp.proposalDiagnostics` and one argument containing the original `uri`,
integer `version`, exact original `source`, and complete `proposal` source.

The response is one of three distinct outcomes:

| Status | Fields | Meaning |
| --- | --- | --- |
| `complete` | `uri`, `version`, `source`, `proposal`, opaque `revision`, `inputsEpoch`, `workspaceIndexGeneration`, `resultId`, `dialectId`, `diagnostics` | The exact requested proposal completed against the retained source and analysis inputs. An empty `diagnostics` array is a completed empty result. |
| `suppressed` | `uri`, `version`, `source`, `proposal`, `reason` (`disabled` or `excluded`) | The source and configuration were revalidated and diagnostics were intentionally suppressed by the current diagnostic policy. It is not a clean result. |
| `unavailable` | `reason` (`pending`, `stale`, `context`, `providerError`, `cancelled`, `unsupported`, or `notOpen`) | The requested analysis could not establish a current complete result. It is not an empty result. |

The server does not schedule analysis from this command or use push
publication as completion evidence. The `dialectId` is the effective resource
dialect captured by the producer and is serialized only inside the final
source-store currentness closure, alongside the matching diagnostics and
`inputsEpoch`. The proposal `revision` is opaque
structured metadata; it is not an editor-facing version and is distinct from
the ordinary current-diagnostics revision contract.

The request flows through these existing owners:

| Boundary | Owner | Responsibility |
| --- | --- | --- |
| Execute-command route | `Backend::proposal_diagnostics_command` | Decode one strict `{uri, version, source, proposal}` request and serialize the typed outcome. |
| Source-bound producer | `Backend::produce_proposal_diagnostics` | Admit the open original document, capture its source/context, and revalidate the completed result. |
| Isolated project | `proposal_diagnostics::build` | Create a detached database and remap the retained project/configuration inputs with only the selected full source replaced. |
| Provider provenance | `ProposalProviderSources` and `ConfiguredProviderSourceReport` | Retain each actual provider capture before consumption and join its currentness to the result. |
| Source syntax | `SourceSyntaxCoverage` | Establish the selected source's syntax coverage only; call closure and dispatch remain separate owners. |

## Server ownership

`Backend::proposal_diagnostics_command` decodes the request and delegates to
`Backend::produce_proposal_diagnostics`. The producer requires the open
versioned document to match both the supplied original source and URI. It
retains the actual `SourceFile`, document admission, resolved input epoch,
configuration, project/index, and provider receipts through final currentness
validation.

The producer creates `ProposalProviderSources` before base, recovery, or deep
analysis can consume provider data. Each actual provider capture, including
recovery and cache-hit captures, joins its request-local ledger. The deep
report's `ConfiguredProviderSourceReport` is built from that authentic ledger;
provider loss or lock poisoning remains unavailable. If analysis would consume
the selected original source as an external provider or would require an
unversioned disk provider, the request is unavailable rather than borrowing
old text or treating missing evidence as absence.

Proposal analysis uses a separately allocated `TclDatabase`, not a clone of
the live Salsa database. It remaps the captured project membership and
per-file configuration mapping, all eight `SourceFile` evidence fields and
all eight `AnalyserConfig` fields, replacing only the selected document's full
text. The projection retains actual admitted path values; it does not recreate
paths from URI display text or a language hint. Project-wide factory,
call-site, subclass and recovery evidence is recomputed for the projected
source/index, with proposal recovery names kept in a request-local cache.

The shared `SourceSyntaxCoverage` receipt establishes only source-body syntax
coverage. It does not establish call closure, dispatch, absence, Native lookup,
execution or edit permission. Existing source-target and call-site owners
retain their own dynamic and unknown-source limits. Final completion rechecks
the original source and admission together with its configuration, project,
index, resolver and evaluator/overlay epochs and captured provider admissions
under the source-store currency boundary.

## Editor and edit boundary

The editor captures one original URI, version and full source before waiting
for proposal diagnostics. Pending and stale responses may be retried only
against that same snapshot. A changed document, timeout, unavailable result,
malformed result or suppression cannot be converted into an empty diagnostic
array. A complete empty array is meaningful only for that completed proposal.

For an existing document, the agentic loop edits a separate proposal string
and analyses that proposal without intermediate writes to the original buffer.
The Node `/fix` caller uses its first complete report for both its initial
diagnostics and `buildPrompt(report.dialectId)`. It binds later reports to the
same dialect and input epoch. A retained initial report cannot be combined
with a dialect override; the editor does not sample a separate resource hint
to choose the prompt.
The retained report is evidence about that proposal as of the captured
original document version. Its revision is kept in structured data and is not
presented as a user-facing token.

Applying the proposal remains the responsibility of the existing checked
`DocumentWriteReceipt` owner, which verifies the original source and context.
The chat command carries report data and the serialised receipt; it does not
carry live document, session or transport objects. Proposal analysis does not
authorise an edit and does not promise that its report remains current at a
later click. New-document authoring remains a separate purpose and receives no
existing-name identity or overwrite authority from this protocol.
