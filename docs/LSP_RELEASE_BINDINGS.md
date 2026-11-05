# LSP binding and frame issue verification

The audit starts from shared work commit `961f17a32`. The original checkout's
older binary is not evidence for the current work branch. Verification uses an
isolated Cargo target and fixed-input regressions, with the pinned C Tcl
8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 interpreters and JimTcl `82fb4d0`.

| Issues | Current evidence | Status |
| --- | --- | --- |
| #2501, #2502 | Exact current report inputs have no W210 under supported profiles; removing their binders in local controls still misses genuine reads. See the external validation receipt. | Scoped non-reproduction; control limits |
| #2246 | C Tcl and Jim confirm the missing-variable read; fresh compiler emits no W210. | Reproduced residual |
| #2371 | Native controls confirm the error. The narrow attempted-read-version candidate compiled but failed its first original-case Tcl 8.4 assertion; no W210 was emitted. | Reproduced residual; candidate parked |
| #2374 | The exact guarded read of `a(j)` emits W210 for Tcl 8.4–9.1; Jim-profile analysis misses it. A same-element read after setting `a(k)` separately warns on base `a`. | Partial across profiles; adjacent false positive |
| #2441 | Current `ad4d43160` plus the IR-stage repair passes Clippy, both integration targets, all 100 Tcl 8.4–9.1 functional rows, the distinct-sentinel CFG control and existing 185 source/admission rows. Jim's combined procedure loop retains no target; its standalone controls warn. The current Jim draft is UNBUILT (`cf77ac5566`). | [C5 proof / open Jim work](https://github.com/bitwisecook/tcl-lsp/issues/2441#issuecomment-6643275900) |
| #2376 | Read-local fix `07a1bd0ac` passes 185 integration rows and 14 guard cases. Joint verification passes 9 query-owner and 7 guard tests plus production Clippy; baseline failures were wrongly scoped after-load fixtures and are corrected without changing assertions. | Closed after coordinator proof review |
| #2379 | Exact size/names/get and may-set/exists-then-get inputs are quiet for Tcl 8.4–9.1; Jim substitutions can still warn. Tcl `statistics`/`startsearch` error controls currently receive no W210. | Partial; Tcl report rows clear, Jim/adjacent controls remain |
| #2294 | Work already carries/rebases `BodyFragment::proven_sites`; whole-file W230 is absent in the parity witness, before testing per-item parity. | Residual under investigation |
| #2268 | Existing witness fails its dead-store control: an unused literal element has no W211. The computed-element read control passes. | Reproduced residual |
| #2289 | Complete current linked file and called creator have no W140, with positive W140 controls across Tcl 8.4–9.1. | Current input verified |
| #2296 | The exact computed-key input retains the original `$k` SSA read at version 1 in all six profiles and has no W220. The literal-key control also has no W220. | Source read verified; diagnostic control limited |
| #2243 | Memoised/uncached seeded-global parity still requires a fresh database regression. | Pending |
| #2247 | Qualified/public dictionary forms emit neither W002 nor W211 in Tcl 8.6–9.1; W143 and unknown-head W123 controls pass. The independent unused-local W211 control is missed. | Availability verified; W211 control limited |
| #2286 | Exact quoted/current linked inputs have no W210, but the original Tcltest control still emits 0 instead of 2 warnings in all five profiles; direct/alias controls also miss their warnings. | Reported false positives absent; admission gap remains |
| #2290 | Latest public `tclmeasure.tcl` emits no W210; upstream source differs from the historical line references. | Current source verified |
| #2297 | Caller-variable read through upvar retains a procedure and original read, with no W220. An independent unused local reports W211 for Jim but stays quiet under all five C profiles. | Exact report quiet; C diagnostic control limited |
| #2305, #2306 | Qualified-root writes and same-frame alias writes preserve branch output and produce no stale I230. | Verified baseline |
| #2323 | Catch followed by its result guard still reports W210 for the successfully assigned file handle. | Reproduced residual |
| #2324, #2325 | Computed typed receivers and expanded deferred callbacks still require focused verification. | Pending |
| #2367, #2370, #2410, #2412, #2414, #2415 | Reported fixed programs preserve original execution after optimisation; associated W211/W220 defects are absent. | Verified baseline |
| #2455 | The nested expression read produces no W220. | Verified baseline |
| #2407, #2408 | Reports describe conservative summary precision limits. | Pending |
| #1778 | The reported distinct colon-edge commands preserve their original distinct output under C Tcl 8.6–9.1 optimisation. Additional lifetime and index consumers require verification. | Partial verification |
| #1796 | ResolveContext selects exact frames, but UpvarInfo still collapses all beyond-caller distances. | Confirmed remaining foundation |
| #2332 | Qualified and variable-linked substitution reads are impure with global effects; explicit set reads remain conservatively impure with unknown effects. | Verified baseline |

