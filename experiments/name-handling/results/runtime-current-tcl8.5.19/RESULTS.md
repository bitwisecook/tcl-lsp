# Actual native runtime: Tcl 8.5.19 upstream naming suites

This capture runs the original `namespace`, `var`, `upvar`, `rename`, `set` and
`proc` files through matching real 8.5.19 `init.tcl` and Tcltest 2.3.8. No suite
body, expected result or interpreter substitute is injected. The shared
backend overlay retains upstream constraints and excludes only its documented
platform/capability boundaries.

The immutable runtime is `/tmp/name-runtime-native-ensemble-fixed`, SHA-256
`eaf091d49143d47282b825ffff53012b799b72acaec1780b6326207728bb180f`.
It uses `Interp::with_native_core` before command allocation and transports
initialization through `init_library_completion`. The
[runtime build receipt](runtime-build-receipt.json) identifies its source
commit and source patch hash; the corresponding
[source patch](../runtime-lifecycle-fix/native-ensemble-fix/source.patch) retains
uncommitted build changes. This executable is distinct from subsequent working
source. [Provenance](provenance.json) retains both executable hashes, exact
upstream/init/Tcltest/overlay/runner hashes, selected library and 90-second
per-file limit. Two workers run C/runtime stdin configuration followed by the
original file in fresh temporary directories.

## Results

Completed cells are passed / skipped / failed. Host refusals have no complete
suite totals and supply no result for remaining tests.

| File | C 8.5.19 | Native runtime 8.5 |
| --- | --- | --- |
| namespace | 280 / 1 / 0 | Host refusal at `namespace-8.1`: original local alias table receipt policy; no assertion failures printed |
| var | 69 / 17 / 0 | 66 / 17 / 3 |
| upvar | 52 / 7 / 0 | Host refusal at `upvar-1.1`: original simple variable cell policy; no assertion failures printed |
| rename | 11 / 8 / 0 | 11 / 8 / 0 |
| set | 63 / 1 / 0 | 63 / 1 / 0 |
| proc | 25 / 8 / 0 | 25 / 8 / 0 |

C completes all six files: 500 passes, 42 skips, 0 failures. Runtime completes
four files: 165 passes, 34 skips, 3 failures. Incomplete files are excluded from
these runtime totals. All 12 processes enter real Tcltest and emit the matching
library/loader record. C and runtime both report `NH_FRAME_DEBUG enabled`.
There are no C frame/source-depth failures in this selection; no control
assertion is discounted as a harness artifact. Tests requiring absent C test
commands remain upstream-constrained skips, not conformance passes.

## Remaining measured gaps

- `var-7.16` and `var-7.17`: zero-argument `variable` returns success and an
  empty value; C returns the documented wrong-argument error, both globally
  and inside a namespace.
- `var-14.2`: `array names tcl_platform -glob os` returns a wrong-argument
  error; C accepts the `-glob` mode.
- `namespace` cannot complete because native local-alias table ownership
  policy is unavailable at the observed invocation window. `namespace-8.1`
  is the last started test, not an emitted failed assertion or evidence about
  later test bodies.
- `upvar` cannot complete because native simple-variable-cell policy is
  unavailable. The stop at `upvar-1.1` leaves subsequent cases unmeasured.

All three failed assertion IDs execute and pass in the same C capture.
They are three observed assertion failures, not a count of independent bugs.
[Machine records](records.json), [classification](classification.json), wrapper
inputs and complete per-file stdout/stderr retain the distinction between
assertion failures, host refusals and unexecuted tests.

The exact upstream Tcl9.0.4 `namespace-56.2` body separately passes with this
same runtime and real 8.5 Tcltest in
[the lifecycle capture](../runtime-lifecycle-fix/native-ensemble-fix/8.5.19/upstream-namespace-56.2.stdout).
That isolated pass does not establish completion of this older 8.5 namespace
suite. The independently measured 8.5 cached namespace reference difference
after real library initialization remains open; see
[the lifecycle report](../runtime-lifecycle-fix/RESULTS.md).
