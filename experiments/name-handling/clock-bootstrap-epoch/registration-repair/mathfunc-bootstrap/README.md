# C8.6 mathfunc bootstrap and compiler currency

Question: does the runtime's pre-init mathfunc registration match actual C8.6,
and what compiler currency remains after the ordinary min procedure birth?

The exact [input](../mathfunc-min-replacement-input.tcl) runs a procedure that
publishes `::tcl::mathfunc::min` before a specialised set. The independently
retained actual C8.6 [min capture](../c-tcl-mathfunc-min.stdout.txt) and
[max capture](../c-tcl-mathfunc-max.stdout.txt) start with both entries absent.
Their respective procedure births leave compiler/resolver epochs `(17, 0)`;
the new mathfunc procedure entries have Present hooks. This differs from the
NULL-hook clock procedure births and does not authenticate arbitrary handler
effects.

The frozen runtime [stderr](../runtime-mathfunc-min-replacement.stderr.txt) is
an execution refusal, and its [stdout](../runtime-mathfunc-min-replacement.stdout.txt)
is empty. The `runtime-mathfunc-min-replacement` row of [records.json](../records.json)
records exit 2, the exact input/binary hashes and selected library. Runtime
source registers all math functions, including synthetic min/max, without
selected native compiler descriptors. Replacing that Unknown token makes
compiler currency unavailable before the following specialised command.

Original C constructor and real init source ranges are retained in
[source excerpts](../original-source-excerpts.json). Exact runtime/registry
source snapshots and binary build observations are shared through
[selected sources](../build-selected-sources.json) and
[build records](../build-records.json); no duplicate or altered capture is
introduced by this page.

The actual matching real-init [refusal](../runtime-startup.stderr.txt) is
consistent with this separate earlier bootstrap mismatch. An exact internal
init instruction location is unmeasured, and successful library initialisation
is not established. Mathfunc bootstrap/procedure compiler support remains open.
C8.4, C8.5, C9, Jim and F5 have no mathfunc answer from this question.

To capture new C8.6/runner controls without overwriting retained evidence:

```
python3 experiments/name-handling/clock-bootstrap-epoch/registration-repair/capture.py --question registration --output-dir /tmp/name-mathfunc-replay-new --runner /tmp/name-clock-registration-repair-runner-final
```

The output destination must not already exist. A replay records its actual
runner hash; it inherits no successful build or semantic claim from a filename.
