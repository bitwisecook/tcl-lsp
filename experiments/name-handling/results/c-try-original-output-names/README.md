Original C try handler variable objects
======================================

Question: does try preserve the actual handler variable-list member objects,
including empty names, failed stores, fall-through clauses and malformed lists?

Capture01 uses the original repository Tcl 8.6.18, 9.0.4 and 9.1.0 headers and
static libraries. The retained pre-hashed driver compiled and executed the exact
C API input once against each provider. All six stages exited 0 with no stderr;
selected source, library and configuration hashes were unchanged. Header and
live interpreter patchlevels agree. The manifest retains compiler, executable,
linked-library hashes, complete invocations, timing and output channels.

The probe uses Tcl_CreateInterp and Tcl_EvalObjv with TCL_EVAL_DIRECT. Tcl_Init,
init.tcl and tcltest are not executed; this is a core-only C API observation,
with no substituted or claimed suite verdict. Rebuildable executables are
excluded; their captured hashes are checked when a local executable is present.

All three releases assign the empty first name. Successful selected original
member headers become parsedVarName objects. An existing-array result store
fails before the options-name header is read or the handler body runs. A
fall-through clause leaves its unused original member headers stringless int
objects, while the running clause's members acquire variable-name caches.
A malformed variable list is rejected before the body executes.

Fixtures are exact primary stdout. They measure completion, body/handler reach
and original member String/type effects. Native reference counts, full error
diagnostics, member mutation during callbacks, compiled try and complete return
option parity remain outside this capture. Rust controls remain unexecuted.