At the 2026-11-04 19:22 checkpoint, the frozen #2441 diagnostic-only classifier
has passed its combined compiler unit/integration no-run build (384.17 s).
The two new integration tests both fail on their first Tcl 8.4 assertion,
still missing `x`; the prepared 75 rows are not a passing result. New owner
fixtures also stop before testing the predicate: their ordinary-statement
filter selects zero invocation tokens. The later bounded stage audit corrected
the initial duplicate-offset explanation: the original vector is marked
`EvaluatedArguments`, with a companion `IterationBindings` statement. The
three existing owner controls pass. Completed regressions are recorded below.
Production Clippy failed on two unchanged `command_binding.rs` functions,
`walk_logical_called_source` and `walk_logical_namespace_source`, for
`too_many_lines`; no lint suppression was added. The diagnostic classifier
leaves Native/body resolution unchanged and requires retained
AuthoredSimulation input, original source/vector, closed conditional lookup
and an identical Registry target. No declaration-graph edit or new
Native/Normal fact is part of the draft. Exact current failure and runtime
receipts remain in `tests/data/w210-empty-loop-exact-source` and `evidence/tcl`.

The 19:48 checkpoint retains the red first-attempt receipt in
`tests/data/w210-empty-loop-exact-source/attempt-1`. Existing 185 rows, 14
guard cases and nine query tests pass; this does not establish the new fix.
The next fixture/premise probe is unbuilt, keeps the production classifier
byte-identical and retains strict source/availability/admission checks. Its
planned public matrix adds MayBound conditional guard controls (90 rows),
without counting any unexecuted row as passing. Generated loop source vectors
must be distinguished from the genuine original invocation before interpreting
the new private owner assertions. No declaration graph or Native owner changes
have been made.

At the 20:02 checkpoint, the experimental source-callee classifier remains
stashed and unverified. The IR contract says `EvaluatedArguments` evaluates
words without invoking the labelled command; iteration-binding stages also
perform no new handler dispatch. The next narrow repair will make the opaque
callee-name consumer respect these existing stage semantics, preserving word
effects and all real opaque calls. No Native or source-purpose fallback is
being widened. Independent #940 validation is meanwhile linking the sealed
phase-eight production libraries and comparing warning collection, full
checks and admission/Normal/provider facts on each same held compilation unit;
that run subsequently passes 65 rows, with its separate permanent receipt in
`tests/data/shimmer-940-source-owner/phase8-same-cu-2026-11-04`.

Fresh [nested word-effect native evidence](../evidence/tcl/w210-empty-foreach-nested-word-write-2026-11-04.md)
records all six interpreters returning `7` from a substitution-initialised
variable after an empty foreach, following an independently caught load error.
It constrains the pending typed-stage repair to preserve argument evaluation
and real unknown callees. The new 100-row compiler regression matrix and two
actual-CFG controls are unbuilt; no #2441 fix is claimed yet.

Coordinator checkpoint, 20:54 UTC: the narrow stage repair is now frozen on
`f2b927c5c` plus reviewed patch
`e663f5806d5f0772448501c8c3ae5ad27dcabd5eda2a68de27c6dec44bd34440`.
Production compiler Clippy passes (62.01 s). The compiler unit and two
integration no-run builds are active; the 100-row matrix and actual-CFG
assertions are not yet runtime results. The first gate attempt stopped on two
missing tracked registry include fixtures; only those exact files were restored
before the new source seal. No source-callee authority fallback is included.

