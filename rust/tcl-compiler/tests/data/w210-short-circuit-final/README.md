# Canonical command-result short-circuit verification

The 29-line consumer at frozen base `b9a77f769` passes all six focused unit tests
and all 130 integration assertions. The latter cover 60 W210 cases and 70
source-value/admission cases across Tcl 8.4–9.1. All 41,082 frozen file hashes
match through compilation and runtime. Exact binary digests, preserved `/tmp`
artifact paths, commands and raw outputs are included here.

The consumer uses an existing completed command result only for canonical `0`
or `1` without a numeric receipt, and only for a direct command or logical
negations of it. It preserves left-hand effects and all abrupt alternatives,
uses the shared Boolean owner and creates no source, frame, getter or compiler
receipt. Whole-expression values remain under the existing expression producer.
The final-store source values are conditional on represented Normal paths;
they are not physical objects or compiler admissions.

This fixes the known-result/called path, including the verbatim #2376 `p 0`
sample and its later real read. It does not resolve the complete issue. A tiny
public `Analyser` probe linked to the exact emitted production rlibs tested 35
uncalled-procedure cases. All five profiles still warn on safe AND, negated OR
and double-negation guards; the later real read remains hidden by that earlier
condition warning. Unsafe OR/negated-AND and standalone unguarded controls warn
in all profiles, so these are active diagnostic failures rather than abstention.
The probe verdicts use diagnostic code/count and source span, never message text.
The probe source is archived as text, not installed as a failing CI test.

The remaining fix needs existing read-local condition-edge/source-read owners
with structured variable identities and effect invalidation. #2376 remains
open. Formatting checks pass; full `rust-check` and `prep-pr` remain pending
under the development-push exception and must pass before the PR is approved.
