# Current external owner routing

Recorded 23 October 2026, 11:07 UTC.

Perf reports an authenticated complete-report proof running against frozen f149597fc926b82a69590e475d091527dd82173e. That Rust source contains the missing `run` API documentation diagnosed in the coordinator’s actual source135 v12 check. Work commit 1dd5e095e4b62e6f4b6c4f6fea357d874972876f moves the existing eight documentation lines from the temporary observer macro back to `propagation::run`; it also fixes Python capture-tool annotations. These are documentation/typing repairs, with no compiler semantic change. If this blocks the frozen proof, refreeze explicitly against the updated source and retain the original failed result; do not silently substitute bytes under the old identity. No latest perf failure is claimed here.

BIGIP’s c695061e132bb6cbd22b5bb0566fb740369aabd0 source-owner blockers are routed: compiler owns the original `when` event operand issuer; compiler-names owns genuine procedure-body invocation geometry and first-operand association. Existing assertions and the stop-after-opaque decision remain mandatory. The consumer patch has independent static review; its 23 PASS / 1 FAIL receipt remains limited to its recorded source. Generic Clippy counts in the older routing reports are historical; current remaining empty-assertion style edits are prepared separately.
