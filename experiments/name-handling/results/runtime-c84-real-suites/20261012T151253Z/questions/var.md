# Tcl 8.4 var naming suite

Does the actual native runtime match the original Tcl 8.4.20 `var.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/var.test),
[C wrapper](../c/var/input.tcl), [native wrapper](../runtime/var/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 64 passed, 0 skipped, 0 failed (64 total) |
| Original native runtime | pass | 64 passed, 0 skipped, 0 failed (64 total) |

C evidence: [record](../c/var/record.json), [stdout](../c/var/stdout.txt),
[stderr](../c/var/stderr.txt).
Runtime evidence: [record](../runtime/var/record.json),
[stdout](../runtime/var/stdout.txt), [stderr](../runtime/var/stderr.txt).

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
