# AnalysisResult comparison blocker — compiler owner

Status: **Blocked/routed; no comparator change made by perf.**

The aliases fixture `rust/tcl-compiler/tests/data/name-consumers/aliases.tcl`
fails complete old/new/old AnalysisResult PartialEq. It also fails old/old/old
with ZZ_OLD_ENV=1 (all lexer/perf query optimizations disabled). Reusing one
independently prepared ResolvedAnalysisInput does not remove the mismatch.
Diagnostics, every comparable public field checked by the standalone diagnostic,
and retained realm/completed world compare equal. No fields were stripped from
the actual full-result verdict.

A separate debug process traced the unmodified Eq routine at latest production
base e4a01bbe with the full batch forced old. Its first false helper is call 75:
`ObjectHandleFacts::analysis_report_eq`, object_types/report_comparison.rs.
Exact trace: evidence/latest-e4a01bbe/aliases-private-eq-trace.out.
The diagnostic only stepped the comparator; it changed no field or assertion.

That comparator requires admitted report-source correspondence; otherwise it
falls back to raw ObjectHandleFacts equality. Raw runtime owner identities are
intentionally distinct across independently constructed modules. The owner must
check why this fixture lacks/does not match report correspondence, or which
specific report clause differs. This handoff does NOT assert that the fallback
is wrong, that caches may be ignored, or that hosted/native domains may be merged.
An all-old f5-irules control ALSO fails complete equality:
samples/irules/cookbook_http_collect.irul, diagnostics_equal=true,
old1 != old2, middle result matches neither endpoint. Exact output:
evidence/latest-e4a01bbe/irule-old-only-report-control.out. This control is
not yet privately traced; hosted inputs are explicitly excluded from the
current report-source capture eligibility, so the owner should inspect that
correspondence path as well.

A valid resolution must retain complete source/input/registry, authentic body
coordinates and independent ownership checks. Do not force Eq true, omit the
object facts, compare only diagnostics, or retry until an old sample matches.
Until this is resolved, perf cannot claim the required complete corpus equality
or mark perf-realm-interprocedural.patch proven. The six owned production files
remain uncommitted.

A second machine trace follows the object-report helper itself. Both filtered
report-source options return None; the function takes the None/None fallback
and raw fact equality returns false. It never enters the coordinate-report
comparison. Exact trace: evidence/latest-e4a01bbe/aliases-object-report-trace.out.
This narrows the owner investigation to report-source capture/currentness or a
legitimate absence of correspondence. It is not permission to remove the guard
or equate independent runtime owners.

## Additional latest corpus blocker: E003 diagnostic subjects

The latest full corpus also fails on
samples/diagnostics/E003_proc_call_arity/example.tcl, with unequal old endpoints.
An isolated all-old control using the same completed latest executable ALSO
fails complete PartialEq (exit 101). Unlike the aliases diagnostic, its public
diagnostics vector itself compares unequal. Exact outputs/inputs:
evidence/latest-e4a01bbe/e003-old-only-report-control.out and
e003-old-only-inputs.json. No private helper trace was taken for this fixture.

The current binary's line `MISMATCH diagnostics_equal=true` compares formatted
Debug strings ONLY. The independent `FIELD_DIFF diagnostics` line is actual
PartialEq and shows that vector is unequal despite identical Debug text. This
label is misleading; the recorded next-build probe now separately prints
diagnostics_debug_equal and diagnostics_equal. The running executable and
its log are preserved unchanged. The full AnalysisResult assertion always
remains the actual verdict; there was no weakened proof.

Diagnostic derives PartialEq over subject, code, span, message, severity and
fixes. Typed source subjects must retain their real owner/evidence. Route the
non-repeatable diagnostic/report correspondence to the compiler owner alongside
the aliases object-report blocker. This note does not identify the specific
subject variant or authorise omitting it, replacing equality with Debug, or
using report text as runtime identity.

An owner-level design option is report-scoped subject correspondence inside
AnalysisResult comparison, authenticated by independently retained input,
registry, complete source and unique genuine body coordinates. Keep runtime
DiagnosticSubject/receipt equality strict wherever it protects ownership;
do not globally change it to printed-name/source-byte equality. This is a
proposal for the owner to assess, not a proven implementation. Hosted source
needs its own authentic hosted-policy/container/configuration correspondence;
the existing Native/versioned eligibility must not be extended by erasing
that distinction.

