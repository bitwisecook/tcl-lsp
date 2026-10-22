# N13 conditional cross-file source continuation (off-tree contract)

Status: proposed implementation contract only. No canonical source, typecheck,
Cargo or executed control. It addresses the existing source-view decision;
it does not extend Native loading, execution or compiler authority.

## Existing producer boundary

`TrustedSourceModuleLoader` (`command_binding.rs:1128`) retains a driver-attested
physical file/decoding implementation and `Loaded` identity.
`source_file::walk_selected_source_file` consumes it only at a reached native
handler and publishes actual source completion routes. Workspace bytes and an
`OriginalProjectSourceLoad` cannot manufacture this carrier.

The generic `PackageTransition::SourceLoad` currently marks the command world
opaque and the loader unknown (`command_binding.rs:19462`). The source advice
tape has no cross-file conditional-state producer. The Registry's existing
`authored_source_operation_continuation` is an operation-descriptor continuation,
not a source-file producer: it requires explicit result-transition coverage,
rejects widened transitions and command-resolution impact, and retains no
child file/entry/normal-path owner. Neither existing API closes this gap.

A second sequential source point therefore remains unavailable after the first
opaque load. The first-edge carrier and host retirement work can proceed, but
cannot be reported as complete two-parent/twelve-hop convergence.

## Compiler-owned carriers

Names are provisional; all constructors and state access stay private to the
existing source kernel. Public handles expose Clone/Eq and checked predicates,
not mutable snapshots or scalar constructors.

- `OriginalSourceConditionalResumePlan`: issued at the actual selected
  post-argv/pre-handler SourceLoad point while the existing walker owns the
  original script plans and its enclosing normal continuation. Retains the
  same OriginalSourcePointEntry, exact original command/source extent, and
  the actual original enclosing script/body continuations with their source
  instance, remaining command plan, namespace/frame and interpreter purpose.
  No consumer reparses a suffix to invent its enclosing scope. Unsupported
  or conflicting continuation stacks refuse. Initial support is finite normal
  suffixes/namespace-body returns; loops/backedges, callbacks and ambiguous
  frames refuse until their original kernel owner supplies a closed plan.
- `OriginalConditionalSourceOutcome`: issued only by the child's genuine
  source walk, retaining the SAME full parent-point entry, child actual
  analysis attachment, image/config/full input and original outcome inventory.
  A retained normal branch is separate from abrupt/unknown branches. The
  existing final_state is insufficient because it can remain the incoming
  state when no normal path exists. Missing child coverage or opaque effects
  cannot become an empty or unchanged successful state.
- `OriginalSourceConditionalContinuation`: joins that exact resume plan and
  child outcome, retaining both strong owners. The child entry must match
  the plan's parent point; arbitrary byte-equal child or parent analyses do
  not satisfy this join. It carries the selected source handler's original
  completion-route grammar and explicit unresolved source applicability debts.
  An issuer-owned immutable equality token may index it only after complete
  payload comparison; hash, URI, display name and source text never issue it.

The full receipt stays in the source entry/realm/CU and host view. Origin or
view tokens alone provide no state/currentness permission.

## Checked API shape

```rust
impl OriginalProjectSourceLoad {
    // The parent kernel must already retain the actual continuation plan.
    pub fn conditional_continuation(
        &self,
        parent: &AnalysisResult,
        child: &AnalysisResult,
    ) -> Option<OriginalSourceConditionalContinuation>;
}

impl OriginalSourceConditionalContinuation {
    pub fn matches_parent_analysis(&self, parent: &AnalysisResult) -> bool;
    pub fn matches_child_analysis(&self, child: &AnalysisResult) -> bool;
}
```

Execution is an internal operation of the existing source kernel over the
retained continuation plan. It produces a distinct conditional tail/view
owner; it must not rerun the entire parent through a fresh authoring analysis
and call that new attachment the old parent. The host retains the original
parent analysis/publication plus this tail owner, and indexes the matching
retained region/view analyses. Prefix declarations keep their actual original
owner; tail declarations and further source points keep the sealed conditional
continuation owner. Recombination must use genuine source regions/owners, never
first-URI or name matching. The precise region-result representation is still
implementation work and must be reviewed before source changes.

