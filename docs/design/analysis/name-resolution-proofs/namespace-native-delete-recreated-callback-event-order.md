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

### tcl8.4

Status: `observed`. Version: 8.4.20 (selected configured build; runtime patchlevel unqueried). Build: Original static library; public header; configured Makefile; observer executable; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

### tcl8.5

Status: `observed`. Version: 8.5.19 (selected configured build; runtime patchlevel unqueried). Build: Original static library; public header; configured Makefile; observer executable; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18 (selected configured build; runtime patchlevel unqueried). Build: Original static library; public header; configured Makefile; observer executable; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4 (selected configured build; runtime patchlevel unqueried). Build: Original static library; public header; configured Makefile; observer executable; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

### tcl9.1

Status: `observed`. Version: 9.1.0 (selected configured build; runtime patchlevel unqueried). Build: Original static library; public header; configured Makefile; observer executable; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No execution of this exact instrumented C namespace callback program for this provider is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this exact instrumented C namespace callback program for this provider is attached.

## Historical references

- `input` (input): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c). Exact original setup/argv plus independent ordered callback and nsId observer.
- `replayer` (input): [scripts/dev/replay-namespace-delete-callback-events.py](../../../../scripts/dev/replay-namespace-delete-callback-events.py). Maintained explicit-provider replay requiring the exact public/private-header, Makefile and archive hashes.
- `receipt-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/receipt.json). Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stdout.tsv). Exact original result-shaped final fields.
- `events-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stderr). Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stdout). Exact original compile stdout.
- `compile-stderr-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stderr). Exact original compile stderr.
- `receipt-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/receipt.json). Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stdout.tsv). Exact original result-shaped final fields.
- `events-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stderr). Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stdout). Exact original compile stdout.
- `compile-stderr-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stderr). Exact original compile stderr.
- `receipt-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/receipt.json). Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stdout.tsv). Exact original result-shaped final fields.
- `events-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stderr). Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stdout). Exact original compile stdout.
- `compile-stderr-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stderr). Exact original compile stderr.
- `receipt-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/receipt.json). Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stdout.tsv). Exact original result-shaped final fields.
- `events-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stderr). Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stdout). Exact original compile stdout.
- `compile-stderr-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stderr). Exact original compile stderr.
- `receipt-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/receipt.json). Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stdout.tsv). Exact original result-shaped final fields.
- `events-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stderr). Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stdout). Exact original compile stdout.
- `compile-stderr-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stderr). Exact original compile stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-namespace-delete-callback-events.py",
  "--provider",
  "8.6.18",
  "--tcl-root",
  "<selected-configured-tcl-source-build-root>",
  "--output",
  "<new-output-directory>"
]
```

Replay requires an explicitly selected build whose tcl.h, configured Makefile, private headers and static library match the recorded whole-file digests. The source/flags and complete stdout/stderr/process comparison are preserved. Compiler version/full linked runtime patchlevel were not queried by the original capture; no build identity is borrowed from another receipt with different library bytes. --verify-only checks retained source/stream/event associations and launches no native process. A new replay retains its own compile/launch/stream receipts.
