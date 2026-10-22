# N13 retained source views: proposed API and end-to-end flow

Status: off-tree design against active8478a7b5; no implementation, compilation,
protocol execution or N13 admission. The constants/settlement drafts alone do
not close this work. Compiler-names confirmed the compiler API direction on
2026-10-22; its issuer/currentness implementation remains queued.

## Authentic compiler boundary

The existing carrier is `command_binding::SourceAnalysisEntry`. Do not build a
`NativeCompilationEntry`, use a Loaded origin, or reconstruct an inherited entry
with `for_supplied_source`/a namespace string.

Compiler-names proposes:

```rust
impl OriginalProjectSourceLoad {
    pub fn source_point_entry(
        &self,
        parent: &AnalysisResult,
    ) -> Option<OriginalSourcePointEntry>;
}
impl OriginalSourcePointEntry {
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool;
}
impl Analyser {
    pub fn analyse_with_source_point_entry(
        &mut self,
        source: &str,
        dialect: &str,
        point: &OriginalSourcePointEntry,
    ) -> Option<AnalysisResult>;
}
```

These are proposed signatures, not callable APIs yet. The opaque receipt retains
actual parent analysis attachment plus exact original pre-handler snapshot,
image, full input/configuration, source invocation and frame. Its private issued
token is usable for Eq/Hash/Ord only after full-owner matching. The child has a
source-only `InheritedPoint` origin with an immutable privately issued token and
exact child image. The full strong receipt remains in the actual child
`SourceAnalysisEntry`/options, retained analysis and every child CU. The token
alone grants neither entry reconstruction nor currentness; escaped identities
must rejoin the same retained view/full receipt or refuse. A stale/ambiguous/incomplete issuer or incompatible child input refuses.
There is no standalone retry.

A current-interpreter load whose issuer returns None remains an explicit
unavailable required edge/view. Omitting it from the desired set would falsely
turn refusal into settlement. Its original path/load card remains available
only for its independent reporting purpose.

Global namespace does not imply standalone entry. The current
`is_standalone_view`/`::` optimization must not be carried into the authentic
point route: root namespace source points still differ by parent variables,
namespace paths, imports, command lifecycle and origin. A real source edge in
`::` must retain its authentic entry/parent. The older namespace-only settlement
draft keeps this shortcut and cannot be applied as an N13-complete repair.

The existing compiler-owned `AnalysisResult::original_optimiser_source_unit(source,
registry, external)` is the proposed CU consumer. Its private
`OriginalOptimiserSourceOrigin::rebuild_for_current_analysis` checks exact child
image, full input, registry snapshot, external evidence and evaluator revision.
The RetainedEntry branch passes `Some(&self.0.entry)` and preserves declared
surface/options; a final original-origin equality check rejects a changed
producer. Core `original_invocation::source_compilation_unit` currently constructs
`for_supplied_source`, losing the original entry. Replace that reconstruction with
the existing retained-CU API, with no standalone fallback. A separately exposed
entry getter is unnecessary unless measured retained-owner availability proves a
narrow additional reborrow is required. This is a proposed consumer change, not
a measured availability claim; ordinary source assistance and external-evidence
cases still need the existing regressions.

Compiler-names' precise off-tree API contract is
`/tmp/compiler-names-n13-api-contract.md` (SHA-256
`96f1e780d0034ef343c8793b26f9c7fa99bebfc3cc7cd6a69c8a991842373c9f`).

## Host records and proposed APIs

A physical source and a source interpretation are separate records. Multiple
views may share one URI and source image. Physical text/line-index storage can
be shared; each original analysis, opaque entry, realm and publication owner is
retained independently.

| Proposed host/index type or method | Actual purpose |
| --- | --- |
| `OriginalSourceViewId` | Index/publication-issued, non-reused view coordinate; no naming or runtime authority. Never reconstructed from URI/text/hash/namespace. |
| `PublishedOriginalSourceView` | Strong actual AnalysisResult, exact source/input/configuration, path admission and publication lease, plus optional strong parent view and authentic point receipt. |
| `OriginalSourceEdgeView` | Actual parent view ID, retained OriginalProjectSourceLoad/point, original path operand and selected physical child path admission. |
| `WorkspaceIndex::replace_original_source_views(uri, views)` | Atomically replaces every view of this physical URI, retains IDs only for the same still-current publication/producer, allocates fresh IDs for new publications, and returns per-view contexts. |
| `WorkspaceIndex::original_source_views(uri)` | Lends retained per-view analyses/inputs/contexts and their issued IDs; does not analyse source again. |
| `OriginalViewInventory` | One frozen query inventory of exact current published views and physical/source dependency leases, captured after materialisation/reconciliation. |
| `OriginalViewDeclarationSelection` | Existing genuine OriginalDeclarationIdentity paired with the actual inventory/view owner selected by the semantic producer. |
| `prepare_in_views` / view-aware references and hierarchy engines | Preserve the selected owner through lookup and target lifting; do not find the first document with the target URI. |

