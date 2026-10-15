# Original Tcl 8.4 execution-environment error-name objects

Question: which actual name objects does Tcl 8.4 retain for error-info
seed/default/append operations, and do those objects survive individual
logging episodes and namespace/procedure calls?

`capture02` runs the original repository-pinned Tcl 8.4.20 provider
(`core-8-4-20`, commit `9ccfe9d1b35741ff7323837f6485ffe48b06fad9`). It observes
actual `Interp::execEnvPtr` and its real `errorInfo` and `errorCode` headers.
Before `Tcl_Init`, the root namespace and ExecEnv already exist. The two
headers are distinct, contain rooted `::errorInfo`/`::errorCode`, have resident
bytes and no internal type, and each has one native reference. Both exact
headers survive initialization, seed/default/append, another append, result
reset and a nested procedure error. Actual logging changes their primary to
`parsedVarName`. Reconstructing fresh names at each call loses that identity
and cache history.

An independently created interpreter has different headers for all four
cross-interpreter comparisons. Only after all operational observations does
the probe add an explicit native reference to each original parent header.
The actual interpreter destructor reduces each count from two to one; the
remaining observation roles keep the headers safe to inspect. The probe then
retires those roles. This demonstrates the resource's one retained role,
without claiming the reference schedule of an observer-free destructor.

The original source order is independently retained and freshly matched to
six immutable upstream files. `Tcl_CreateInterp` creates the result and empty
objects and root namespace before calling `TclCreateExecEnv`; that constructor
creates errorInfo then errorCode, retaining each once. `DeleteInterpProc`
removes namespaces and command/variable state before `TclDeleteExecEnv`
retires errorInfo then errorCode. This ordering comes from the actual source;
the probe does not manufacture retirement callbacks to infer it. In C8.4,
`Tcl_ObjSetVar2` reaches the original name getter and `TclObjLookupVar`, so
these held objects are a different purpose from the bare CString API used by
`Tcl_SetObjErrorCode`.

Every `ENV` row records phase, ExecEnv presence, root namespace presence,
info/code identity against the original headers, then each header's native
reference count, actual type name (`none` for NULL), resident-byte presence
and resident bytes in hex, followed by ExecEnv stackTop. No String getter is
used for these observations. `DISTINCT` records actual pointer inequalities;
`SCRIPT` records the raw nested-error completion; `DELETE` records returned
completion of the independent interpreter deletion. `AFTER_DELETE` records
the two explicit observation roles and the still-live actual headers.

The driver obtains exact definitions from the original `tclConfig.sh`, links
the actual original static library, and retains source/compiler/executable,
linked-library, invocation and stdout/stderr hashes. All 432 before/after
provider inputs are unchanged. The original library/test inventory is checked
at setup, including three support files outside the driver's suffix-based
before/after census (`library/tclIndex`, `tests/README`, `tests/httpd`). The
original `result.test` runs with that exact interpreter's original
`init.tcl`, `tcltest.tcl` and support files: **21 total, 1 passed, 20 skipped,
0 failed**. The skipped tests require original C test commands absent from
this ordinary tclsh. The compiled probe directly exercises the selected C API.

`capture01` is a failed setup only: its sandbox could not reach the network
proxy for immutable upstream checks, so compilation and execution were not
attempted. It supplies no semantic observations. `capture02` was run with
network access for those public immutable source checks; no provider files or
original harnesses were replaced.

Reproduce with a newly provisioned exact repository toolchain:

```
python3 capture.py --repo /absolute/repository --sdk-dir /absolute/original-tcl8.4.20 --out /new/capture-directory
```

`verify.py` checks retained artifacts and observations without compiling,
accessing the network or executing a provider. No Rust/VM tests were run.
These observations establish original object ownership and reuse; they do
not grant a new native object capability, settle all allocator/reference
stages, or establish a VM execution-environment implementation.