The 20:23 checkpoint retains [public stage baseline controls](../rust/tcl-compiler/tests/data/w210-empty-loop-exact-source/stage-baseline/README.md).
They confirm the loop-only opaque-name contribution and a genuine unseen-call
contrast, while both nested word-effect controls are already quiet. All 138
emitted baseline libraries stay unchanged. The narrow IR-stage patch is
reviewed and awaits its cached compiler allocation after the priority S102 and
database jobs. The evidence-format check passes 1,647 current records; that is
a metadata check, not a compiler test result. No caller resolver, source graph
or Native admission edit is part of the pending repair.

At the 20:35 checkpoint, sparse fixture cleanup preserves the approved product
patch and untracked regression byte-for-byte. Restoring only 29 exact compile
fixtures (104,050 bytes) returns the compiler preflight to 221 available
includes and zero unresolved paths. All Git history, stashes, targets and
libraries are preserved. The earlier priority job stopped at its resource
guard without ENOSPC; this lane still has no new compiler result and awaits
its allocated retry. Production Clippy is prepared to run before no-run
compilation so a baseline lint failure cannot consume a full test build.

The 21:16 checkpoint retains the successful C5 functional result and the
separate CFG-fixture correction. Production Clippy passes and combined no-run
compilation passes after restoring exact dependency fixture inputs; the missing
input failures remain in raw logs. All 100 functional verdicts, existing 185
rows, three callee-owner tests, fourteen guard cases and nine query tests pass
on unchanged emitted libraries and source. The first CFG assertion wrongly
required an iterator binding after an unknown call; the actual producer keeps
argument evaluation plus a conservative residual wrapper instead. A corrected
public harness passes all three tests. Its final strengthening gives the real
opaque call a distinct sentinel name, preserving both callback checks; that
test-only rerun is pending. Fresh Jim-profile analysis still emits no W210 on
the combined source, with working standalone-procedure and top-level controls.
No #2441 closure or broader Native/source-purpose change is claimed.

At the 21:28 checkpoint, the final sentinel fixture passes against the exact
snapshot library, with its source and all 138 libraries unchanged. Its real
opaque call uses `sentinel`, distinct from loop binder `x`; that name survives
both lookup callbacks. The always-resolved callback excludes the wrapper's
`x`, while the unresolved callback retains it. No iteration-binding proof is
invented after an unknown mutation. A verified compressed archive preserves
the old libraries and required link dependencies in 213,842,676 bytes. Current
shared source has additional compiler changes, so the next allocation performs
fresh current-source Clippy and integration-only compilation/runtime checks.
The prior results remain explicitly scoped to their frozen product snapshot.

Current-source verification on `ad4d43160` now passes production Clippy and
both integration-only targets. All 100 functional rows, the strengthened
sentinel control and the existing 185 rows pass; 15,227 source files and all
145 production dependency files remain unchanged. The permanent receipt is
[`reviewed-stage-snapshot/current-ad4`](../rust/tcl-compiler/tests/data/w210-empty-loop-exact-source/reviewed-stage-snapshot/current-ad4/README.md).
The minimal fix changes only the IR stage predicate and its opaque-callee
inventory consumer. Jim's combined procedure position has `unknown=true`,
zero targets and an ordinary call, unlike its closed-target standalone
control. That missing binding is not replaced by a source spelling or a new
execution claim; #2441 remains open.

## Fresh exact-input W210 diagnostic receipt (2026-11-04)

A separate [36-process binding control audit](../rust/tcl-compiler/tests/data/binding-diagnostic-controls-2026-11-04/README.md)
rechecks #2296, #2297 and #2247 against the preserved verified production
library. It distinguishes retained original reads and working availability
checks from the C profiles' missing unused-local control. All three issues
remain open; a quiet report alone is not treated as complete verification.

