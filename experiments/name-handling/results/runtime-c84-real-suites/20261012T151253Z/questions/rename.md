# Tcl 8.4 rename naming suite

Does the actual native runtime match the original Tcl 8.4.20 `rename.test`
under its real matching initialization and Tcltest harness?

Inputs: [unaltered upstream source](../oracle-inputs/tests/rename.test),
[C wrapper](../c/rename/input.tcl), [native wrapper](../runtime/rename/input.tcl).
[Shared provenance](../provenance.json) identifies the native/C binaries,
matching library hashes, selected original-native construction and bounded
invocations. The [runner source](../runner-source.txt) retains exact constraints
and status classification. No backend overlay or Logical reporting adapter ran.

| Provider | Status | Counts |
| --- | --- | --- |
| Real C Tcl 8.4.20 | pass | 11 passed, 0 skipped, 0 failed (11 total) |
| Original native runtime | pass | 11 passed, 0 skipped, 0 failed (11 total) |

C evidence: [record](../c/rename/record.json), [stdout](../c/rename/stdout.txt),
[stderr](../c/rename/stderr.txt).
Runtime evidence: [record](../runtime/rename/record.json),
[stdout](../runtime/rename/stdout.txt), [stderr](../runtime/rename/stderr.txt).

Upstream skips retain their original constraints and supply no oracle answer.
This experiment covers C Tcl 8.4 only; Jim and F5 were not queried for this suite.
