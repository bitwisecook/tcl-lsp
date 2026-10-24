# ReadFlow foreign-card conformance failure — compiler owner

Status: owner repair delivered as0f4b8e19; not yet exercised by this perf
proof tree. The frozen837 failure remains evidence, not a current-branch
acceptance claim. No further perf production change is proposed.
Source base83774d43, full9 switched perf source. The existing test
`command_binding::generated_procedures::original_native_body_tests::original_read_card_requires_its_actual_vector_origin_and_snapshot`
fails BOTH new and forced-old at generated_procedures.rs645:
`assert!(!foreign.owns_original_read_tokens(tokens, input, registry));`.
The genuine-positive, changed word vector, missing binding and wrong-registry
assertions preceding it pass; the later changed-snapshot negative is UNREACHED.
Neither the test nor its assertions are changed, ignored or marked expected.

Raw complete23-case controls and source/binary receipts remain under
`evidence/current-83774d43-repaired-workflow/report-controls/`. All22 actual
authenticated-report/actual-diagnostic-realm controls pass BOTH in that run.
The extra ordinary ReadFlow test is outside that API inventory and was added
by perf while checking the necessary typing repair. Its failure is preserved
and it is included again in BOTH full/clean6 compiler comparisons. It is not
reported as an API pass or a passing ordinary unit test. The original strict
API-only control filter (all22 required; no missing/failing control accepted)
is restored for a fresh run before the complete corpus. No report field,
correspondence guard, test expectation or equality assertion is weakened.

The corrected source-head access reads the actual retained CommandTokens
WordExpr source span; original full-vector equality and every other predicate
remain. That one-line E0609 correction is canonical83774d43 and cargo check
passed0warnings. The foreign rejection failure's cause is not established.
The method documents geometry only, while its caller retains Module/body
correspondence; the owner must establish actual source/card/entry ownership
in the two genuine builds and fix the appropriate existing boundary. Keep
all positive and negative assertions and strict runtime/global/cache equality.
Do not substitute displayed names, equal bytes or a synthetic reporting owner
for actual retained current correspondence.

The compiler owner delivered0f4b8e19 after this failure was routed. It retains
the issuing OriginalSourceAnalysisOwner in the ReadFlow card and requires
actual pointer correspondence with the invocation binding before all existing
checks. It preserves the original test and strengthens its production gate.
Active837 corpus/compiler/client proofs remain frozen; the final rebase must
retain this repair and record any later validation separately.
