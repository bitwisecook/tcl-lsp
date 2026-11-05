# naming.namespace-native.delete-recreated-callback-event-order

Kind: `native-observation`

## Problem statement

An outer namespace delete validates B before a command DeleteProc triggered by retiring A deletes that B and creates a new B with its own namespace DeleteProc. Final absence and two callback counts cannot establish which incarnation was visible at each event. A Rust command-retirement surrogate also does not measure a native namespace DeleteProc.

## Question

Which ordered namespace presence/incarnation and command-versus-namespace DeleteProc events occur when the original command retirement callback deletes and recreates B during namespace delete A B?

## Conclusion

All five configured C captures complete with code0, empty result and A/child/B absent, hook1, oldBdeleted1, newBdeleted1. Ordered events show the original B present at command DeleteProc entry, old namespace DeleteProc with B absent, a nonnull creation with only the new nsId present, then new namespace DeleteProc with B absent before outer evaluation returns. The instrumentation distinguishes actual callback kinds and selected native nsId equality; it proves no Rust callback-ownership or event-order parity.

## Scope

One exact original scenario-3 setup per selected configured C build, including its unmeasured warm-up command deletion. The four original Tcl_EvalObjv flags0 operands and final JSON fields are preserved. Added stderr event observations perform Tcl_FindNamespace and compare saved private nsId values as booleans; no IDs or addresses are printed. These reentrant observer reads are part of this separate instrumented program, not an assumption of non-interference with arbitrary namespace deletion. Interpreter cleanup after the final result is excluded. Jim/BIG-IP are not tested. Release labels identify selected build associations, not a queried runtime patchlevel.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations. Version: tcl8.4 8.4.20 (selected configured build; runtime patchlevel unqueried); tcl8.5 8.5.19 (selected configured build; runtime patchlevel unqueried); tcl8.6 8.6.18 (selected configured build; runtime patchlevel unqueried); tcl9.0 9.0.4 (selected configured build; runtime patchlevel unqueried); tcl9.1 9.1.0 (selected configured build; runtime patchlevel unqueried).

Final original JSON result fields:

```json
{"case":"callback-recreates-later","code":0,"result":"","A":0,"child":0,"B":0,"hook":1,"oldBdeleted":1,"newBdeleted":1}
```

Ordered original event fields:

```text
ordinal stage callbackKind B oldIncarnation newIncarnation createReturnedNonNull hook oldBdeleted newBdeleted
1 before-eval none 1 1 0 -1 0 0 0
2 retire-entry command-DeleteProc 1 1 0 -1 1 0 0
3 retire-before-delete-B command-DeleteProc 1 1 0 -1 1 0 0
4 old-b-delete-proc namespace-DeleteProc 0 0 0 -1 1 1 0
5 retire-after-delete-B command-DeleteProc 0 0 0 -1 1 1 0
6 retire-before-create-B command-DeleteProc 0 0 0 -1 1 1 0
7 retire-after-create-B command-DeleteProc 1 0 1 1 1 1 0
8 new-b-delete-proc namespace-DeleteProc 0 0 0 -1 1 1 1
9 after-eval none 0 0 0 -1 1 1 1
10 after-result-publication none 0 0 0 -1 1 1 1
```
The callbackKind field identifies retire as a command DeleteProc and old_b/new_b as namespace DeleteProc callbacks.

### jim, bigip

Status: `not-tested`.

No execution of this exact instrumented C namespace callback program for this provider is attached.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c)
- [scripts/dev/replay-namespace-delete-callback-events.py](../../../../scripts/dev/replay-namespace-delete-callback-events.py)
- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20)
- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19)
- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18)
- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4)
- [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0)
