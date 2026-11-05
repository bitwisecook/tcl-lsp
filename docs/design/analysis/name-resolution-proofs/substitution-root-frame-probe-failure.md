# naming.substitution.root-frame-probe-failure

Kind: `native-observation`

## Problem statement

A private-header probe can crash when it assumes a non-null root varFrame. Calling that crash a Tcl semantic error or unavailable naming capability would confuse harness validity with guest behavior. The original failed process and the separately pinned finite null-frame controls must remain independently recorded.

## Question

Did the v4 private root-frame probe execute, and did explicit null-frame handling in v5/v6 restore this harness?

## Conclusion

v4 C8.4 compiled successfully but exited with SIGSEGV (-11) and emitted zero rows; the other four C providers emitted their retained rows with exit0. The probe dereferenced the null root varFrame. v5/v6 retain an explicit null-frame boolean and all five providers compile/process exit0. The failed C8.4 v4 attempt answers process availability only, not any guest substitution or naming question.

## Scope

Exact configured v4 process receipts and raw streams, compared with the separately pinned v5/v6 probes. Signal11 and zero rows are preserved. No Rust result, engine-level guest failure, or availability of an unexecuted operation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame. Version: tcl8.4 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v4 SIGSEGV -11 and zero rows; v5/v6 compile/process0 with explicit null-frame handling.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Configured native probe process and private varFrame inspection; v4 C8.4 terminates before any guest result row, while v5/v6 explicitly handle a null root frame. Version: tcl8.5 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.6 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.0 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.1 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

v4/v5/v6 compile/process0 and retain actual frame/cache booleans.

### jim, bigip

Status: `not-tested`.

This exact C substitution/private-header question was not tested on this provider.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_substitution_owner](../../../../rust/tcl-registry/tests/data/native_substitution_owner)
- [rust/tcl-registry/tests/data/native_substitution_owner/v1](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1)
- [rust/tcl-registry/tests/data/native_substitution_owner/v2](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2)
- [rust/tcl-registry/tests/data/native_substitution_owner/v3](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3)
- [rust/tcl-registry/tests/data/native_substitution_owner/v4](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4)
- [rust/tcl-registry/tests/data/native_substitution_owner/v5](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5)
- [rust/tcl-registry/tests/data/native_substitution_owner/v6](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6)
