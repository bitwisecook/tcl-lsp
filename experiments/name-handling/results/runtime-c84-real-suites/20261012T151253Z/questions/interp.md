# Tcl 8.4 interp naming suite

Does the actual native runtime match the original Tcl 8.4.20 `interp.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/interp.test),
[C wrapper](../c/interp/input.tcl), [native wrapper](../runtime/interp/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 261 passed, 5 skipped, 0 failed (266 total) |
| Original native runtime | process-error | 13 observed assertion failures; no completed summary |

C evidence: [record](../c/interp/record.json), [stdout](../c/interp/stdout.txt),
[stderr](../c/interp/stderr.txt).
Runtime evidence: [record](../runtime/interp/record.json),
[stdout](../runtime/interp/stdout.txt), [stderr](../runtime/interp/stderr.txt).

The last started case is `interp-11.4`. The exact host refusal is:

```text
run_script: execution refused: nested Runtime alias source interpreter native handler policy is not selected
```

This supplies no Tcl completion or final suite counts. Cases after that boundary
are unmeasured. Earlier assertion failures remain separately observable.

## Measured gaps

| Cases | Captured discrepancy |
| --- | --- |
| interp-1.6, interp-3.4, interp-3.8 | `interp slaves` wrong-argument diagnostics render the newer `children` spelling. |
| interp-4.8 | Deleting the current interpreter has a different rejection message. |
| interp-8.3 | Alias-invocation wrong-argument presentation differs. |
| interp-9.3, interp-10.1, interp-10.2, interp-10.3, interp-10.5, interp-10.6, interp-10.7 | Cross-interpreter/hidden alias cases raise a single-interpreter-support error instead of their C-backed outcomes. |
| interp-11.2 | Alias-target query checks the alias before reporting a nonexistent interpreter. |

These classifications describe captured behaviour in the full ordered suite.
They do not establish an isolated cause or prove that earlier state differences
cannot affect a later assertion. Reuse the retained inputs before attributing a
fix to a specific owner.

Observed failing IDs: `interp-1.6`, `interp-10.1`, `interp-10.2`, `interp-10.3`, `interp-10.5`, `interp-10.6`, `interp-10.7`, `interp-11.2`, `interp-3.4`, `interp-3.8`, `interp-4.8`, `interp-8.3`, `interp-9.3`.

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
