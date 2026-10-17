# Live Studio source publication

A native editor-hosted Studio panel acquires a Backend-owned source lease for
its exact `.tclspec` document URI before creating its files. A saved file below
`.tcl-lsp/.spec-studio/` does not by itself select a live Studio override.
The server starts with no active Studio sources. A new Backend never restores
leases from paths or from a previous connection's handles.

The editor calls `workspace/executeCommand` with command
`tcl-lsp.studioSource` and exactly one JSON object in `arguments`:

| Operation | Request fields | Successful reply |
| --- | --- | --- |
| Acquire | `operation: "acquire"`, `uri` | `accepted: true`, `lease: {issuer: [16 bytes], token: u32}` |
| Publish | `operation: "publish"`, `uri`, `lease`, `revision: u32`, `source: string` | `accepted: true` |
| Retire | `operation: "retire"`, `uri`, `lease` | `accepted: true` |

The issuer is the Backend ledger's independently allocated `EditorWireSession`.
It is lossless JSON equality data, not a name, authentication signature,
Native binding or source-currency receipt. Missing randomness withholds
acquisition. Tokens do not repeat within that allocation; a supplied foreign
issuer is refused before token selection. Malformed requests are invalid
parameters; a missing, foreign or stale publication lease returns
`accepted: false`. A known retired lease is retryable only for its exact URI's
withdrawal confirmation and can never publish again. A provider/path/saved-image
refusal remains an explicit protocol error.

Acquisition retains the actual source provider's original URI/path admission.
Its selected Path must be a pack file strictly below one of this Backend's
workspace Studio directories. The file need not exist yet. Acquisition is
inactive. Acquiring the same URI or exact selected provider Path supersedes the
previous lease and withdraws its publication. It cannot grant ownership of any
file, permission to delete it or authority over an independently selected
physical file allocation.

Publication follows a successful, verified save. `source` is the exact UTF-8
saved image; open-buffer text before save does not suffice. A positive
`revision` must increase strictly for that lease. The server compares actual
selected-provider bytes with that image, then retains the complete image with
the original path receipt. It never substitutes newer path contents for the
published source. Changed bytes, missing files or changed provider mappings do
not supply a new publication. Re-registering the same URI/path/bytes in a
virtual source provider cannot authenticate the old provider allocation.

Reload retains the complete Studio ledger generation before discovery and
loading. The checked discovery entry enumerates ordinary configured/workspace,
user and bundled packs with their existing rules, excludes unregistered Studio
files, and selects only active original Studio Paths. The checked loader joins
the exact retained images to those actual discovered Studio records, preserving
metadata such as dependency tiers. A missing image cannot fall back to a read
of that Studio path. Embedded fallback, Tcl evaluation, source/load notices,
cache policy, tier precedence and ordinary pack trust remain the existing
shared owners' responsibilities. The same merge has an unpublished preparation
entry: extension routing no longer escapes during speculative Server loading.
The existing registration publication emits routing inside the final lease,
including an unchanged-content reload. Standalone loaders preserve their eager
routing publication. Includes and provisioned backing dependencies retain the
loader's existing context/digest policy; the root-image lease does not certify
an atomic filesystem snapshot of those independent inputs.

Immediately before publishing, reload joins the same ledger generation and all
retained original provider mappings and exact expected saved images under the
existing analyser-input writer and pack-publication lock. The actual SourceStore
saved-source facet supplies distinct provider contracts: MemoryStore holds its
alias selections and byte map jointly through the callback; Native/WASI reads
actual selected-provider bytes immediately before the callback, without
excluding external writers or promising a continuous/atomic filesystem snapshot.
The callback performs no I/O, awaits or provider reentry. Missing capability is
unavailable. The Server also reborrows saved bytes after publication/refresh
awaits before acknowledging a source publication. Acquiring, publishing or retiring a lease invalidates
an older in-flight snapshot before waiting for the reload mutex. Publication
uses the existing hook, environment, Registry-overlay, Salsa-config and
analyser-input transitions. A changed Studio owner generation is a publication
change even when source bytes and the content key agree. The command refreshes
open-document diagnostics and existing on-demand consumers after reload.
Acquire/publish replies additionally reborrow their original lease, published
source revision and provider admission; concurrent changes can withhold a
positive reply.

Retirement is terminal and independent of the file's continued existence or
path admission. It removes that lease and invokes the same reload/publication
path to withdraw its override. A positive retirement reply additionally
confirms that the actual published-owner inventory no longer contains that
allocation. A stale or failed reload cannot report successful withdrawal;
retirement retains an exact issuer/token/URI tombstone for a genuine retry.
There is no fake filesystem deletion and no filesystem cleanup permission. Server shutdown permanently closes its lease allocator and retires all its
leases through that reload path; a delayed acquire/publish cannot reopen it.
Shutdown reports unavailable if actual withdrawal publication has not completed,
and a repeated shutdown can retry that publication without reopening leases. A panel must retire a late successful acquisition after
its own disposal, serialize saved-source publications, and retire its exact
lease on actual disposal. Reconnection requires fresh acquisition and saved
source publication. Leases have no timer: a missed panel-disposal event on an
otherwise live connection is not detected by this protocol. Such a leak ends
at explicit retirement, same-source supersession or Backend termination.

The standalone `discover`/`discover_in` APIs retain their filesystem-only
compatibility purpose and cannot certify a live editor session. The Server
uses `discover_with_active_studio_in` and
`load_discovered_with_studio_sources_in`. This lease controls the live
metadata-pack set only; sample dialect overrides and generated-output source
provenance retain their own document/session guards. It supplies no Native,
execution, Normal-completion, name-identity, absence or edit permission.
