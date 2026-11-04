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
| #2374, #2441 | Distinct literal array elements and empty-loop variables emit W210; C Tcl and Jim confirm the errors. | Verified baseline |
| #2376 | Read-local fix `07a1bd0ac` passes 185 integration rows and 14 guard cases. Current joint verification passes 9 query-owner and 7 guard tests plus production Clippy; baseline failures were wrongly scoped after-load fixtures and are corrected without changing assertions. | Functional fix verified; coordinator closure review |
| #2379 | Absent-array size/names/get emit no W210, matching C Tcl and Jim. | Verified baseline |
| #2294 | Work already carries/rebases `BodyFragment::proven_sites`; whole-file W230 is absent in the parity witness, before testing per-item parity. | Residual under investigation |
| #2268 | Existing witness fails its dead-store control: an unused literal element has no W211. The computed-element read control passes. | Reproduced residual |
| #2289 | Complete current linked file and called creator have no W140, with positive W140 controls across Tcl 8.4–9.1. | Current input verified |
| #2296 | Existing computed-array-key test passes in `name_lookup_issue_witnesses.rs`. | Verified baseline |
| #2243 | Memoised/uncached seeded-global parity still requires a fresh database regression. | Pending |
| #2247 | Qualified dictionary implementation emits neither W002 nor W211; W143 correctly advises the public spelling. | Verified baseline |
| #2286 | Exact quoted/current linked inputs have no W210, but the original Tcltest control still emits 0 instead of 2 warnings in all five profiles; direct/alias controls also miss their warnings. | Reported false positives absent; admission gap remains |
| #2290 | Latest public `tclmeasure.tcl` emits no W210; upstream source differs from the historical line references. | Current source verified |
| #2297 | Caller-variable read through upvar produces no W220. | Verified baseline |
| #2305, #2306 | Qualified-root writes and same-frame alias writes preserve branch output and produce no stale I230. | Verified baseline |
| #2323 | Catch followed by its result guard still reports W210 for the successfully assigned file handle. | Reproduced residual |
| #2324, #2325 | Computed typed receivers and expanded deferred callbacks still require focused verification. | Pending |
| #2367, #2370, #2410, #2412, #2414, #2415 | Reported fixed programs preserve original execution after optimisation; associated W211/W220 defects are absent. | Verified baseline |
| #2455 | The nested expression read produces no W220. | Verified baseline |
| #2407, #2408 | Reports describe conservative summary precision limits. | Pending |
| #1778 | The reported distinct colon-edge commands preserve their original distinct output under C Tcl 8.6–9.1 optimisation. Additional lifetime and index consumers require verification. | Partial verification |
| #1796 | ResolveContext selects exact frames, but UpvarInfo still collapses all beyond-caller distances. | Confirmed remaining foundation |
| #2332 | Qualified and variable-linked substitution reads are impure with global effects; explicit set reads remain conservatively impure with unknown effects. | Verified baseline |

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
[`w210-short-circuit-oracles.json`](../rust/tcl-compiler/tests/data/w210-short-circuit-oracles.json).
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
[`w210-short-circuit-proposal.patch`](../rust/tcl-compiler/tests/data/w210-short-circuit-proposal.patch).
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
