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
