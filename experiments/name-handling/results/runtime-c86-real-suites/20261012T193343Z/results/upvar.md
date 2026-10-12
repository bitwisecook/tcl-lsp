# upvar: measured actual C8.6 / Native result

Actual C: 57 passed / 8 skipped / 0 failed; full suite summary, exit 0. Native: `host-refusal`, exit 2; No final summary; 0 observed assertion failures. Last started ID: `upvar-1.1`. The started boundary is retained separately from completed assertions.

Native refuses with `original simple variable cell native handler policy is not selected` at the first started case. No assertion-failure block precedes refusal.

Assertion failure IDs: None in the retained stream.

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/upvar.md); [C record](../c/upvar/record.json), [C stdout](../c/upvar/stdout.txt), [C stderr](../c/upvar/stderr.txt); [Native record](../runtime/upvar/record.json), [Native stdout](../runtime/upvar/stdout.txt), [Native stderr](../runtime/upvar/stderr.txt).

First observed Native assertion failure: none before termination. First terminal host-refusal boundary: `upvar-1.1`.

Observed skip constraints (counts): actual C `{"testupvar": 8}`; Native `{}`. Native skip IDs: none observed before termination. These counts apply only to the retained stream. [Ordered observations](../observations.json).
