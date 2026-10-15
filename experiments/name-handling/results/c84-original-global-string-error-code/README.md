# Original Tcl 8.4 CString global error-code publication

Question: does `Tcl_SetObjErrorCode` select the global cell through a bare C
string while preserving the current procedure frame and namespace, and when
are its result and error-code-set flag changed on a failed write?

`capture01` observes the original repository-pinned Tcl 8.4.20 provider
(`core-8-4-20`, commit `9ccfe9d1b35741ff7323837f6485ffe48b06fad9`). The
probe calls the actual public C API inside `N::p`, with independent local,
namespace and root `errorCode` cells. The actual supplied Int header has one
explicit caller reference throughout the API call. Observations do not add
references to the local, global or interpreter-result headers. Supplied type
and resident-byte presence are recorded before its final value observation.

Run through the repository toolchain owner after provisioning that exact SDK:

```
python3 capture.py --repo /absolute/repository --sdk-dir /absolute/original-tcl8.4.20 --out /new/capture-directory
```

The driver obtains compile definitions from that SDK's original
`unix/tclConfig.sh`, links its `libtcl8.4.a`, and records exact commands,
compiler, executable, linked-library and stream hashes. It checks the retained
original library/test inventory and freshly retrieves six original C sources
at the immutable upstream revision; every retrieved byte matches the SDK.
All 432 recorded provider inputs remain unchanged across execution. The
original `tests/result.test` runs with the exact original binary,
`library/init.tcl`, `library/tcltest/tcltest.tcl` and support files. Its result
is **21 total, 1 passed, 20 skipped, 0 failed**: the ordinary tclsh lacks the
original test-only `testsaveresult`, `testseterrorcode` and
`testsetobjerrorcode` commands. This is not a full 21-test pass. The separate
compiled probe directly exercises `Tcl_SetObjErrorCode`.

The six cases are: ordinary missing global; successful write trace; failing
write trace; existing array global; trace that replaces the written scalar;
and failing trace that first publishes a new interpreter result. All six
complete. The observed trace name is bare `errorCode`, part2 is NULL, and the
trace has GLOBAL_ONLY while retaining the actual caller frame and namespace.
The local and namespace cells survive unchanged. ERROR_CODE_SET is false
inside each trace and true after the API, including failed writes. A missing
global receives the exact supplied header without materialising its Int
string. Array collision leaves the array intact. Quiet failure preserves
MARKER unless the trace itself publishes TRACE_RESULT; that actual trace
result survives its reported failure. No unconditional result reset follows
quiet failure.

`OBS` fields after its tag are mode, trace count, flag before/after, same
caller frame, same namespace, same local header, global-is-supplied, supplied
type, supplied resident-byte presence, result hex and global hex (`-` for no
scalar). `TRACE` fields are mode, count, name1 hex, name2 hex or `-`,
GLOBAL_ONLY, same caller frame, same namespace and ERROR_CODE_SET. `CALL`
fields are mode, raw completion code, result hex and namespace-value hex.

The retained original source establishes separate purposes:

- `tclResult.c::Tcl_SetObjErrorCode` calls `Tcl_SetVar2Ex` with bare
  `errorCode`, GLOBAL_ONLY and no TCL_LEAVE_ERR_MSG, then sets ERROR_CODE_SET.
  `tclVar.c::Tcl_SetVar2Ex` uses the actual CString lookup/write kernel.
- `tclBasic.c::Tcl_AddObjErrorInfo` uses rooted name objects retained in the
  actual execution environment; `tclExecute.c` creates them at ExecEnv birth.
  Those name-object operations are not covered by a fresh CString issuer.

The VM's paired regression uses only the ordinary and array cases; its
existing five original script-trace controls remain independent. Raw C trace
callback evidence does not grant the VM an arbitrary C trace ABI or custom
object capability. Source review and these observations do not establish full
VM native reference-count, trace-lifecycle, signal or compiled-path parity.
The existing captured-store kernel's unused returned owned reference remains
a separate reference-stage limitation. Retained ExecEnv name-object transport
for seed/default/append remains unfinished.

`verify.py` checks retained hashes and observations only. It does not execute
an interpreter, compile, access the network or infer software conformance.
