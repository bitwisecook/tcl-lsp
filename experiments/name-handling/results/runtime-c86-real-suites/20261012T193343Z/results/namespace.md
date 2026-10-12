# namespace: measured actual C8.6 / Native result

Actual C: 310 passed / 3 skipped / 0 failed; full suite summary, exit 0. Native: `host-refusal`, exit 2; No final summary; 12 observed assertion failures. Last started ID: `namespace-51.13`. The started boundary is retained separately from completed assertions.

Observed failures cover child namespace-deletion errorInfo (8.5/8.6); rename/import chains (9.8/9.9, 10.4/10.5/10.6/10.8); empty namespace creation (14.13); ensemble argument presentation (50.2/50.6/50.7). Native then refuses with `captured original command binding native handler policy is not selected`. Ordered setup/cleanup state is retained; these groupings do not isolate a root cause.

Assertion failure IDs: `namespace-10.4`, `namespace-10.5`, `namespace-10.6`, `namespace-10.8`, `namespace-14.13`, `namespace-50.2`, `namespace-50.6`, `namespace-50.7`, `namespace-8.5`, `namespace-8.6`, `namespace-9.8`, `namespace-9.9`

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/namespace.md); [C record](../c/namespace/record.json), [C stdout](../c/namespace/stdout.txt), [C stderr](../c/namespace/stderr.txt); [Native record](../runtime/namespace/record.json), [Native stdout](../runtime/namespace/stdout.txt), [Native stderr](../runtime/namespace/stderr.txt).

First observed Native assertion failure: `namespace-8.5`. First terminal host-refusal boundary: `namespace-51.13`.

Observed skip constraints (counts): actual C `{"knownBug": 1, "memory": 1, "testevalobjv": 1}`; Native `{}`. Native skip IDs: none observed before termination. These counts apply only to the retained stream. [Ordered observations](../observations.json).
