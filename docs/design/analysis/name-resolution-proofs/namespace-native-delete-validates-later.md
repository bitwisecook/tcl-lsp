# naming.namespace-native.delete-validates-later

Kind: `native-observation`

## Problem statement

A valid first namespace followed by a missing namespace can either be deleted immediately or remain until every original operand validates. These choices produce different observable tables even though the command eventually fails.

## Question

Does namespace delete ::A ::missing retire A or its child before reporting the later missing operand?

## Conclusion

Every C capture returns code1 naming ::missing and preserves A, child and B, with zero command-delete callback counts. This exact later-validation failure precedes namespace retirement in the sampled controls.

## Scope

Eight original C object-vector deletion controls per release, flags0, each in a fresh interpreter. Exact guest code/result are printed before namespace-existence and callback-count observers. Setup uses native namespaces/command delete callbacks or release-selected ASCII unset trace syntax. Original raw-zero String and pure ByteArray inputs are independently manufactured. All40 complete original JSONL rows are recovered by agreement with recorded original log SHA256. No return-options, namespace-name cache, allocator/reference-count or Jim/BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; full runtime patchlevel unqueried). Build: Header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; private-header SHA256 f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a; library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; observer SHA256 b6d6d0be27c3c8f673e488a4d7c26b4f1359d334285e2126055550f8e2b544b2; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.4.20|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; full runtime patchlevel unqueried). Build: Header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; private-header SHA256 72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa; library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; observer SHA256 248ce14ac0d86c23bbb7ad6f1ada5592410b5d47c4b01b82cda043451505bf6d; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.5.19|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; full runtime patchlevel unqueried). Build: Header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; private-header SHA256 e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e; library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; observer SHA256 4846caf0a95f8cc91ca7500885fa21ad0dc5360f1f94e1912fd89912df86f16d; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
8.6.18|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; full runtime patchlevel unqueried). Build: Header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; private-header SHA256 f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053; library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; observer SHA256 50610157fcc5c32892ea68cd6acc55806d6e549b847dec009e511c79c632d1cf; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.0.4|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; full runtime patchlevel unqueried). Build: Header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; private-header SHA256 fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; observer SHA256 b807ec8f32e09c3bc3de915758428e1dbc7bfea96cdfda508f59e94b657c8822; compile/process0, recorded empty stderr. Compiler/configure details unrecorded.. Channel: Original Tcl_EvalObjv flags0 with native command-delete callbacks or ASCII Tcl_Eval unset-trace setup; subsequent actual namespace/count observations.. Dialect: C Tcl.

Selected release/case/code/result-hex/A/child/B/hook/old-B/new-B/trace-calls projections:

```text
9.1.0|invalid-later|1|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d6573706163652064656c65746520636f6d6d616e64|1|1|1|0|0|0|-
```
The complete original JSONL stream matches its recorded whole-log digest. Guest result precedes subsequent namespace/count observations; no unobserved return-option/cache/reference state is inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original two-pass deletion/callback capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original two-pass deletion/callback capture for this provider is attached.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.c). SHA-256 `3e762a04dfa3761958d189818c3e3640076d72ccda107cc43bfeb5bf6966c1cc`. Exact original argv, ordered callbacks, counted String/ByteArray construction and reached existence/count observers.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. Original selected build/launch/status/log associations.
- `projection` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass.txt). SHA-256 `12738bb994e1f441d554ce7bf6d68000fcc0b90565dfe8995e7d3f352428baa6`. Exact 40 original field projections; B is column 6 after the empty result field.
- `stream-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.4.20.stdout). SHA-256 `621dbb847f385d1da84200db5023f644645316c81ad8ba1ac27a6a1e1f51252e`. Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. JSON pointer `/runs/0`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.5.19.stdout). SHA-256 `621dbb847f385d1da84200db5023f644645316c81ad8ba1ac27a6a1e1f51252e`. Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. JSON pointer `/runs/1`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-8.6.18.stdout). SHA-256 `621dbb847f385d1da84200db5023f644645316c81ad8ba1ac27a6a1e1f51252e`. Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. JSON pointer `/runs/2`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.0.4.stdout). SHA-256 `621dbb847f385d1da84200db5023f644645316c81ad8ba1ac27a6a1e1f51252e`. Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. JSON pointer `/runs/3`. Exact original header/internal-header/library/observer/build/process association.
- `stream-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/delete-9.1.0.stdout). SHA-256 `621dbb847f385d1da84200db5023f644645316c81ad8ba1ac27a6a1e1f51252e`. Complete original JSONL stdout matching the recorded original log digest.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/delete_two_pass_provenance.json). SHA-256 `1d6898b9c79f5e22b570ea7e08c992b24e393513c1d71fdc08a09daed9d09f9e`. JSON pointer `/runs/4`. Exact original header/internal-header/library/observer/build/process association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `delete_original`: Shared original namespace validation and ordered later-target lookup; actual backend callback/lifetime facts remain independently owned.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks` (linked): Exercises missing-later preservation and dependent duplicate retirement; no callback recreation presence claim borrowed.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::native_deletion_validates_all_names_then_relooks_dependent_targets` (linked): Checks the exact later-validation and dependent-duplicate table obligations, not a measured native execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source and complete original streams are attached with matching original whole-log digests. A reconfirmation must compile unchanged against identified headers/internal headers/static libraries and preserve flags0, source/setup and actual callback kind, then retain separate stdout/stderr/process receipts. The native command DeleteProc retire calls namespace deletion/creation with namespace DeleteProc counters. A Rust command-retirement surrogate does not measure native namespace DeleteProc ownership or event timing.
