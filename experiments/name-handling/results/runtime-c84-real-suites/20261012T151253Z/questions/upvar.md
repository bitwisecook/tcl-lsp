# Tcl 8.4 upvar naming suite

Does the actual native runtime match the original Tcl 8.4.20 `upvar.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/upvar.test),
[C wrapper](../c/upvar/input.tcl), [native wrapper](../runtime/upvar/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 38 passed, 0 skipped, 0 failed (38 total) |
| Original native runtime | process-error | 0 observed assertion failures; no completed summary |

C evidence: [record](../c/upvar/record.json), [stdout](../c/upvar/stdout.txt),
[stderr](../c/upvar/stderr.txt).
Runtime evidence: [record](../runtime/upvar/record.json),
[stdout](../runtime/upvar/stdout.txt), [stderr](../runtime/upvar/stderr.txt).

The last started case is `upvar-1.1`. The exact host refusal is:

```text
run_script: execution refused: original simple variable cell native handler policy is not selected
```

This supplies no Tcl completion or final suite counts. Cases after that boundary
are unmeasured. Earlier assertion failures remain separately observable.

## Measured gaps

| Cases | Captured discrepancy |
| --- | --- |
| upvar-1.1 | The original simple-variable cell prerequisite refuses during the first started case; there is no case verdict. |

These classifications describe captured behaviour in the full ordered suite.
They do not establish an isolated cause or prove that earlier state differences
cannot affect a later assertion. Reuse the retained inputs before attributing a
fix to a specific owner.

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