The production analyser was linked read-only from the frozen normal compiler
library at source `f5b2f7d05ea1a322ef8f3f018e1af04394b36477` (joint patch
SHA-256 `da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`;
`libtcl_compiler.rlib` SHA-256
`5d1be13ab0851b1f85a83172acaa6d1ac0465a1bd51d8db594d670546c32759c`). The
current work head at capture was `bf9ec63be936a3c4eb2eb990194d3937914c8553`.
The W210 dataflow owner, compiler test source and Tcl `array` registry source
remain byte-identical through the current work head; later unrelated index
diagnostic identity code is outside this probe's path. A standalone
`ResolvedAnalysisInput` probe ran 17 fixed
inputs under Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and Jim profiles (102 rows); every
row had a resolved profile and no analysis-context error. Probe source SHA-256
`7e2060f1c79addc7662ff1df4416297b7c2d8b176648a9434a20fa2820b2bbef`, binary
SHA-256 `165671b8fa32825071601cc5606273ffd2e4f2fd3a8529b8e5f5a9cff160fb8a`,
and raw diagnostic output SHA-256
`192be090b365cc1743f49728c2d99be161d56a87776df1efd11e5910ae1b2dd7` are
retained under `/tmp` with the frozen rlib manifest.

The Tcl-profile rows show no W210 for the exact #2379 standalone `array size`,
may-set `size`/`names`, or `array exists` followed by `array get` inputs. The
matching native script prints `0`, `1`/`k`, `0`/empty, and `0`/empty. However,
the `array statistics` and `array startsearch` error controls also receive no
W210, so the array-command result is partial. The exact #2374 input emits W210
for `a(j)` in all five Tcl profiles, and native interpreters report the missing
element. A same-element read after `set a(k) 1` also warns on `a`, an adjacent
false positive; the Jim profile misses the exact `a(j)` warning. The #2441
exact input emits W210 for top-level `y` but not procedure-local `x` after
`[info exists x]`; an isolated unguarded procedure-local empty-loop read does
emit W210. Native interpreters report missing-variable errors for both exact
reads. The exact #2441 report therefore remains a residual.

Jim's runtime agrees that the exact #2374 and #2441 reads fail, but its analyser
profile emits neither warning on those exact inputs. Jim-profile W210 also
differs for #2379: nested absent-array `size`, `names`, and `get` substitutions
can warn, while the corresponding `puts` report inputs are quiet; `array get`
after `array exists` also warns. The production matrix does not generalise
Tcl-profile results to Jim. Absent-array `array statistics` and
`array startsearch` raise under the C Tcl runtime and the generic Jim catalogue
does not implement those commands, while the analyser probe emits no W210 for
these command forms; they are adjacent controls, not claimed as part of the
three original repro results.

The byte-exact native script and per-interpreter results, including the
`statistics`/`startsearch` error controls, are in
[`w210-array-and-empty-foreach-runtime-controls-2026-11-04.md`](../evidence/tcl/w210-array-and-empty-foreach-runtime-controls-2026-11-04.md).
Its fixed source was run with Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim
0.84. The retained analyser driver, all 102 rows, source/build hashes and the
separate Jim #2376 guard controls are in
[`w210-arrays-foreach-current`](../rust/tcl-compiler/tests/data/lsp-external-bindings/validation-2026-11-04/w210-arrays-foreach-current/README.md).
`make check-tcl-evidence` passed on `587ba05cff2a3b166689b6f8d463357ec9379acc`
(1,324 files and 14 format tests; log SHA-256
`9667fa777796d1582e429785b478dc05169248213f44a15435ce53d88221a06c`). This is
a diagnostic revalidation receipt only; #2374, #2379 and #2441 remain open.

The same frozen rlib was also used for three focused Jim-profile #2376 checks:
safe `&&` produced no W210, unsafe `||` warned on its `$x` operand, and the
later genuine `$x` read warned at bytes 79–81. All three inputs resolved a Jim
analysis context. This confirms those controls only; it does not establish
full Jim parity for the broader #2376 matrix.

The observations below are chronological history. The table above and the final
read-local receipt describe the active #2376 snapshot; earlier uncompiled drafts
and generic failures are retained for provenance, not current status.

The first build required fresh dependency downloads (`sha2` 0.11.1 was absent
from the cache). Network-enabled execution resolved the dependency limitation;
no dependency versions were changed.

Fresh compiler build completed successfully. The focused release audit has five
passing cases and four reproduced failures; existing name-lookup witnesses have
seven passes and one failure. Draft fixes are not counted as verified.

