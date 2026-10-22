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
compatibility graph is not a Native/provider admission proof. Origin timings
are saved; work timings remain pending. Other review costs (candidate lists,
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
implement this change. The queued ordinary-path probe measures actual analysed
source and checks procedure/index/lens counts, so withdrawal cannot masquerade
as a speedup. A final implementation needs exact complete lens/reference report
equality, including retained identities and coverage, not just count equality.
