# proc: measured actual C8.6 / Native result

Actual C: 29 passed / 9 skipped / 0 failed; full suite summary, exit 0. Native: `completed-suite`, exit 0; 29 / 9 / 0. Last started ID: `proc-7.6`. The started boundary is retained separately from completed assertions.

The full file completes with identical summary counts and no assertion failure.

Assertion failure IDs: None in the retained stream.

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/proc.md); [C record](../c/proc/record.json), [C stdout](../c/proc/stdout.txt), [C stderr](../c/proc/stderr.txt); [Native record](../runtime/proc/record.json), [Native stdout](../runtime/proc/stdout.txt), [Native stderr](../runtime/proc/stderr.txt).

First observed Native assertion failure: none before termination. First terminal host-refusal boundary: none; see recorded completion or script error.

Observed skip constraints (counts): actual C `{"procbodytest": 9}`; Native `{"procbodytest": 9}`. Native skip IDs: `proc-4.1`, `proc-4.2`, `proc-4.3`, `proc-4.4`, `proc-4.5`, `proc-4.6`, `proc-4.7`, `proc-4.8`, `proc-4.9`. These counts apply only to the retained stream. [Ordered observations](../observations.json).
