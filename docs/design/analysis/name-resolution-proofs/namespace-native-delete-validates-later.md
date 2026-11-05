# naming.namespace-native.delete-validates-later

Kind: `native-observation`

## Problem statement

A valid first namespace followed by a missing namespace can either be deleted immediately or remain until every original operand validates. These choices produce different observable tables even though the command eventually fails.

## Question

Does namespace delete ::A ::missing retire A or its child before reporting the later missing operand?

## Conclusion

Every C capture returns code1 naming ::missing and preserves A, child and B, with zero command-delete callback counts. This exact later-validation failure precedes namespace retirement in the sampled controls.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. The retained JSONL contains 40 complete original rows. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.4 8.4.20 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.5 8.5.19 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl8.6 8.6.18 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.0 9.0.4 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations. Version: tcl9.1 9.1.0 (capture association; full runtime patchlevel unqueried).

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### jim, bigip

Status: `not-tested`.

No original two-pass deletion/callback capture for this provider is attached.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `delete_original`: Shared original namespace validation and ordered later-target lookup; actual backend callback/lifetime facts remain independently owned.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks` (linked): Exercises missing-later preservation and dependent duplicate retirement; no callback recreation presence claim borrowed.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::native_deletion_validates_all_names_then_relooks_dependent_targets` (linked): Checks the exact later-validation and dependent-duplicate table obligations, not a measured native execution.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout)