This table describes proposed interfaces; no new field is added to every legacy
`OriginalDeclarationDocument` literal. A view-aware carrier wraps that existing
document plus its issued view ID. Legacy standalone API wrappers can remain.

`PublishedOriginalSourceView` retains:

- child physical URI and `DocumentPathAdmission` (actual chosen source path,
  VFS/browser/WASI origin where present), source image/channel and actual text;
- actual `Arc<AnalysisResult>`, complete retained analysis input, lexer config,
  diagnostic configuration, and the same source analysis entry/derived origin;
- open revision/version or closed `SourceFile`/database source observation,
  publication URI generation, effective config/pack and class-factory owners,
  input epoch, and the exact view publication ID/revision;
- for inherited views, a STRONG parent-view link and the authentic source-point
  receipt; that parent recursively retains its own physical/publication owner;
- the source-edge/path association selected from this exact parent point.

A parent URI/path lease remains required even if opaque entry equality happens
to be shared through an allowed compiler memo. Equal parent bytes/full input do
not establish that A's path owner belongs to B. A private host constructor couples
parent view+point before the child is built; callers cannot swap the physical
parent later. Pointer/hash equality never replaces these checks.

The view sequence belongs to the live Backend/publication owner and cannot reset
when a WorkspaceIndex is rebuilt or a URI is removed. Checked exhaustion refuses
new issuance; no wrapping. A number alone supplies no ownership. Each lookup
rejoins the exact live table record and its full saved publication owner. A
retired coordinate is never reused, including after equal-byte restoration.

An unchanged live view must retain the SAME actual analysis attachment and opaque
entry across reconciliation. Reanalysing it every round would manufacture new
parent attachments/receipts and prevent full-state convergence. The child cache
must therefore compare the authentic parent point plus exact child physical
publication/full configured input, and reuse only that still-current stored
analysis. URI/text-only cache hits cannot satisfy this. Retirement invalidates
eligibility even if an old Arc remains retained by a wire handle.

## Producer and atomic publication

1. Retain a root view from its actual already-built fresh/DB analysis and actual
   path/publication source receipt. This is a storage operation; do not rerun
   standalone analysis merely to enumerate declarations.
2. `index_sources` still has that AnalysisResult. For each genuine current-
   interpreter `OriginalProjectSourceLoad`, call the compiler issuer there.
   Store both the source-load path card and the opaque point, associated with
   this exact parent VIEW ID. Child-interpreter visibility remains separate.
   Missing point is explicit unavailable; a namespace label cannot fill it.
3. Resolve the path with the same complete constants/edge state and original
   source-path owner. Capture the child physical admission independently.
   Work is keyed by parent view/site + selected child URI/admission + opaque
   entry, not by a namespace set or just the child URI.
4. Build the child under its OWN configured input/path via the proposed authentic
   analyser ingress. Retain the returned analysis and same SourceAnalysisEntry.
   If construction refuses, no fresh/global/namespace-string analysis is used.
5. A complete desired child-view batch is published under the existing source,
   inputs and rehoming transaction leases. Revalidate parent ancestry, path
   admissions, child source and complete configuration BEFORE committing.
   The index retains each view's actual analysis/input/realm and header owners.
   `DocumentRecords` cannot overwrite all worlds/inputs with the last view's
   scalar context while keeping every earlier header in one flat vector.
6. Publish the host view table and index view IDs in that SAME transaction.
   Retire old view IDs/edges before the reconciler can report Settled. A failed
   or pending batch cannot expose half-new views as current. Fresh standalone
   publishers cannot erase an inherited view behind its retained ID.
7. Recompute the full desired state until the exact view/edge state agrees.
   Cycles, unavailable entries, stale sources and pending publication remain
   explicit; no fixed round limit or partial equality becomes completion.

The retained analyses avoid both `publish_rehomed_if_current` dropping them after
indexing and `original_declaration_documents` later deduplicating URI/reanalysing
standalone. Those current methods must be replaced at the view-aware boundary.

