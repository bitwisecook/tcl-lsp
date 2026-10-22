# N13 finite conditional source regions: coupled kernel/host interface

Status: off-tree implementation interface, not product code or executed evidence.
It extends conditional continuation contract v2 without new execution authority.

## Actual producers and stored plan

`SourceCommandBindings::walk_source_segments` is the issuer of an original
remaining command plan: it owns the exact full SourceImage, original base,
SegmentedCommand inventory, current index and source context. A private
`SourceConditionalScriptFrame` retains that immutable inventory, its original
source instance, actual typed namespace/frame, source parsing purpose, full
input and original parent analysis attachment. It never stores a consumer
substring or reparses an enclosing body inferred from a span.

A stack in the actual source kernel records these frames while descending.
`walk_source_image` records the same original frame restoration that it applies
to all outcome branches. `walk_selected_native_body` supplies a separately
positive conditional NamespaceEval delegation/exit plan at the existing
OriginalNamespaceBody capture boundary. Its source geometry, selected handler,
exact one-body operand, namespace creation/selection and parent variables/frame
are retained. The conditional exit must not call
retain_closed_native_handler_completion or borrow an entered Native frame.
Other body exits, loops/backedges, callbacks, alternate frame selection, active
expression/substitution arguments and unsupported stacks refuse plan issuance.

At the exact selected SourceLoad post-argv/pre-handler point, the existing
source-point issuer records the SAME full point receipt plus this actual
kernel stack in `OriginalSourceConditionalResumePlan`. Repeated or conflicting
plans at one point withdraw it. The selected source completion grammar must be
positively applicable before source_file::completion_route is called; that
function is only a route mapper. The path-only navigation card cannot supply
completion/decode/handler authority.

## Child outcome and sequential dependency

The genuine child source kernel retains complete SourceOutcomes before its
publication/fallback into final_state. A source-only outcome receipt keeps the
same inherited point, actual child analysis attachment, source/image/config/
full input, real normal/abrupt inventory and every can-normal alternative. The
source transition advice graph must be retained at that SAME selected normal
outcome; a separate whole-source end graph cannot be assumed to correspond to
it. Unsupported or unknown can-normal effects join conservatively or refuse.
An abrupt-only child cannot issue a later suffix. The child root fallback
final_state is never a success receipt.

The first region consumes this child outcome through the selected source
completion projection. Its kernel walks only the already retained original
inner script suffix. Its genuine normal outcome then undergoes the retained
conditional namespace/frame restoration and becomes the ONLY entry for the
next outer script suffix. Every later region retains that immediate predecessor
outcome identity. The host receives a region set for transport, but cannot
reorder regions or independently seed them all from the original point/child.
No intermediate result is actual Loaded/Normal completion or a compiler visit.

## Public opaque carriers

```rust
impl OriginalProjectSourceLoad {
    pub fn conditional_continuation(
        &self,
        parent: &AnalysisResult,
        child: &AnalysisResult,
    ) -> Option<OriginalSourceConditionalContinuation>;
}
impl OriginalSourceConditionalContinuation {
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool;
    pub fn matches_child_analysis(&self, child: &AnalysisResult) -> bool;
    // Performs the existing kernel's finite ordered resumption once. Returns
    // None on unsupported/ambiguous/opaque coverage, never a fresh parent view.
    pub fn regions(&self) -> Option<Arc<OriginalSourceConditionalRegionSet>>;
}
impl OriginalSourceConditionalRegionSet {
    pub fn regions(&self) -> &[OriginalSourceConditionalRegionView];
}
impl OriginalSourceConditionalRegionView {
    pub fn analysis(&self) -> Arc<AnalysisResult>;
    pub fn matches_region_analysis(&self, analysis: &AnalysisResult) -> bool;
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool;
    pub fn matches_child_analysis(&self, child: &AnalysisResult) -> bool;
    pub fn owns_original_command(&self, command: &CommandTokens) -> bool;
}
```

All constructors remain private to the existing source kernel. A region view
holds its actual AnalysisResult Arc plus the complete sealed region receipt.
The analysis retains that same receipt; receipt payloads do not point back to
the analysis, avoiding an ownership cycle. Private immutable tokens are only
indexes after complete payload equality, never source/currentness authority.

One region view covers one original script-frame suffix, with its actual
namespace/frame, full parent image and exact original command inventory.
It does not represent a flattened whole-parent analysis. A set can therefore
contain an inner namespace suffix followed by the outer/root suffix. Their
original parent/child/predecessor owners remain distinct and strongly held.
Source declarations retain their real region or ancestor origin; prefix A
records do not acquire a new tail identity.

## Analysis and query admission

The existing analyser command walker can consume the kernel-owned original
commands under that exact conditional frame/namespace and retained realm.
Its region entry never runs a new whole-parent source interpretation, and no
consumer can call the ordinary analyse_commands API with a span to issue it.
The kernel retains full image/config/input but issues no whole-image coverage.
Outside-region command/point lookup is terminally unavailable, including
projection_at_source/unpositioned fallbacks that would otherwise borrow the
region's final state. Overlapping prefix/region selection must use the sealed
command-owner inventory. Smallest span, first URI, reporting names and current
namespace strings are not selection rules.

Ordinary whole-CU/optimiser rebuilds, native/body compiler providers, completed
worlds and generic physical/source-purpose grants refuse region analyses.
The whole-input original_source_analysis_entry adapter is not broadened to
regions. Any later region entry reborrow must preserve the exact region stamp,
original plan, actual frame and predecessor chain. Independently recorded
compiler observations remain at their original source/world/table owner and
cannot be reattached by equal source offsets. Region source lookup alone does
not create Reached or Inline admission under the 13:17 decision.

## Host table and reuse

The host retains a current physical parent URI/path/publication and prefix A,
the selected child view, ordered region set and each exact region analysis.
It indexes prefix plus regions as separate per-view memberships. Appending a
tail cannot retire the still-live prefix on which the child depends. A single
URI-wide index revision or wholesale URI replacement cannot provide this join.
Strong compiler ownership does not prove that a host publication is live.

Unchanged actual owners reuse the SAME region set and analysis attachments;
fresh reanalysis on every reconciliation round would invalidate receipts and
prevent convergence. Retiring A removes eligibility. Equal-byte B or restored
A3 cannot renew the old child, tail, declaration or provider handles. Required
source points whose issue/continuation remains None stay unavailable queue
rows; the queue cannot skip them or declare settled by a fixed round count.

## Required controls before this interface is implemented/admitted

- Genuine root suffix and namespace-inner then outer suffix chain; outer state
  must contain the inner region's actual writes/path/import/declarations.
- A prior inner normal-state change cannot be omitted by reusing the child
  baseline. Reordered/foreign predecessor and same-byte foreign parent refuse.
- Error/return/levels/break/continue and unknown can-normal alternatives use
  exact positive selected completion grammar, join/refuse appropriately.
- Parent prefix A remains selectable after adding a tail; overlapping source
  geometry chooses the actual issued owner rather than URI/span precedence.
- Outside-extent lookup/projection and full-CU/compiler requests refuse.
- Same-live region reuse converges; retired A/equal-byte B/restored A3 cannot
  renew old handles. Physical publication and compiler predicates are separate.
- Existing two-parent twelve-hop/Unicode/inherited-directory D2 assertions stay
  unchanged; unsupported intermediate region/decode remains explicit failure.