The isolated forced-old control now covers ALL 15 hosted iRules: all 15 fail
full equality and all old endpoints are non-repeatable. Exact inputs/source/
binary digest: evidence/latest-e4a01bbe/irules-all-old-inputs.json; raw output:
irules-all-old-control.out. Only the earlier aliases helper was privately
traced; do not attribute every iRule to that helper without tracing it.

Before changing AnalysisResult::eq, audit memo/Salsa cutoff consumers as well:
report correspondence must not make a stale runtime subject current or pair an
old result's owners with a fresh module/host publication lease. If these equality
scopes differ, an explicit report-comparison operation for the harness is safer
than changing global equality. This remains an owner design question; perf
changed no comparator, currentness check or source authority.

## Fresh current-work confirmation: 39b39296

A separate shallow checkout at 39b39296 rebuilt all local crates, retaining
only third-party caches. The full switched batch built with zero warnings
(default dev opt=0/debug=0, 6m56s). BOTH E003 and aliases still fail complete
AnalysisResult equality in all-old AND mixed controls. E003 diagnostics
PartialEq is false while Debug text agrees; aliases diagnostics PartialEq
is true. Current outputs: evidence/current-39b39296/{tcl-old-only,tcl-mixed}.out;
source/binary/cache receipts are adjacent. No private helper was traced on
this newer base; the detailed machine trace above remains scoped to e4a01bbe.
Hosted all-old/mixed controls completed: ALL 15 fail in BOTH modes, zero
panics. Both Tcl fixtures also fail in both modes. Current source failure
is confirmed independently of the older broad proof; no production code
was committed and neither patch is declared proven.

## Authenticated API controls at0322d9cd

Compiler/source-report owner: both modes now finish12PASS/3FAIL out of15,
zero warnings, identical inventories and zero pass-to-fail. Exact controls,
source/binary receipts and summary are in evidence/current-0322d9cd/. The
Native E003 positive and foreign-subject refusal controls pass. Both Native
original_aliases and the additive supported_logical_alias controls stop at
`actual Named and NamespaceScript owners at one original declaration`, before
their full correspondence assertions. The genuine hosted control refuses
actual capture with CurrentOwnerUnavailable. Do not change these assertions
or infer an exact producer cause from the assertion text alone.

Separate public-API smoke linked from the exact normal Cargo libraries gives
complete authenticated Equal old1/old2, old1/new and new/old2 for the actual
E003 and aliases corpus fixtures, forced-old and mixed. Raw strict runtime Eq
is false independently and unchanged. This distinguishes report-level progress
from the still-failing issuer controls. It is not full102+15 corpus proof and
provides no permission to accept Unavailable, omit any field or release the
realm/interprocedural patch. Hosted full actual correspondence remains a
blocking owner task. Perf continues only independent regression checks until
the complete original controls can admit the full proof.

Current unchanged-control tracing narrows hosted capture further. Raw original
word and hosted metadata are Some, then no hosted-event-units trace appears.
The intervening original-word-vector / selected-command OR guard refuses;
which boolean is true remains unobserved. The later bodies.event-word None
is downstream, so do not infer a stored word was successfully retained.
`perf-hosted-event-refusal-trace.patch` preserves the original comparisons,
single evaluation, OR short-circuit and return paths while exposing those two
refusals under the existing cfg(test) trace flag. It is an off-tree, uncompiled,
unrun diagnostic proposal routed to compiler, never a semantic repair.

The genuine Logical alias control captures named0/unit2: NamespaceScript and
OriginalDeclaration at site24, declaration extents24..55 versus42..54. Its
original Named+NamespaceScript same-declaration assertion still fails. Full
raw current trace and compact observations are in report-refusal-trace/.
Neither this observation nor the successful actual native smoke authorizes
changing the role expectation, dropping any body or forwarding the perf patch.

## Successor a8e7bc7a

The owner has landed source-procedure traversal, hosted event-word issuance
and exact authentic alias role/purpose/span controls. The0322 failures above
are superseded source evidence. Fresh16 report/realm controls are building;
availability is not inferred from source review. The old logging draft must
not be applied to the successor. Full corpus and compiler-patch acceptance
remain pending actual new-source gates, with every field/assertion retained.

Actual production smoke at a8e7 still refuses complete cookbook_http_collect
report comparison: all3 old1/new/old2 pairs return CurrentOwnerUnavailable.
The example links the exact completed normal Cargo libraries with source and
binary hashes; executable removed. This is actual corpus-source evidence,
independent of the source_graph.rs lib-test type error. No new private
refusal stage is attributed without tracing. A tiny genuine hosted public
control is being measured separately; full proof remains unavailable.
