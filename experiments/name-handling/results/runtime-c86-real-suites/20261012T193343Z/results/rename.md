# rename: measured actual C8.6 / Native result

Actual C: 11 passed / 8 skipped / 0 failed; full suite summary, exit 0. Native: `completed-suite`, exit 0; 11 / 8 / 0. Last started ID: `rename-6.1`. The started boundary is retained separately from completed assertions.

The full file completes with identical summary counts and no assertion failure. Eight upstream testdel-dependent cases are skipped by the unchanged harness.

Assertion failure IDs: None in the retained stream.

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/rename.md); [C record](../c/rename/record.json), [C stdout](../c/rename/stdout.txt), [C stderr](../c/rename/stderr.txt); [Native record](../runtime/rename/record.json), [Native stdout](../runtime/rename/stdout.txt), [Native stderr](../runtime/rename/stderr.txt).

First observed Native assertion failure: none before termination. First terminal host-refusal boundary: none; see recorded completion or script error.

Observed skip constraints (counts): actual C `{"testdel": 8}`; Native `{"testdel": 8}`. Native skip IDs: `rename-4.1`, `rename-4.2`, `rename-4.3`, `rename-4.4`, `rename-4.5`, `rename-4.6`, `rename-4.7`, `rename-4.8`. These counts apply only to the retained stream. [Ordered observations](../observations.json).