## Provider and retained-handle flow

`OriginalViewInventory` captures every live view once, including distinct equal-
namespace or same-URI views. Queries on a physical URI evaluate its genuine views
independently. A readonly Location may deduplicate identical physical ranges
AFTER each owner-aware selection. Hierarchy/lens handles retain the actual view
selection; they do not discard it when two presentation labels happen to match.
A declaration cursor may therefore yield multiple genuine view-owned items.

Core target engines carry the selected inventory owner alongside identities.
`procedure_target`, math/member targets, reference edges and hierarchy item
lifting cannot later call `.find(doc.uri == target.uri())`. A foreign view with
an equal image/configuration is not a substitute. Existing duplicate-owner
refusal remains on legacy URI-only inventories; a new view inventory validates
unique issued view IDs and exact current analyses, rather than deleting that
guard indiscriminately. All physical edit/collision coverage remains visible;
conflicting views require atomic refusal, never a partial edit plan.

Retained handles must remain bound to the ISSUE-time view:

- Existing Native/TYPE wire guards and formats remain unchanged. A source-only
  view supplies no Native interpreter, execution, Loaded origin or Normal fact.
- Proposed registry storage adds an `OriginalInView { identity, publication }`
  variant under the existing readonly original wire schema. The publication is
  a typed host payload, analogous to the existing Logical host payload; it is
  never serialized and grants no semantic/source identity.
- Its equality includes the actual physical view lease/ID, so issuing B cannot
  reuse A's token merely because their OriginalDeclarationIdentity compares
  equal. Existing legacy `.resolve` refuses this distinct retained variant.
- View resolution first retrieves the saved typed host stamp, then looks up
  that EXACT view ID in the current published table. If it is absent/retired,
  resolution stops. It never searches for another view by URI/text/name/input.
- Under the final current-source transaction, validate the saved parent chain,
  original point against the retained parent AnalysisResult, physical admission,
  child image/full input/configuration and live view ID/revision. Only then use
  the existing original declaration currentness/owner checks on that analysis.
- Return a view-owned selection (identity + retained actual document/view), not
  a bare identity followed by `analysis_for(uri, ...)`. Incoming/outgoing calls,
  type hierarchy, lenses and workspace-symbol resolve consume that same owner.

This adds publication invalidation to a previously genuine readonly identity;
it does not mint a new Native semantic issuer. A final implementation may use a
narrow separately named typed host payload, reusing the existing private payload
machinery. It must not cast Logical CALL/TYPE/LENS handles into original handles.

### Exact existing host seams

- `FreshAnalysisDriver::analyse` currently chooses namespace-string or standalone
  ingress. Add an explicit inherited-entry route and retain its Option refusal;
  do not route a failed point back through the existing two branches.
- `WorkspaceIndex::index_sources` is the issuer join point while the exact parent
  analysis is still present. Its `WorkspaceSource` path/schema card must keep the
  authentic point and actual parent view owner, rather than only site_namespace.
- `publish_rehomed_if_current` currently adds each analysis and then drops the
  entire slice. Commit each full view record with its own index context in the
  same transaction, preserving the unchanged producer attachment on reuse.
- `original_declaration_documents`, `lift_original_call_item`, incoming/outgoing,
  type hierarchy, lens and symbol resolution currently recover analysis by URI.
  The retained view locator must be selected BEFORE `analysis_for`/standalone
  reanalysis. Return actual view-owned documents through target lifting. A
  stale source-view row is terminal unavailable; no fresh/legacy retry.
- `readonly_reference_reports::capture_readonly_reference_document` rebuilds a
  database analysis. Introduce view-owned capture borrowing the actual analysis,
  while reusing its existing physical `ReadonlySourceDocument` currency and
  `original_readonly_analysis_matches_input` checks. Do not relabel an inherited
  analysis as a standalone database producer.
- Existing `LibraryPublicationSource`, `class_type_source`,
  `readonly_saved_sources`, `with_current_declaration_sources` and
  `DocumentPathAdmission::with_current_saved_sources` already provide physical
  revision/path/saved-byte validation. Reuse those genuine host leases plus the
  exact per-view table membership and parent chain, with no URI-only recapture.
  Capture parents under their actual publisher transaction; collecting a later
  equal-text document is not the producer's publication.

