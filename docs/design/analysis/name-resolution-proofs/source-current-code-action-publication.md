# Original code-action source publication

Ordinary Tcl code actions retain the actual source document, its client version
and raw text, complete DB or fresh-analysis input, source-provider admissions,
input/factory epochs and the actual workspace export generation. The existing
shared publication transaction holds those owners together while it constructs
versioned `documentChanges`. A missing version, closed document, stale or busy
input/index, withdrawn provider or changed source withholds the response.
Strict UTF-16 request positions are checked against the same retained document.
Core still decides whether each original action subject can be edited. Currency
does not supply a selected command, closed effects, Native execution or an edit
subject.

The VS Code language-client converter creates a plain WorkspaceEdit and drops
the received TextDocumentEdit version. Both editor entry points therefore use
one middleware adapter. It captures the actual TextDocument, full source,
version, session and complete per-resource Tcl settings before the provider
await. The server's original source/version metadata must match that capture.
The adapter retains the actual single-document plan, removes the automatic
version-less edit and issues an opaque, bounded, one-use local action receipt.
Unavailable entropy, malformed/missing intent, foreign documents, invalid UTF-16
positions and overlapping plans refuse. Coincident insertions preserve their
supplied LSP order. Metadata is not itself an authentic document receipt.

The action command consumes that receipt through the existing
`replaceDocumentFromSnapshot` writer. The original source/session and complete
resource settings are rechecked before and after editor-opening and admission
awaits; configuration events invalidate the receipt even after a settings
change is reverted. The writer's existing TextEditor edit and returned actual
snapshot own post-edit source/version. Rename continuation opens that same
retained document, validates its original source/session/settings again and
checks its UTF-16 positions before selection. No active editor, bare coordinate
command or guessed post-edit version supplies an intent.

The dedicated BIG-IP dialect adapter remains separate and is explicitly marked
on its existing response. It gains no migration or execution acceptance from
this ordinary Tcl adapter. A missing marker from an unknown server is not this
legacy selection. Generic LSP clients retain the versioned wire; the local
checked-application adapter is specific to the Node/browser integration. Server
input currency is a publication-time lease, not a distributed transaction over
subsequent client configuration changes.

Fixed controls cover the actual Server endpoint with a nonempty Core action,
stale document versions, changed full configuration and export generation,
missing versions, surrogate-interior/out-of-range requests and actual equal-byte
provider replacement. Editor controls cover the actual checked source writer,
original-document selection, one-use receipts, edit/revert ABA, bare rename
coordinates and a source change during provider await. These controls require
ordinary execution; source inspection alone establishes no passing result.


## Current index binding

Stable proof ID: `naming.editor.original-code-action-source-publication`.

## Problem statement

Versionless editor conversion can discard the server source/version intent of a code action.

## Question

How do ordinary server and editor code actions retain their actual source and session through publication and checked application?

## Conclusion

Server publication captures the actual source/input/provider owners and versioned document changes. Existing editor middleware retains that exact plan under a one-use local action receipt and the original checked document writer rejoins source/session/settings before application.

## Scope

Ordinary Tcl action currency and transport only. Core edit-subject/equivalence guards remain independent; dedicated BIG-IP adapters and Native execution are not admitted by this record.

All seven provider answers in the index are not-tested. Test linkage is not test execution. Original narrative above is retained; current owner linkage does not add a runtime observation.
