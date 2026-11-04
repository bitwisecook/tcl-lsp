# naming.namespace-native.delete-dependent-child-duplicate

Kind: `native-observation`

## Problem statement

The deletion operands contain a parent, its child and a repeated parent. Retaining initial table pointers or blindly resolving a missing child after parent retirement can create a spurious failure.

## Question

What completion and tables follow original namespace delete ::A ::A::q ::A?

## Conclusion

All five C releases return code0 with empty result; A and child are absent, B remains and callback counters stay0. This finite dependency/duplicate case does not measure arbitrary callback reentrancy.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. All40 complete original JSONL rows are recovered by agreement with recorded original log SHA256. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|dependent-child-duplicate|0||0|0|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|dependent-child-duplicate|0||0|0|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|dependent-child-duplicate|0||0|0|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|dependent-child-duplicate|0||0|0|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; full runtime patchlevel unqueried). Build: Header; private-header; library; observer; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|dependent-child-duplicate|0||0|0|1|0|0|0|-
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
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks` (linked): Exercises missing-later preservation and dependent duplicate retirement; no callback recreation presence claim borrowed.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::native_deletion_validates_all_names_then_relooks_dependent_targets` (linked): Checks the exact later-validation and dependent-duplicate table obligations, not a measured native execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source and complete original streams are attached with matching original whole-log digests. A reconfirmation must compile unchanged against identified headers/internal headers/static libraries and preserve flags0, source/setup and actual callback kind, then retain separate stdout/stderr/process receipts. The native command DeleteProc retire calls namespace deletion/creation with namespace DeleteProc counters. A Rust command-retirement surrogate does not measure native namespace DeleteProc ownership or event timing.
