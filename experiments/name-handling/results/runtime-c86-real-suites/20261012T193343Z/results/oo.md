# oo: measured actual C8.6 / Native result

Actual C: 289 passed / 17 skipped / 0 failed; full suite summary, exit 0. Native: `host-refusal`, exit 2; No final summary; 10 observed assertion failures. Last started ID: `oo-7.10`. The started boundary is retained separately from completed assertions.

Observed failures cover TclOO package/version and initial object inventory (0.9, 1.21), superclass construction/dependency behavior (1.18.4/1.18.5), constructor tailcall (2.6), argument presentation (2.7, 6.18), object namespace deletion (3.5a), and local `my` forwarding across rename (6.19/6.20). Native then refuses with `original command deletion caller namespace native handler policy is not selected`.

Assertion failure IDs: `oo-0.9`, `oo-1.18.4`, `oo-1.18.5`, `oo-1.21`, `oo-2.6`, `oo-2.7`, `oo-3.5a`, `oo-6.18`, `oo-6.19`, `oo-6.20`

Captured unchanged harness configuration: c: NH_CONSTRAINT singleProcess 1 singleTestInterp 1; runtime: NH_CONSTRAINT singleProcess 1 singleTestInterp 1. Both providers reach real init and real Tcltest; no overlay or changed package is used. Skipped tests are unmeasured and aborted tails gain no pass/fail claim.

[Source/question](../questions/oo.md); [C record](../c/oo/record.json), [C stdout](../c/oo/stdout.txt), [C stderr](../c/oo/stderr.txt); [Native record](../runtime/oo/record.json), [Native stdout](../runtime/oo/stdout.txt), [Native stderr](../runtime/oo/stderr.txt).

First observed Native assertion failure: `oo-0.9`. First terminal host-refusal boundary: `oo-7.10`.

Observed skip constraints (counts): actual C `{"knownBug": 3, "memory": 14}`; Native `{"knownBug": 2, "memory": 7}`. Native skip IDs: `oo-0.3`, `oo-0.4`, `oo-0.5.1`, `oo-0.5.2`, `oo-0.8`, `oo-1.18.1`, `oo-1.18.2`, `oo-1.23`, `oo-1.24`. These counts apply only to the retained stream. [Ordered observations](../observations.json).
