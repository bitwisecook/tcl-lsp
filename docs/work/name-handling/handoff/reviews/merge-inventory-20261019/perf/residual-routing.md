# Residual Analyser candidates — compiler-names

Status: **Blocked/routed, unproven drafts**. Neither patch is applied to the
full-batch proof tree and neither production change is committed by perf.
Apply `perf-analyser-images.patch`, then `perf-analyser-segments.patch`.
Both touch compiler-names' analyser/commands.rs; images also touches validity.rs.

The image patch reuses the actual retained root image only when its complete
bytes and Document channel still equal the public source buffer. A stale or
foreign source buffer constructs a fresh document image exactly as before.
The complete-buffer equality guard still scans O(source bytes) per reuse.
This draft reduces allocation, hashing and index construction constants; it is
not a proof of linear total analysis. Eliminating that comparison requires the
owner to validate once at an immutable analysis-operation admission and pass
the resulting context through the operation. Reusing a mutable public buffer's
pointer/length or a cached digest without mutation invalidation is insufficient.
Every existing original input/configuration/realm guard remains. SourceMap
construction reuses the existing native LF line index. It does not replace
that index with the LSP CR-aware variant.

The segment patch keeps retained_invocation_tokens' whole-source/config/full
argument-vector and lookup receipt checks. It rejoins the actual root origin,
then borrows its existing immutable complete-vector lexical memo. That memo
already checks the same offsets, IR words, native word geometry and full
configuration and constructs the same nested-config SegmentedCommand. The
current method instead freshly captures that same projection, discards its
segment, and segments the same first-to-last whole-word region again.
No handler, namespace, role, lookup or execution answer is memoised here.

Before landing, compiler-names must run old/new/old AnalysisResult equality on
the 102 Tcl + 15 iRules inputs and synthetics (include mutation of the public
source buffer, channel/config changes, malformed/partial words, expansions,
CRLF/bare CR and nested F5 continuation/lookahead). Keep a control for the
presubstituted-argument refusal. Then repeat the same crate filters and both
named server tests. Measure all 10/40/100/253 tclIndex prefixes. Existing
projection and owner tests stay unchanged. These source arguments are not an
executed equality result; perf makes no before/after claim for the drafts.

Review 4.1/4.2 measurements use tools/lspprobe_example.rs.txt, built as a
zz_lspprobe example in tcl-lsp-core, on the actual analysed flat document path.
The probe asserts all procedures survive index intake and lens collection.
The public RunOrder::build source-chain fixture is measured separately; its
compatibility graph is not a Native/provider admission proof. Origin and latest-work positive-control timings are saved in
evidence/latest-e4a01bbe/lsp-positive-medians-ms.json and evidence/timings.md.
All index/procedure/lens counts and one actual reference per lens passed at
10/20/40/80 procedures; public source-chain sizes 250/500/1000/2000 passed.
At 80 procedures, work lenses take 25,542.691 ms versus origin 106.465 ms
(contended opt-level-1 wall observations, not release latency). Other review costs (candidate lists,
key parsing, body/scope scans, reachable-offset rounds, qualified-reference
attachment, workspace existence, cross-file settlement, method lenses, rename
merging, alias chasing, wildcard rebuilding and retained memory) remain open
with their semantic owners. No claim that the memo batch fixes those shapes.

## Ordinary lens path: current Logical implementation

The review's historical line numbers are not a complete description of current
`work`. On the current proof base, code_lens.rs calls
`logical_lens_reference_advice` separately for every retained lens. For a
callable, code_lens/logical_source.rs calls
`call_hierarchy::logical_incoming_from_inventory`; its `edges` function calls
`Inventory::capture(documents)` and `inventory.occurrences()` afresh per lens.
Capture rebuilds a WorkspaceIndex, all retained procedure headers and whole
source snapshots. LogicalCallIdentity::is_current_in also captures the inventory
afresh; both the outer lens guard and the edges guard invoke this, multiplying
the rebuilds before occurrence selection. A shared inventory must cover these
nested validation entry points while retaining every validation condition. Occurrence selection then repeats the same document walk.
This is avoidable multiplicative work on the ordinary retained Logical path,
even if the older lexical namespace-scope scan is bypassed. Each returned
location also constructs a fresh LSP LineIndex in the lens adapter.

Blocked/routed to the LSP owner: introduce an immutable, request-scoped retained
inventory for the exact independently supplied document/analysis owners; capture
and enumerate once, then perform the unchanged declaration-specific selection.
Do not memoise by display name, reporting key, source bytes alone, or a client
handle. Keep each identity's whole-source/input and unique-URI checks, source
occurrence reissue checks, partial coverage, applicability debt, duplicate
handling and per-owner range projection. Reusing a report across documents or
host publication leases is outside this proposal. The memo batch does not
implement this change. The completed ordinary-path probe measures actual analysed
source and checks procedure/index/lens counts plus actual reference titles,
so withdrawal cannot masquerade as a speedup. A final implementation needs exact complete lens/reference report
equality, including retained identities and coverage, not just count equality.

## Selected compiler tests: source-transition body walk

A single read-only stack sample at 18:50 UTC of the active new-mode selected
compiler test process (base e4a01bbe plus full batch) shows two active worker
paths outside this batch. One repeatedly nests BodyWalk::regions -> command_inner
-> command -> script -> regions and retains possible/prepared region effects.
The other clones AdviceGraph from BodyWalk::regions below
retain_original_body_command_catalogue / retain_original_body_catalogue /
retain_source_inventory / retain_original_invocation /
OriginalSourceTransitionSeed::prepare_for_query, reached through
original_logical_procedure_body_cards ->
original_logical_procedure_may_dependencies_for_module ->
OriginalProcedureSummaryOwners::may_calls -> CfgBuilder::source_call_may_writes.