The expanded baseline uses the already-built isolated `961f17a32` compiler
artifacts. Nine fixed programs have paired original/optimised oracle evidence
across C Tcl 8.4–9.1 and JimTcl. Original and rewritten outcomes agree in every
row. C Tcl 8.4/8.5 reject the colon-tail declaration; JimTcl has distinct
namespace-frame/colon-name behaviour that the rewrite also preserves. These are
not interchangeable oracle expectations.

The first W210 rebuild after integration stopped in an existing registry build
error: `security_floor.rs` used a non-exported `crate::CommandForm` type. The
registry owner supplied `60f93a19d`; the diagnostic fixes await a retry.
`rust-check` first stopped on formatting in an uncommitted audit fixture
(subsequently formatted). `prep-pr` stopped while npm fetched dependencies
through the sandbox-restricted proxy. These are failed gates, not green checks.

The restored-environment external validation uses product source
`b7b653dd2bf0ca65edfbc1a322bab4dcc768ba42`; the built `b71a4b312` differs
only in this documentation. Complete fetched files, licence notices, original
issue snippets, source/input/binary hashes, raw results and draft evidence
comments are retained in
[`lsp-external-bindings/validation-2026-11-04`](../rust/tcl-compiler/tests/data/lsp-external-bindings/validation-2026-11-04/README.md).
The five selected tests pass; all 60 original-input/profile measurements are
complete. This is scoped diagnostic evidence, with the Tcltest and adjacent
missing-read control failures recorded explicitly, not a full diagnostic or
editor-path pass. No external issue was closed by this lane.

Unverified implementation work and the earlier failing/debug audit fixture are
preserved in the local stash named
`bindings-unverified-w210-drafts-before-external-validation`, with independent
recovery copies under `/tmp`. They were excluded from every new validation.

The independent #940 review matrix is in
[`shimmer-940-source-owner`](../rust/tcl-compiler/tests/data/shimmer-940-source-owner/README.md).
It records exact source inputs, warning/IR expectations and source-purpose,
provider-obligation and codegen negative controls. Its observational baseline at
`182ea7784` completed 27 cases with unchanged source hashes; a separate
same-binary missing-read control emits W210. All six S102 controls remain quiet,
as do the adjacent removed-binder and Tcltest genuine-read controls. The
top-level dict/list S100 control works, while its procedure-loop counterpart
does not. The exact receipts distinguish existing compiler/Normal authority
and provider plans from absent child proofs. No production fix is included.

The #2371 fixed-input native matrix is in
[`w210-unset-guards.json`](../rust/tcl-compiler/tests/data/w210-unset-guards.json),
with all 54 direct C Tcl 8.4–9.1/Jim interpreter observations in
[`w210-unset-guards-oracles.json`](../rust/tcl-compiler/tests/data/w210-unset-guards-oracles.json).
Each row records its exact input digest, catch wrapper and interpreter command;
a caught status of one preserves the native missing-variable error. Safe
controls preserve a guarded value, restore a destroyed value or destroy an
independent scalar/array cell. A standalone missing-read control checks W210
execution separately. These are native semantics, not a passing product fix.

The pre-change production baseline misses all four guarded deletion reports.
It also warns on a distinct safe conditional-producer/existence-guard example;
that existing false positive is recorded under `known_gaps`, outside this
direct-unset fix's regression acceptance. The narrow candidate used the shared
attempted-read cell/version to recognise an existing SSA destruction, without
inventing a readable value or completed Normal result. It compiled but failed
the first original-program Tcl 8.4 assertion: no W210 appeared, and the remaining
44 case/profile assertions were not reached. The bounded source trace records
`dynamic_writes=true` and `physical_missing=0`, before that candidate check can
help. Exact hashes, failed output, archived patch/test and nine refreshed
current-Jim runs are in
[`w210-unset-guards-attempt`](../rust/tcl-compiler/tests/data/w210-unset-guards-attempt/README.md).
The implementation and failing test are safely stashed, with no physical/source
gate change or failing CI test installed. #2371 remains unresolved.

The #2376 source matrix is in
[`w210-short-circuit.json`](../rust/tcl-compiler/tests/data/w210-short-circuit.json),
with 138 fixed native observations in
`w210-short-circuit-oracles.json`.
It includes `&&`/`||`, nested negation, the later genuine read's intended span,
and command results frozen before later stores in their argv. Unsupported
composite expression results cannot borrow a child's completed value. Native
NaN behaviour is recorded per engine: truthy on Tcl 8.4/Jim, an error on Tcl
8.5–9.1. The negative guards allow legitimate future precision instead of
asserting that an unavailable result must stay unavailable.

