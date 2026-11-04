# naming.namespace-native.delete-command-callback-recreates-later

Kind: `native-observation`

## Problem statement

A command DeleteProc triggered while retiring A deletes B and creates a fresh B while an outer namespace deletion still owns its operand list. Deleting the initially validated incarnation or resolving the new incarnation on the second pass would leave different tables and namespace DeleteProc counters.

## Question

Which B existence and old/new deletion counters remain when the command-delete callback recreates B during namespace delete ::A ::B?

## Conclusion

All five exact original streams report code0 with empty result, A/child/B absent, hook1, oldBdeleted1 and newBdeleted1. Both original and recreated B namespace DeleteProc counters are reached. These result fields agree with B absence in the separate unset-trace recreation control, but do not establish the same callback kind, ownership or event order.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. The retained JSONL contains 40 complete original rows. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|callback-recreates-later|0||0|0|0|1|1|1|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|callback-recreates-later|0||0|0|0|1|1|1|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|callback-recreates-later|0||0|0|0|1|1|1|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|callback-recreates-later|0||0|0|0|1|1|1|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|callback-recreates-later|0||0|0|0|1|1|1|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original two-pass deletion/callback capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original two-pass deletion/callback capture for this provider is attached.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c). Exact original argv, ordered callbacks, counted String/ByteArray construction and reached existence/count observers.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). Original selected build/launch/status/log associations.
- `projection` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt). Exact 40 original field projections; B is column 6 after the empty result field.
- `stream-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout). Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). JSON pointer `/runs/0`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout). Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). JSON pointer `/runs/1`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout). Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). JSON pointer `/runs/2`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout). Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). JSON pointer `/runs/3`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout). Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). JSON pointer `/runs/4`. Exact original header/internal-header/library/observer/build/process association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `delete_original`: Shared original namespace validation and ordered later-target lookup; actual backend callback/lifetime facts remain independently owned.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source and complete original streams are attached with matching original whole-log digests. A reconfirmation must compile unchanged against identified headers/internal headers/static libraries and preserve flags0, source/setup and actual callback kind, then retain separate stdout/stderr/process receipts. The native command DeleteProc retire calls namespace deletion/creation with namespace DeleteProc counters. A Rust command-retirement surrogate does not measure native namespace DeleteProc ownership or event timing.
