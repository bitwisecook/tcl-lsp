# Residual Analyser candidates — compiler-names

Status: **Blocked/routed, unproven drafts**. Neither patch is applied to the
full-batch proof tree and neither production change is committed by perf.
Apply `perf-analyser-images.patch`, then `perf-analyser-segments.patch`.
Both touch compiler-names' analyser/commands.rs; images also touches validity.rs.

The image patch reuses the actual retained root image only when its complete
bytes and Document channel still equal the public source buffer. A stale or
foreign source buffer constructs a fresh document image exactly as before.
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
