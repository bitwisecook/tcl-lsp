# naming.namespace-native.delete-unset-trace-later-effects

Kind: `native-observation`

## Problem statement

Namespace retirement runs variable-unset traces. A trace can delete a later namespace or recreate it, and these callback effects differ from the independently captured command-delete callback recreation axis.

## Question

What completion, later B table and trace count remain when an unset trace deletes or recreates B during namespace delete ::A ::B?

## Conclusion

All five C releases return code0 and empty result for both trace controls; A and B are absent and calls1. The separate command DeleteProc recreation control also leaves B absent while sampling distinct old/new namespace DeleteProc counters. No callback silence, equal callback ownership or original object/cache/refcount invariant is inferred.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. The retained JSONL contains 40 complete original rows. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.4 8.4.20 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|trace-deletes-later|0||0|-|0|-|-|-|1
8.4.20|trace-recreates-later|0||0|-|0|-|-|-|1
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.5 8.5.19 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|trace-deletes-later|0||0|-|0|-|-|-|1
8.5.19|trace-recreates-later|0||0|-|0|-|-|-|1
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.6 8.6.18 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|trace-deletes-later|0||0|-|0|-|-|-|1
8.6.18|trace-recreates-later|0||0|-|0|-|-|-|1
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.0 9.0.4 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|trace-deletes-later|0||0|-|0|-|-|-|1
9.0.4|trace-recreates-later|0||0|-|0|-|-|-|1
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.1 9.1.0 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|trace-deletes-later|0||0|-|0|-|-|-|1
9.1.0|trace-recreates-later|0||0|-|0|-|-|-|1
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### jim, bigip

Status: `not-tested`.

No original two-pass deletion/callback capture for this provider is attached.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `delete_original`: Shared original namespace validation and ordered later-target lookup; actual backend callback/lifetime facts remain independently owned.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks` (linked): Checks both actual modeled unset-trace later-target controls, B absence and calls1; command-delete callback recreation is independent.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout)