In the initial #2376 baseline, the binary walker computed short-circuit truth from its expression
reader, which deliberately declines command nodes, even when the original
left evaluation retained a completed command value. A possible repair would
consume that actual value only for a direct command or logical negations of
it, using the shared boolean-context and numeric-release owners. No such
production edit had been made at that checkpoint. The production baseline/regression driver is
prepared but remains uncompiled while #940/provider work has exclusive compiler
priority. This checkpoint publishes native evidence and planned acceptance;
it is not a product-fix or green-gate claim.

A concrete review-only proposal is archived as
`w210-short-circuit-proposal.patch`.
It accepts completed values only for a command AST or its logical negations,
uses the shared boolean-context acceptor with the actual invocation numeric
syntax (unanimous release agreement when absent), and leaves all existing
abrupt outcomes and fallback expression analysis intact. The production file
is unchanged; the proposal is neither compiled nor claimed to fix the current
report. Its iterative negation walk adds no parser or name interpretation.

The archived initial #2376 production observation completed all 115 case/profile rows;
see [`w210-short-circuit-baseline`](../rust/tcl-compiler/tests/data/w210-short-circuit-baseline/README.md).
Source hashes matched across compilation/runtime, and genuine unsafe controls
remain active. All retained final-Normal probes are unavailable; those Native
completion refusals must remain intact. The consumer proposal is still
unapplied at this baseline, and no product fix is claimed.

The then-unverified canonical draft narrowed the archived proposal to completed direct
command results whose text is exactly `0` or `1`, without a retained numeric
receipt, plus logical negations of that command. Other values retain the
existing expression-reader fallback. This avoids deriving cached numeric truth
from bytes. The draft, strict regression driver and revised completion-refusal
expectations are stashed during the benchmark reservation and this integration
checkpoint. No candidate compilation or runtime has run, and `rust-check` and
`prep-pr` remain pending for the eventual product change. The unresolved
unknown-condition branch and exact later-read span still require verification.

The baseline guard and later-read findings use physical W210 wording, rather
than the conditional declared-frame warning. Any read-local predicate filter
must precede both physical candidate collection and the def-use chain's
per-variable minimum. The shared condition-transfer/existence-query owners are
under review for that consumer; declaration/root-graph owners are unchanged.
The canonical draft and its model-dispatch checks remain uncompiled. Unsupported
completed-value cases must retain their existing result refusals.

Another 72 native observations, with full sources and interpreter binary hashes,
are in
[`w210-short-circuit-native-boundaries.json`](../rust/tcl-compiler/tests/data/w210-short-circuit-native-boundaries.json).
They include the verbatim multiline report and distinct scalar/array-element
controls: an existence query cannot guard a different cell. Tcl 8.4 accepts NaN
inside `expr {$x || 0}` but errors on the whole `if {$x}` condition; Jim accepts
both, while Tcl 8.5–9.1 reject NaN in the tested expressions and conditions.
Binary-scan cached NaN and literal NaN agree within each tested form. This is
bounded native evidence, not a universal getter rule or a passing compiler fix.
The prepared production matrix now has 60 diagnostic checks and 70 unchanged
published-completion refusals; candidate compilation remains queued by priority.

The canonical draft's fresh clean build emitted an integration executable. All
60 W210 checks pass, including the exact report, unsafe/distinct-cell controls
and the final genuine-read span. Its 70-result check is red: 30 final-store
values become available and match native outcomes. The existing
`SourceNormalResult` contract is conditional on Normal alternatives and grants
neither a physical object nor totality; separate admission/totality gates and
whole-expression values require review before revising that blanket invariant.
Exact frozen inputs, patch and failed/passing receipts are in
[`w210-short-circuit-canonical-attempt`](../rust/tcl-compiler/tests/data/w210-short-circuit-canonical-attempt/README.md).
The draft remains uncommitted. Five missing authentic sparse unit fixtures are
restored for the next private-model retry; full gates remain pending.

