# naming.tcloo.private-original-script-bytes

Kind: `native-observation`

## Problem statement

Converting a counted private body into a display string can change raw ff, an opaque surrogate or a binary zero in a method name. A valid Unicode display is insufficient evidence for native script bytes. The exact private method and my wrapper controls distinguish byte preservation from replacement or clipping on C9.

## Question

Can a counted original private body declare and invoke methods whose names contain raw ff, native surrogate bytes or binary zero?

## Conclusion

C9.0/C9.1 accepts all three counted original bodies and their matching my calls return OK. The captured -private roster reports only the public invoke wrapper, keeping true private declaration distinct from ordinary unexporting. Earlier C versions and Jim report the worker absent. The measured result and roster do not grant native compiler, cache, header or editable geometry authority.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `unsupported`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

All three PRIVATE_RAW_* definitions and matching private my invocations succeed; PRIVATE_RAW_*_NAMES contains invoke only.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated source passed to Jim_Eval. Version: jim 0.84-9-g5bac7c9.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### bigip

Status: `not-tested`.

No appliance observation for this exact question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `cmd_private`: Enter the original script object through the selected body evaluator; retain byte-based inline worker lookup.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_private`: Evaluate the original control-body object instead of a detached byte or display copy.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare captured query and private opaque method invocation results on the independently selected C9.0/C9.1 VM cores. This test does not assert complete private-roster parity.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare mode query, opaque original body definitions, true-private roster and matching my invocation results on independently selected C9.0/C9.1 runtime cores.

## Evidence

- [rust/tcl-vm/tests/data/native_private_legacy_trace/v2](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2)
- [rust/tcl-vm/tests/data/native_private_legacy_trace/v3](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3)
