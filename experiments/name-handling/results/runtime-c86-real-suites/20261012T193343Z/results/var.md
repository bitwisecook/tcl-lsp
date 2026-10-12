# var: measured actual C8.6 / Native result

Actual C: 90 passed / 20 skipped / 0 failed; full suite summary, exit 0. Native: `host-refusal`, exit 2; No final summary; 0 observed assertion failures. Last started ID: `var-1.15`. The started boundary is retained separately from completed assertions.

Native refuses with `original variable lookup diagnostic native handler policy is not selected`. No assertion-failure block precedes this refusal.

Assertion failure IDs: None in the retained stream.

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/var.md); [C record](../c/var/record.json), [C stdout](../c/var/stdout.txt), [C stderr](../c/var/stderr.txt); [Native record](../runtime/var/record.json), [Native stdout](../runtime/var/stdout.txt), [Native stderr](../runtime/var/stderr.txt).

First observed Native assertion failure: none before termination. First terminal host-refusal boundary: `var-1.15`.

Observed skip constraints (counts): actual C `{"memory": 3, "testgetvarfullname": 3, "testsetnoerr": 12, "testupvar": 2}`; Native `{}`. Native skip IDs: none observed before termination. These counts apply only to the retained stream. [Ordered observations](../observations.json).
