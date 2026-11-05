# naming.interpreter.child-command-probe-availability

Kind: `native-observation`

## Problem statement

C-shaped interp controls and C public child APIs cannot be assumed available in Jim simply because its probe process succeeds.

## Question

Which child interpreter operations were actually attempted by the six probes, and which guest/API operations were unavailable?

## Conclusion

Every probe compiled and exited0. All retained Jim C-shaped interp source controls returned error1/wrong # args: should be "interp"; the C public child-allocation API was explicitly not tested in Jim. All five C operations were actually attempted, including expected lookup and deleted-interpreter errors. These availability answers prove no native class/OO/compiler/cache capability or Rust parity.

## Scope

Exact counted ASCII source passed to Tcl_EvalEx or original Jim_NewStringObj/Jim_EvalObj on a fresh interpreter per source control. C public active-allocation control preserves Tcl_GetSlave actual interpreter through deletion. Finite cases: all actual v1/v2 startup and source/API availability rows. Result primary/resident fields precede the public getter; retained fields do not independently prove object release/callback closure. No Rust execution or whole lifecycle Normal is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Every probe compiled and exited0. All retained Jim C-shaped interp source controls returned error1/wrong # args: should be "interp"; the C public child-allocation API was explicitly not tested in Jim. All five C operations were actually attempted, including expected lookup and deleted-interpreter errors. These availability answers prove no native class/OO/compiler/cache capability or Rust parity.

### jim

Status: `observed`. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Version: jim 0.84-9-g5bac7c9.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`.

No appliance observation.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_child_command_lifetime/v1](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1)
- [runtime/rust/tests/data/native_child_command_lifetime/v2](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2)
- [runtime/rust/tests/data/native_child_command_lifetime/v1/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture)
- [runtime/rust/tests/data/native_child_command_lifetime/v2/capture](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture)