The existing current-publication order is analyser-input reader, rehoming gate,
actual document guard, then nonblocking index read and synchronous saved-source
callback. Keep that order and release guards before async saved reads; after the
read, rejoin every saved owner before issuing results. Parent DAG validation
collects actual retained physical publications, not only child URI owners.
A new side table cannot add a reverse lock dependency; retaining view metadata
in the same rehoming transaction allows immutable snapshots without holding an
extra async lock during saved-source reads.

A view inventory capture must first join `rehomed_index_guard`, then clone the
actual current view table under the same transaction. Current Native original
hierarchy routes do not do this before `original_declaration_documents`; merely
switching its vector element type is insufficient. A parent replacement retires
its derived views/edges synchronously or makes their saved parent lease fail
before the new parent publication is announced complete. Background settlement
cannot leave an old descendant eligible between parent completion and a later
query. The first new query can perform one real full reconciliation; repeated
naming requests cannot be a readiness or repair mechanism.

Ordinary captured-request policy (existing #1854) remains separate from later
retained-handle resolution. The latter requires the saved view still be live;
it cannot use the captured-request allowance for an advanced editor version to
renew an already retired A handle. This does not change the existing general
workspace-symbol captured-answer contract.

## DB/CU transport

View providers must not call ordinary `analysis_for`/document-CU queries keyed
only by physical file+config to recover inherited analyses. Prefer the retained
view analysis. For a CU, first use its existing `original_optimiser_source_unit`
owner-checked rebuild, which preserves the exact private retained entry. Missing
owner/evidence is unavailable; do not retry using `for_supplied_source`,
`entry=None`, or namespace/profile-label reconstruction.

If a DB memo is subsequently required for performance, its query must carry the
authentic entry/full input alongside the actual child `SourceFile`; the host
physical/publication lease remains independently held. This needs narrow
DB/helper coordination with names/compiler and cannot be keyed solely by URI,
text or a rendered namespace. No new field on every general UnitBuildOptions
caller is required merely to transport one view.

## Source-origin purpose audit

The new source-only origin must not be treated as Authored or Native by default.
Each consumer needs the existing typed purpose join or an explicit unavailable
result. Static findings, not executed failure attribution:

| Consumer | Existing gate and required treatment |
| --- | --- |
| `references::invocation_calls_named` | After a failed local typed declaration join, non-Authored origins currently fall back to rendered allocation-command equality. InheritedPoint must not enter that fallback; use an actual view-owned target join or refuse. |
| `definition.rs` indirect-head routes around1800/1860 | Authored destructuring currently refuses other origins. Do not replace it with broad image equality; select through exact retained source-view ownership. |
| `receiver_identity.rs` / `original_oo.rs` | Several Authored checks feed Native receiver/member/frame selection. Keep Native purpose requirements independent; source-only inherited state grants no runtime receiver or Normal certificate. |
| `OriginalClassInventory` | Legacy duplicate-URI guard remains. A view-aware inventory validates unique issued view IDs and authentic per-view analyses before typed class/declaration selection. |
| `original_invocation::source_compilation_unit` | Reuse the actual private retained-entry producer via original_optimiser_source_unit; never manufacture a source-only or Native substitute. |

Tests must include an InheritedPoint lookup whose rendered qualified name matches
another view but whose typed owner differs. Positive helper navigation requires
the genuine selected view; equality of display names must not create references.

## Mandatory genuine negative and positive controls

Native protocol fixture:

- `A/root.tcl` initially contains `namespace eval lib {proc héλper ...}` and
  `namespace eval app {namespace path ::lib; source ../shared.tcl}` (the actual
  fixture uses a source expression relative to info-script's parent directory).
  `shared.tcl` is unchanged `proc run {} {héλper}`. All source is tcl8.6 and a
  matching pinned C run prints GOOD. B is initially absent.
- Await independent workspace scan + exact current completed A diagnostics.
  First child definition/hierarchy preparation uses the inherited A view.
  Save the actual returned child hierarchy handle H_A and its parent A lease.
- Change A at version2 to a source without the edge; create B/root.tcl with
  EXACTLY old A's bytes, same effective config/profile/grammar. Shared child's
  bytes, physical URI and input remain unchanged. Publish the real didChange
  and watched-create events, genuinely open B, and await exact completed
  A2/B1 diagnostics; watched Tcl-file changes do not themselves emit a
  workspace_folders_scan marker. Initial workspace readiness still waits scan;
  no naming-result polling or repeated query is allowed to repair state.
- The first incoming/outgoing/type/lens resolve using H_A must refuse/return
  unavailable through the existing protocol, even though B is current and its
  bytes equal old A. Assert current B view really exists and its independent
  first new handle/definition is positive, with matching C GOOD execution.
- The saved H_A cannot be renewed by copying B's display label/range or by
  retaining equal child source/input. Its saved parent view ID is retired.
  A controlled owner-level test additionally supplies the actual current B
  view to the old-A resolver and checks rejection at the physical/view lease
  boundary, rather than relying only on semantic origin inequality.
- Restore A's original bytes at version3 as an additional non-reuse control:
  the old view ID/handle remains retired. Any newly issued A view has a fresh
  publication ID, even when source/configuration return to earlier values.

Keep original source, parse/channel/current-binding guards and original timing
budgets. D2's existing two-namespace twelve-hop/retired-edge protocol regression
remains required alongside this equal-byte-parent control. These controls are
not implemented/run yet; no acceptance or stale-handle closure is claimed.

## Concrete off-tree consumer/test candidates

These are proposed artifacts, not applied Rust, compiler results or acceptance:

| Candidate | Existing boundary preserved | Limits |
| --- | --- | --- |
| `/tmp/lsp-n13-retained-cu/proposal.patch` | Core current-analysis/input/registry guards plus existing compiler private same-entry rebuild | Retained producer availability/ordinary assistance/external evidence need runtime tests. |
| `/tmp/lsp-n13-source-view-wire/proposal.patch` | Private distinct OriginalInSourceView row, unchanged schema1 wire, legacy resolver refuses, saved typed publication before exact current owner join | Storage-layer test uses typed test stamps; genuine physical-parent lease proof still required. |
| `/tmp/lsp-n13-parent-protocol/proposal.patch` | Existing native e2e module and30s readiness budgets; first old-handle resolve before new B naming | Authentic compiler issuer/host publication implementation absent; test uncompiled/unrun. |

Each directory includes exact canonical-before/proposed/patch SHA-256 receipt.
The retained-CU and source-view-wire candidates touch independent files and can
compose after current coordinated sources are rechecked; none is automatically
authorised for canonical application. The protocol draft's helper names and
ranges are checked statically against the manual-oracled fixture. Its first old
handle request has no arbitrary-error escape: only the existing empty/null
unavailable protocol result is accepted.

## Compiler stage2 review (off-tree only)

Compiler-names now derives the inherited source graph from the actual parent's
source-transition tape at the same selected post-argv/pre-handler point. Path,
imports and producer cells stay attached to that parent state. Its source-only
origin contains the immutable issued token; no interior graph memo participates
in map keys. The full entry remains in child options/analysis/CU. Key exhaustion
now returns unavailable, following review of the earlier panic branch. This is
agent-reported moving-shadow implementation, not a compiled or integrated API.

A second necessary consumer is proposed as
`OriginalSourceProcedureCall::declaration_in_parent(child_analysis, parent_analysis)`.
Existing `declaration(analysis)` rejoins the call owner and then searches that
analysis's headers; inherited parent helper headers are absent from child
metadata. The new getter must first validate this call's child attachment, child
image/full input, same retained full source entry/origin token and actual direct
or transitive ancestor analysis owner. Only then may the existing typed header
match return the parent's declaration. It cannot inspect host retirement; the
host first validates the entire saved physical/publication chain and supplies
that exact live parent analysis. A saved retired A Arc is not eligible merely
because the compiler getter still recognizes its original producer.

The corresponding host flow is concrete:

1. A child consuming call is selected from one actual retained child view.
2. The same frozen inventory supplies its recorded strong ancestor views and
   current physical leases, rather than searching by URI or reporting name.
3. Each eligible exact ancestor is passed through the compiler getter. Zero
   matches means unavailable; more than one distinct declaration remains
   ambiguous. Equal-byte independent parent B cannot substitute for retired A.
4. A returned header is paired with that same ancestor view coordinate and
   physical URI for lifting. The wire handle retains the saved view publication
   stamp and existing genuine declaration identity.
5. Resolution rejoins the saved current table entry and all ancestor leases
   before obtaining the original analysis/header; a stale saved stamp is a
   terminal refusal, including after equal-byte restoration under A3.

This supplements the publication/view table design above. It does not make
`OriginalClassInventory` URI deduplication, first-URI call-item lifting or
`references::invocation_calls_named` reporting-name fallback valid for inherited
views. Their view-aware routes remain required host/Core implementation work.