The corrected model build and all six focused tests now pass; see
[`w210-short-circuit-model-gates`](../rust/tcl-compiler/tests/data/w210-short-circuit-model-gates/README.md).
All 12 actual-input probes retain AuthoredSimulation/Direct purpose, supplied
metadata, no Native entry and no inline/named compiler admission. Final-store
source values are `0`, with normal completion for that invocation when reached;
whole expressions keep unavailable results and no complete-normal proof, so
raw child `9` is never a Boolean root result. The 30 prior source values match
the documented conditional value contract. The current coherent warning matrix
also passes 60/60 with unchanged source hashes. Stronger exact value/gate
assertions await the scheduled final combined run; no candidate code has been
published, and full gates remain pending.

Final canonical-consumer verification passes all six focused units and all 130
integration assertions with 41,082 unchanged file hashes. Exact receipts and
the additional 35 uncalled-procedure observations are in
[`w210-short-circuit-final`](../rust/tcl-compiler/tests/data/w210-short-circuit-final/README.md).
The uncalled bodies retain the safe-guard false positive and earlier deduped
span in all profiles, while unsafe and unguarded controls remain active. The
canonical known-result repair is therefore separately scoped; #2376 stays open
for proper read-local condition-edge facts with effect invalidation. No Native
or Normal authority is inferred to silence the remaining diagnostics. Full
release gates remain pending.

The 16:14 UTC checkpoint preserves a separate unverified read-local predicate
draft. It uses original variable-access spans and structured cells from the
existing existence-query and condition-edge owners; writes, aliases and unknown
commands expire inherited guards. The draft includes unknown-entry safe/unsafe
regressions and mutation-order owner checks, but has not been compiled or run.
It is stashed for this integration; no new passing result is claimed. The
canonical repair is published as `a6408fb3e`; #2376 remains open.

The 16:30 UTC checkpoint records the first read-local attempt in
[`w210-short-circuit-read-local-attempt`](../rust/tcl-compiler/tests/data/w210-short-circuit-read-local-attempt/README.md).
The cleanly built production matrix passes 175/175 assertions with 41,094
unchanged file hashes. The expanded owner suite is 6/7: a same-array-element
positive has no guard. Subsequent trace/other-element owner cases did not run.
The failing draft is stashed; no product fix is published or closure claimed.

The separate S100 foundation review passes thirteen independent same-held-CU
comparisons, with a genuine top-level S100 positive and seven negative controls.
IR, SSA, source result/completion gates, Native admission and provider obligations
remain unchanged; Bytecode/Traced snapshots agree. The named-procedure variant
remains a measured warning gap, and #940/S102 is not claimed fixed. Exact library
custody, source inputs and receipts are in
[`independent-s100-guards`](../rust/tcl-compiler/tests/data/shimmer-940-source-owner/independent-s100-guards/README.md).

The extra array owner's first failure was a fixture error: its query was
provably false and retained no original RHS read. A public probe of the exact
compiled candidate confirms that a conditional array-element write produces
the required original Element identity/read. Thirty native observations pass
on all six engines; see
[`w210-short-circuit-array-boundaries`](../rust/tcl-compiler/tests/data/w210-short-circuit-array-boundaries/README.md).
Only the positive harness changes; the production guard code is byte-identical
to the 175-assertion passing snapshot. The corrected 185-row/14-case run and
production Clippy are pending allocation.

Final read-local verification passes 7/7 focused units (fourteen guard cases)
and all 185 integration rows, with 41,817 unchanged file hashes. The permanent
receipt is
[`w210-short-circuit-read-local-final`](../rust/tcl-compiler/tests/data/w210-short-circuit-read-local-final/README.md).
The additional query-owner suite is 6/8; both strict-query must-None failures
reproduce in the preserved pre-read-local canonical binary. Production Clippy
is blocked by the unrelated retained-source cache type. Full gates remain
pending. The verified guard fix is published under the authorised checkpoint
exception; #2376 remains open while the coordinator reviews the baseline causes.

