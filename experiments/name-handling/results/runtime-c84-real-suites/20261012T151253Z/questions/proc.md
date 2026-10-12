# Tcl 8.4 proc naming suite

Does the actual native runtime match the original Tcl 8.4.20 `proc.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/proc.test),
[C wrapper](../c/proc/input.tcl), [native wrapper](../runtime/proc/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 19 passed, 0 skipped, 0 failed (19 total) |
| Original native runtime | pass | 19 passed, 0 skipped, 0 failed (19 total) |

C evidence: [record](../c/proc/record.json), [stdout](../c/proc/stdout.txt),
[stderr](../c/proc/stderr.txt).
Runtime evidence: [record](../runtime/proc/record.json),
[stdout](../runtime/proc/stdout.txt), [stderr](../runtime/proc/stderr.txt).

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
