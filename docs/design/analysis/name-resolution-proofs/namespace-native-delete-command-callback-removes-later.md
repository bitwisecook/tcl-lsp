# naming.namespace-native.delete-command-callback-removes-later

Kind: `native-observation`

## Problem statement

A command-delete callback triggered while retiring A deletes the later B operand. The native second pass must distinguish that changed table from its initial validation state.

## Question

What later B table and callback counts remain when A's command-delete callback deletes B during namespace delete ::A ::B?

## Conclusion

Every C release returns code0 and empty result; A, child and B are absent, hook count1 and oldBdeleted1 with newBdeleted0. The exact callback removal outcome is separate from unset-trace effects or a recreated namespace.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. The retained JSONL contains 40 complete original rows. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.4 8.4.20 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|callback-deletes-later|0||0|0|0|1|1|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.5 8.5.19 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|callback-deletes-later|0||0|0|0|1|1|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.6 8.6.18 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|callback-deletes-later|0||0|0|0|1|1|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.0 9.0.4 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|callback-deletes-later|0||0|0|0|1|1|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.1 9.1.0 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|callback-deletes-later|0||0|0|0|1|1|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### jim, bigip

Status: `not-tested`.

No original two-pass deletion/callback capture for this provider is attached.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `delete_original`: Shared original namespace validation and ordered later-target lookup; actual backend callback/lifetime facts remain independently owned.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::deletion_second_pass_observes_callback_deleted_and_recreated_incarnations` (linked): Checks later-target removal and recreation result fields. Its Rust command-retirement surrogate has a different callback kind from the native namespace DeleteProc counters; no callback-ownership or timing parity is claimed.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout)
