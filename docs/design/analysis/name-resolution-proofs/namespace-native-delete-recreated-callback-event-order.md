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

Status: `observed`. Version: 8.4.20 (selected configured build; runtime patchlevel unqueried). Build: Original static library SHA256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47; public header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; configured Makefile SHA256 0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1; observer executable SHA256 c417911e76501f0841c615c7a70c43d2930410e9856e2c6c20d6d57174834b29; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (selected configured build; runtime patchlevel unqueried). Build: Original static library SHA256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; public header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; configured Makefile SHA256 26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5; observer executable SHA256 b3ebfc4c43b3f7bd5a97bee9f2bdf233216f6e27a435d23275b419a85064b0d9; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (selected configured build; runtime patchlevel unqueried). Build: Original static library SHA256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; public header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; configured Makefile SHA256 8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e; observer executable SHA256 75e6b0c089653bda587fc14b6d14e4ba51dc5eca2213d03ffe91350743f28862; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (selected configured build; runtime patchlevel unqueried). Build: Original static library SHA256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; public header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; configured Makefile SHA256 69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5; observer executable SHA256 5b27e2f224fbebb18601e8ce6915892d0c61b7e6e19887abbfe61c9f68c8e5ce; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (selected configured build; runtime patchlevel unqueried). Build: Original static library SHA256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; public header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; configured Makefile SHA256 c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1; observer executable SHA256 f4952b203f2a9d17c756c81f8905298403d24ec19b00bc12864198adff53b7ac; configured flags/private-header hashes retained. Compile/process0; stderr intentionally contains the10 measured events, not an empty-stream claim.. Channel: Four original counted Tcl_EvalObjv flags0 operands; native command DeleteProc calls native namespace deletion/creation; private nsId/lookup event observations.. Dialect: C Tcl.

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

## Exact evidence

- `input` (input): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/probe.c). SHA-256 `f0ed74051da352cc6aff27eb126da8505c5b92c2a1ef4447c82bae3cf2ba8b64`. Exact original setup/argv plus independent ordered callback and nsId observer.
- `replayer` (input): [scripts/dev/replay-namespace-delete-callback-events.py](../../../../scripts/dev/replay-namespace-delete-callback-events.py). SHA-256 `d5233c9c2fe4940af2dcbe76dfd6962b6972b83dcc2c617964c09b84a39dc12f`. Maintained explicit-provider replay requiring the exact public/private-header, Makefile and archive hashes.
- `receipt-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/receipt.json). SHA-256 `9577d1f689e26be0afc7d47423d7cfa6ba4f8166f85f7dda734a94decfa36082`. Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stdout.tsv). SHA-256 `3b4ce2a19b41b5f1d4da0e0e009519980afb5cddbb34a9c66c6e0095b10c3fbf`. Exact original result-shaped final fields.
- `events-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/stderr). SHA-256 `a08e58d18e3485270c99ea0239d5cfed7b2828cc1d4287e94ed394e3ca384c4e`. Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stdout.
- `compile-stderr-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stderr.
- `receipt-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/receipt.json). SHA-256 `ebb748fe9d6d505a5131f7ca9f1e5ce8b17941a6ea03c1779e83c3ef995e78f1`. Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stdout.tsv). SHA-256 `3b4ce2a19b41b5f1d4da0e0e009519980afb5cddbb34a9c66c6e0095b10c3fbf`. Exact original result-shaped final fields.
- `events-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/stderr). SHA-256 `a08e58d18e3485270c99ea0239d5cfed7b2828cc1d4287e94ed394e3ca384c4e`. Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stdout.
- `compile-stderr-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stderr.
- `receipt-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/receipt.json). SHA-256 `fecefdb1205593d99629336a847439f347de212944e25942671c2598a4b5ca03`. Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stdout.tsv). SHA-256 `3b4ce2a19b41b5f1d4da0e0e009519980afb5cddbb34a9c66c6e0095b10c3fbf`. Exact original result-shaped final fields.
- `events-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/stderr). SHA-256 `a08e58d18e3485270c99ea0239d5cfed7b2828cc1d4287e94ed394e3ca384c4e`. Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stdout.
- `compile-stderr-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stderr.
- `receipt-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/receipt.json). SHA-256 `cb2934bd216a698d2e2347d3f0ce972872f500e39b35a353480507f444a4d563`. Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stdout.tsv). SHA-256 `3b4ce2a19b41b5f1d4da0e0e009519980afb5cddbb34a9c66c6e0095b10c3fbf`. Exact original result-shaped final fields.
- `events-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/stderr). SHA-256 `a08e58d18e3485270c99ea0239d5cfed7b2828cc1d4287e94ed394e3ca384c4e`. Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stdout.
- `compile-stderr-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stderr.
- `receipt-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/receipt.json). SHA-256 `54ae0a28c19f850b80aa6ea0028d273c88de85538cf62571be1a2153c9147256`. Exact configured flags/public-private-header/Makefile/archive/observer/source/stream/compile-process association.
- `stdout-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stdout.tsv](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stdout.tsv). SHA-256 `3b4ce2a19b41b5f1d4da0e0e009519980afb5cddbb34a9c66c6e0095b10c3fbf`. Exact original result-shaped final fields.
- `events-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/stderr). SHA-256 `a08e58d18e3485270c99ea0239d5cfed7b2828cc1d4287e94ed394e3ca384c4e`. Complete original ten ordered instrumentation events, including callback kinds and nsId equality booleans.
- `compile-stdout-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stdout.
- `compile-stderr-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_delete_callback_events/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compile stderr.

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