## Normal-route meaning

The child walk retains possible outcomes under source-only applicability.
The selected source handler's existing audited completion projection maps those
outcomes into *conditional* parent continuation routes. This is not merely
`child root had Ok`: Tcl source's return/level/error handling must use its exact
selected dialect/grammar, including explicit non-normal routes. Unknown route
selection refuses continuation; no-normal-route children issue no later tail.
`source_file::completion_route` is only a mapper: without a supported source-file
grammar it can return non-Return Ok unchanged. The issuer must positively retain
the selected applicable audited completion grammar/contract first. Navigation-only
path_candidate and `completion_route(Ok) == Ok` cannot issue that grammar.

The normal-route tail is a conditional branch: its existence assumes that this
selected source handler read/decoded the associated file and took the retained
normal completion path. Other abrupt/unknown alternatives remain recorded as
obligations. They are not erased because the host wants to analyse the later
source site. Joined normal branches retain the existing kernel joins; conflicts,
unknown cells/path/observers and opaque effects withdraw facts normally. Any
opaque/unknown child branch that can also reach normal must join conservatively
or refuse; it cannot be parked only as debt while a known normal branch becomes
a unique whole-tail binding. If branch-specific views are ever supported, each
must retain its exact branch identity and alternatives at every consumer.

Required unresolved obligations on every tail, nested point and provider:

1. Actual parent source-point and current selected handler applicability.
2. Host-selected file association, actual file read and decoding applicability.
3. Exact child analysis/view applicability and normal-source completion route.
4. Original containing body/frame applicability and prior observer debts.
5. Current host physical parent/child publication chain, independently checked.

The continuation is not `Loaded`, actual SourceOutcomes Normal, a completed
world, physical NativeCompilationEntry, Native frame, live receiver or stored
variable/handle value. Command-source/namespace/path/import/variable facts are
conditional and keep their existing source provenance. Unknown effects remain
unknown rather than inferred absent from a positive inventory.

## Compiler observations and user 13:17 decision

Actual compiler visits already recorded by a genuine compiler owner retain
that independent fact under their original source/world/table owner. A tail
must not reattach those visits to changed dispatch state by equal bytes/offsets. The continuation never manufactures such a visit.
Host reading a child, surveying its syntax, selecting a conditional normal
branch or continuing a parent source view does not issue Reached, opcode
admission or ProcedureCode. Conditional tail/new-child interpretation disables
implicit Native compiler/body providers and fresh-table/trace-absence grants.
The 13:17 after-load correction cannot promote these host previews.

## Host reuse and retirement

The host keeps a strong current physical parent publication/path/URI owner,
original parent analysis A, exact point, exact child view and continuation.
A cache hit is eligible only for those same still-current owners and full
input/config. It reuses the SAME child/tail analysis attachments; rebuilding
on every reconciliation round would create new keys and prevent convergence.
Retiring A removes its eligibility. Equal-byte B or restored A3 creates a new
publication/analysis attachment and cannot renew old point/tail/provider handles.
Compiler matches against saved A alone cannot prove host A is still current.

Unavailable later points and child outcomes remain explicit dependencies. The
work queue cannot omit them or declare settlement because no view was issued.
No fixed number of rounds or namespace-string fallback supplies closure.

## Controls before integration/acceptance

- Two sequential namespace eval blocks source the same child; both conditional
  views retain distinct point/parent state and preserve path/import competitors.
- Twelve literal/composed hops, inherited global sourceDir read and Unicode
  helper; both app/other views, unopened files and original D2 assertions.
- Exact-source return, return levels, error/break/continue and unknown completion;
  no-normal child cannot issue a tail, normal alternative keeps all debts.
- Changed source handler/observer/namespace path/variable state, ambiguous frame,
  loop/backedge and unsupported continuation plan refuse without fresh fallback.
- Parent/child image/channel/full input/Registry changes; child entry from a
  foreign point; byte-equal independently analysed parent/child all refuse.
- Same current owners reuse the exact continuation/tail; retirement, equal-byte
  B and restored A cannot renew old handles; host's FIRST stale query refuses.
- Direct/retained-CU/DB entry equivalence and source-only purpose/Native frame
  negatives; genuine independent compiler visits retain original expectations.
