# interp: measured actual C8.6 / Native result

Actual C: 339 passed / 15 skipped / 0 failed; full suite summary, exit 0. Native: `host-refusal`, exit 2; No final summary; 9 observed assertion failures. Last started ID: `interp-11.4`. The started boundary is retained separately from completed assertions.

Observed failures cover current-interpreter deletion diagnostics (4.8), cross-interpreter aliases (9.3, 10.1/10.2/10.3/10.5/10.6/10.7), and missing interpreter target lookup (11.2). Native then refuses with `nested Runtime alias source interpreter native handler policy is not selected`.

Assertion failure IDs: `interp-10.1`, `interp-10.2`, `interp-10.3`, `interp-10.5`, `interp-10.6`, `interp-10.7`, `interp-11.2`, `interp-4.8`, `interp-9.3`

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/interp.md); [C record](../c/interp/record.json), [C stdout](../c/interp/stdout.txt), [C stderr](../c/interp/stderr.txt); [Native record](../runtime/interp/record.json), [Native stdout](../runtime/interp/stdout.txt), [Native stderr](../runtime/interp/stderr.txt).

First observed Native assertion failure: `interp-4.8`. First terminal host-refusal boundary: `interp-11.4`.

Observed skip constraints (counts): actual C `{"!singleTestInterp": 2, "knownBug": 7, "testinterpdelete": 6}`; Native `{}`. Native skip IDs: none observed before termination. These counts apply only to the retained stream. [Ordered observations](../observations.json).