Evidence: evidence/latest-e4a01bbe/compiler-long-case-stack.out. Symbols from
the exact active executable resolve the Rust frames; system shared libraries
were unavailable in the debugger's namespace, so those frames remain unknown.
This is one sample, not a frequency estimate or proof of a complexity bound.
The debugger paused/detached without changing inputs, assertions or registers;
the compiler run's total wall time includes that pause and is not a clean
performance comparison. Semantic test outcomes remain authoritative.

Blocked/routed to compiler and compiler-names: investigate repeated immutable
seed/body-catalogue preparation and recursive region traversal before selecting
a memo boundary. Preserve actual body/registry/input ownership, branch joins,
unknown calls, traversal-depth outcomes and complete source-transition effects.
Do not replace graph semantics with name-only summaries or bypass recursion
limits. No production edit or proven fix is claimed for this finding.

## Named server cases on frozen e4a01bbe full batch

Both requested 180-second cases completed their bounded comparison, with zero
pass-to-fail but neither gate green. Source tree and full Rust diff were checked
against the build-input receipt before copying outputs to latest-e4a01bbe.
Startup metadata fails its first retained-read-count assertion, 0 versus 1 at
original_document_metadata_tests.rs:618, in BOTH modes (24.451 / 23.893 s).
The later factory reindex assertions are unreached. Route this positive-role
withdrawal to compiler/LSP; perf changed no expectation.

workspace_symbol_caps_its_answer times out at 180 s in BOTH modes
(180.085 / 180.224 s). Its unchanged fixture analyses 41 real procedures once
and reuses that analysis across enough documents to exceed the 1,000-symbol
cap. The timeout does not identify which phase dominates; do not claim an
index/query attribution without a separate profile. The completed positive
ordinary-path measurements above establish substantial remaining LSP cost.
Route the server case and inventory/index reuse to the LSP owner. Retain
all cap, role, source-owner and reference assertions. Raw summaries distinguish
timeouts from assertion failures; no timeout is a fabricated Rust test result.

## Current-source refresh and bounded LSP draft

The image/segment patches are refreshed against f149597f and ordered clean
application reconstructs the formatted snapshots exactly. They remain
uncompiled, unproven and unapplied to the production/proof trees. Main split
batch patches are independently refreshed; neither residual is in that batch.

`perf-lsp-current-inventory.patch` is a separate **Blocked/routed to LSP** draft
against 0322d9cd. It extracts the existing LogicalCallIdentity currentness
checks into a private helper over the actual captured immutable Inventory.
The public method still captures the independently supplied documents. The
edges and advice validation paths already own that same complete capture and
reuse it for their nested currentness checks. Every original source/input,
unique-URI, owner, consumer and selecting-occurrence reissue guard remains in
the helper; no receipt or reporting-key cache is introduced. The helper uses
the captured document references directly, preventing a second mismatched
slice from being supplied. No public API or production code is changed here.

This removes duplicate captures within one query but retains per-lens outer
capture and occurrence enumeration. It does not establish linear scaling or
resolve all review 4.1 shapes. The owner must type it, prove complete advice
and final lens/reference equality (including currentness/coverage/identities),
retain foreign/changed/missing/duplicate URI and occurrence negatives, and
measure the unchanged positive ordinary-path probe before accepting it.
`lsp-inventory-draft-inputs.json` pins exact before/after bytes. Current source
sites for the broader request-scoped reuse proposal are retained in
`evidence/current-f149597f/ordinary-lsp-source-audit.json`; that is a source
audit, not a fresh latency or output-equivalence result.

## Canonical refresh a3a4b181

Both analyser drafts and the nested LSP currentness draft are refreshed against
a3a4b181. Ordered analyser patch application reproduces the scoped formatted
snapshot exactly. Production sources remain untouched. These are still
Blocked/routed, uncompiled and unproven. Full semantic proofs and unchanged
positive ordinary-path measurements remain with compiler-names and LSP.

## Canonical refresh b6ea6dc5

All three drafts refresh tob6ea6dc5. Ordered analyser application reproduces
the scoped formatted snapshot. No production source is changed; all remain
Blocked/routed, uncompiled and semantically unproven. The line-index and
immutable-vector admission arguments retain the earlier documented limits.

## Empty checked body coverage draft

`perf-body-empty-regions.patch` is Blocked/routed to compiler/compiler-names.
The existing regions helper clones the whole AdviceGraph even after checked
coverage returns an empty vector, then executes zero iterations. This draft
returns the same Some(()) after those same checks without that unused clone.
No graph mutation, region guard, counter or nonempty traversal is changed.
Only source/parser/fmt review exists; no compiled/equality proof or latency
claim. It does not repair the1000-level nested-source resource failure. Owners
must retain all guards and prove full AnalysisResult/PackSet equality before
accepting it. The earlier one-stack clone observation is not a frequency or
complexity estimate.

## Canonical refresh83774d43

Image/segment drafts and the nested Logical inventory draft refresh to83774d43.
Ordered analyser application reproduces scoped formatted snapshots. Relevant
source files match the exact canonical HEAD despite the separate full9 proof
batch. No production source is edited. All remain Blocked/routed, uncompiled
and semantically unproven, with the same full-buffer comparison and per-lens
outer reconstruction limits. The1000-depth OOM and broader review4.1 shapes
are not fixed by these drafts. Owners must retain all currentness/coverage/
registry/identity guards and prove complete reports, PackSets and positive
ordinary-path results before accepting changes.
