# jim namespace upvar root worker forwarding

ID: `naming.variable.jim-namespace-upvar-root-worker-forwarding`

## Problem statement

Jim namespace upvar is a helper whose selected root upvar worker can be replaced. A namespaced shadow, the helper's empty padded local name and the variable frame are separate owners; literal command spelling cannot establish worker continuity.

## Question

Does the captured Jim namespace-upvar helper ignore a namespaced upvar shadow, retain the caller variable frame and forward to a replaced root worker?

## Answers

### tcl8.4 — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### tcl8.5 — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### tcl8.6 — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### tcl9.0 — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### tcl9.1 — not-tested

No observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

### jim — observed

0 {VALUE {FORWARD 0 ::missing::a {}}}

Version: Jim label; exact patch/revision not recorded. Build: Binary path /tmp/2286-oracles/jimtcl/jimsh; SHA not recorded. Input channel: Exact ASCII jim-forwarding.tcl via native shell.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.

## Conclusion

The measured Jim helper links the padded empty local name in namespace n so the target value is VALUE, ignores the namespaced LOCAL shadow, and later forwards through replaced root upvar as FORWARD 0 ::missing::a {}. This helper-specific original argument/frame result grants no stock C behaviour or lasting worker identity.

## Scope

One exact ASCII Jim script and captured result. Current-label Jim binary path is recorded but patch/revision/header/binary hash are not. No C or BIG-IP engine was queried for this forwarding question.

## Exact retained evidence

- `rust/tcl-registry/tests/data/native_namespace_upvar_arguments/jim-forwarding-manifest.json`: Exact original native provenance/captured observations.
- `rust/tcl-registry/tests/data/native_namespace_upvar_arguments/jim-forwarding.tcl`: Retained exact script/case bytes for the stated question.

## Rust comparison

`cmd_namespace::native_upvar_tests::jim_namespace_upvar_forwards_root_worker_without_changing_variable_frame` in `runtime/rust/src/cmd_namespace/native_upvar_tests.rs`. No Rust execution result is recorded here.
