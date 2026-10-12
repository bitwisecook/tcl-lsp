# Tcl 8.4 namespace naming suite

Does the actual native runtime match the original Tcl 8.4.20 `namespace.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/namespace.test),
[C wrapper](../c/namespace/input.tcl), [native wrapper](../runtime/namespace/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 186 passed, 3 skipped, 0 failed (189 total) |
| Original native runtime | fail | 176 passed, 3 skipped, 10 failed (189 total) |

C evidence: [record](../c/namespace/record.json), [stdout](../c/namespace/stdout.txt),
[stderr](../c/namespace/stderr.txt).
Runtime evidence: [record](../runtime/namespace/record.json),
[stdout](../runtime/namespace/stdout.txt), [stderr](../runtime/namespace/stderr.txt).

## Measured gaps

| Cases | Captured discrepancy |
| --- | --- |
| namespace-8.5, namespace-8.6, namespace-8.7 | ErrorInfo/ErrorCode preservation through child namespace teardown and traces. |
| namespace-10.6, namespace-10.7 | Forgotten imports remain callable; both bodies return normally where the C-backed assertion expects error completion. |
| namespace-14.13 | Creating an empty non-global namespace is accepted instead of the measured C rejection. |
| namespace-25.6 | ErrorInfo contains stale earlier error text before the current namespace-eval failure. |
| namespace-28.1 | No-argument import query returns surviving Tcltest imports instead of the expected empty result in this ordered suite state. |
| namespace-34.2, namespace-34.3 | Missing-name flag invocations return empty success instead of the measured argument error. |

These classifications describe captured behaviour in the full ordered suite.
They do not establish an isolated cause or prove that earlier state differences
cannot affect a later assertion. Reuse the retained inputs before attributing a
fix to a specific owner.

Observed failing IDs: `namespace-10.6`, `namespace-10.7`, `namespace-14.13`, `namespace-25.6`, `namespace-28.1`, `namespace-34.2`, `namespace-34.3`, `namespace-8.5`, `namespace-8.6`, `namespace-8.7`.

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
