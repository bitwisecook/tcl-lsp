# ooNext2: measured actual C8.6 / Native result

Actual C: 57 passed / 5 skipped / 0 failed; full suite summary, exit 0. Native: `process-error`, exit 1; No final summary; 3 observed assertion failures. Last started ID: `next-tailcall-simple-1`. The started boundary is retained separately from completed assertions.

Observed failures cover nextto argument presentation (oo-nextto-2.5) and mixin/filter call-chain order (oo-call-1.20/1.21). The process exits 1 with `can't rename to "::puts": command already exists` after starting next-tailcall-simple-1. This ordinary error has no final suite summary; it is separate from a host refusal.

Assertion failure IDs: `oo-call-1.20`, `oo-call-1.21`, `oo-nextto-2.5`

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/ooNext2.md); [C record](../c/ooNext2/record.json), [C stdout](../c/ooNext2/stdout.txt), [C stderr](../c/ooNext2/stderr.txt); [Native record](../runtime/ooNext2/record.json), [Native stdout](../runtime/ooNext2/stdout.txt), [Native stderr](../runtime/ooNext2/stderr.txt).

First observed Native assertion failure: `oo-nextto-2.5`. First terminal host-refusal boundary: none; see recorded completion or script error.

Observed skip constraints (counts): actual C `{"memory": 5}`; Native `{"memory": 5}`. Native skip IDs: `oo-call-1.18`, `oo-call-1.19`, `oo-call-2.13`, `oo-call-2.14`, `oo-call-3.2`. These counts apply only to the retained stream. [Ordered observations](../observations.json).
