# Selected C8.6 private format argument worker

The selected native C8.6 runtime now registers `::tcl::clock::ParseFormatArgs`
as a registry-owned object command with a measured NULL compiler hook,
independent of purity. The shared `tcl-cmd-core::clock` parser uses the existing
original-object option, Boolean, wide-integer and list owners. It returns three
elements, retains supplied option objects, uses retained default/GMT literals,
and publishes the original conflict literal. Other selected releases gain no
worker admission from this bounded implementation.

The [fresh object-vector capture](20261012T205500Z/provenance.json) retains exact
commands, exits, streams, original inputs, source/binary/library hashes and the
capture/control sources. The rebuilt runtime and fresh matching C8.6 `tclsh`
emit byte-identical [eleven-case output](20261012T205500Z/runtime/eleven/stdout.txt):
four successes and seven caught worker errors. Both process exits are zero;
neither stderr contains bytes. This includes repeated-format last-value wins,
unique `-g`, Boolean-before-time and time-before-conflict validation, conflict
even for false GMT, and acceptance of `9223372036854775808`.

The [C object-vector control](20261012T205500Z/c-object-control/stdout.txt)
measures stable default literal pointers, original supplied object pointers,
three list elements and the NULL hook. Default format refcounts are two after
one retained result and three after two. Actual C shares its ClockClientData
pool across seven support workers: deleting ParseFormatArgs alone leaves the
pool reference; retiring the other six removes it. The bounded runtime owns
only its implemented worker and releases that worker's pool on retirement.
Returned lists retain their original members and remain usable. Full
seven-worker shared pool lifetime and constructor conformance are not claimed.

The [earlier string-API control](20261012T205300Z/c-object-control/stdout.txt)
is preserved. Its `Tcl_Eval` calls reached the legacy string-result bridge and
observed reparsed result elements, so its pointer/refcount observations do not
establish the worker's original result-object identity. The refined control
calls `Tcl_EvalObjv` directly. Both controls and all raw outcomes remain intact.

The original [all.tcl wrapper](20261012T205500Z/all-tcl-var.tcl) uses real
matching init, unchanged real tcltest, and the original runAllTests path with
`var.test` selected. Fresh C exits zero with 110 total, 90 passed, 20 skipped
and zero failed. The rebuilt runtime reaches init, harness and orchestration,
then [exits one](20261012T205500Z/runtime/all-tcl-var/stderr.txt) at missing
`getenv` before entering any suite. ParseFormatArgs is no longer that stop;
no runtime tcltest suite pass is claimed. Other clock workers and map order
remain outside this implementation. F5 is unmeasured.

Focused runtime regressions passed three tests: the existing relative reading
epochs, the eleven exact original C8.6 rows, and retained supplied/default
object identity after worker retirement with absent compiler hook and unchanged
relative compiler epochs. The focused registry unit test passed across release
surface selections and confirmed conservative purity independent of NULL hook.
The broader registry package test failed to compile the unrelated
`differential_fold` integration target because its `compare_string` signature
differs from the trait. An earlier wrong-manifest invocation is also retained.
Build/test receipts and these failures are in [validation.json](validation.json).

Replay into a required fresh destination with a newly rebuilt runner:

```sh
python experiments/name-handling/results/clock-parse-format-args-runtime/capture.py \
  --output-dir /tmp/clock-parse-format-args-runtime-NEW \
  --runner /absolute/path/to/fresh/run_script
```