The two query-owner baseline failures are now scoped by 48 bounded public API
observations in
[`w210-query-purpose-review`](../rust/tcl-compiler/tests/data/w210-query-purpose-review/README.md).
The ordinary uncalled fresh-source path intentionally models a call after load;
Native entry and complete-normal proofs remain absent. A genuinely Conditional
factory-child fixture is under review while preserving the old negative
assertions, real after-load positives and unknown-history refusal. No production
source-purpose change has been made.

The 17:30 UTC checkpoint retains a further 24 public factory-child contrasts.
They confirm genuinely Conditional query/Normal-transfer refusal, independently
of compiler-only admission, and preserve actual-call selection. The reviewed
query correction is cfg(test)-only and awaits its focused retry. The root's
separate #2426 typed index-subject deduplication draft is also stashed for the
planned joint build; no unverified tests or product changes are published.

The 17:43 UTC integration retains both reviewed joint drafts uncompiled: the
query-fixture correction and the coordinator's typed IndexBounds identity
comparison. The source-graph formatting correction is integrated separately.
The functional guard receipt remains green; no new production result or
closure is claimed while the #940 priority compiler job runs.

The joint query/index verification is green: nine query-owner tests, five
source-value/index tests, seven guard tests and production compiler Clippy. The
old query failures were fixture-scope errors under the intended after-load
model; both negative assertions remain unchanged and now use a genuine
Conditional child. The earlier cache-type lint is fixed upstream. Exact current
fixture/guard output and source custody are in
[`verified-fixtures`](../rust/tcl-compiler/tests/data/w210-query-purpose-review/verified-fixtures/README.md).
No production source-purpose change was needed. #2376 remains open for the
coordinator's final receipt review; full workspace gates remain separate.

## Jim empty-loop phase audit (2026-11-04 21:58 UTC)

The published `02a9c35c6` C5 stage fix remains verified. Read-only current-library
probes distinguish two Jim gaps: a conditional procedure can retain a closed
registry foreach target while losing its structured source shape, and a
separate top-level empty foreach withdraws the source world before the
procedure preview. Generic object preparation changes the latter state from
non-dynamic to opaque/dynamic. AuthoredJim's existing literal-pool contract
permits text effects only; it supplies no C object conversion receipt.

The proposed next change is a stock-Jim, registry-issued empty paired-list
source phase after original argument/observer handling, with no binder/body,
Native compiler or physical-object permission. It is not implemented or built.
Native source/effect and available-observer controls have been captured on all
six engines in the [permanent raw receipt](../rust/tcl-compiler/tests/data/w210-empty-loop-exact-source/reviewed-stage-snapshot/jim-empty-phase-native/README.md).
The exact scripts and streams are in the two linked `evidence/tcl` records.
Jim malformed-list behaviour and unavailable observer interfaces remain
explicit. No #2441 closure or general conditional-body authority is claimed.

## Paused Jim implementation checkpoint (2026-11-04 22:03 UTC)

At the user's request to stop after the next completed issue and push all
current work, the isolated three-file Jim source-phase draft is included as an
**UNBUILT development checkpoint**. Its exact patch SHA-256 is
`1ff1dd50908f572fdc69c03cc5d9d5d4f279fd00d0de92b485c21a1fc70f9823`;
[the handoff](../rust/tcl-compiler/tests/data/w210-empty-loop-exact-source/reviewed-stage-snapshot/jim-empty-phase-native/README.md)
retains its source base, raw native controls and remaining obligations. No
build, test, Clippy, rust-check or prep-pr result is claimed for this draft.
The current compiled libraries still represent the separately verified
`ad4d43160` plus the published `02a9c35c6` C5 stage change. The new draft has no
regression tests yet. All older parked/rejected stashes remain preserved and
have not been restored. #2441 remains open for its two Jim boundaries.

The authorised [#2441 progress comment](https://github.com/bitwisecook/tcl-lsp/issues/2441#issuecomment-6643275900)
was read back byte-for-byte and the issue remains OPEN. It links the exact C5
source/artifact receipt, successful native controls and the unbuilt Jim
checkpoint `cf77ac5566`. All current work is committed for the coordinator's
final integration to `rust`; no new implementation, compilation or issue
closure is pending in this lane. Parent coordination owns the remaining
#2131 completion and final branch publication. Existing target libraries stay
unchanged until the last authorised read-only consumer releases them.
