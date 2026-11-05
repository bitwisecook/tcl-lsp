# naming.tcloo.private-definition-mode

Kind: `native-observation`

## Problem statement

A private modifier with no arguments may be a query or an arity error, and nested modifiers must restore the prior definition mode. Using an empty result or sticky depth changes surrounding declarations. These controls apply to actual admitted C9 definition frames; they do not attest arbitrary source or object caches.

## Question

Does the actual private definition worker report and restore its mode before, inside and after a private block?

## Conclusion

C9.0/C9.1 returns 0 1 0 before, inside and after the block. The worker is absent in the measured C8.4-C8.6 and Jim installations. This is an actual definition-frame mode observation; no editable source, method-header or compiler capability follows.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `unsupported`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

PRIVATE_QUERY=0 1 0.

### jim

Status: `unsupported`. Channel: ASCII NUL-terminated source passed to Jim_Eval. Version: jim 0.84-9-g5bac7c9.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### bigip

Status: `not-tested`.

No appliance observation for this exact question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `cmd_private`: Query and restore the actual private definition mode independently of body byte evaluation.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_private`: Report the authenticated current private definition mode.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare captured query and private opaque method invocation results on the independently selected C9.0/C9.1 VM cores. This test does not assert complete private-roster parity.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare mode query, opaque original body definitions, true-private roster and matching my invocation results on independently selected C9.0/C9.1 runtime cores.

## Evidence

- [rust/tcl-vm/tests/data/native_private_legacy_trace/v2](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2)
- [rust/tcl-vm/tests/data/native_private_legacy_trace/v3](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3)
