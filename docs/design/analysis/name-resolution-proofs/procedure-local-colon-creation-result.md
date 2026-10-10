# naming.procedure.local-colon-creation-result

Kind: `native-observation`

## Problem statement

A single-colon :p procedure name in non-global ::ns is neither an ordinary p nor an explicitly rooted name. Older C releases reject its creation while later C and Jim accept it, and successful native return text differs from a pure validation result.

## Question

Does proc :p succeed inside namespace eval ::ns, and what exact caught completion/result does each provider return?

## Conclusion

C8.4 and C8.5 catch code1 with the exact non-global colon-leading procedure-creation error. C8.6, C9.0 and C9.1 catch code0 with an empty result; Jim catches code0 with result :p. This measures declaration creation acceptance and result only; the procedure body is not invoked.

## Scope

One exact ASCII source creates ::ns and performs local-colon, root-colon and ordinary-local procedure declarations under catch. It prints case/completion/result-character hex in that order; no body invocation occurs. Six original shell digests, native source/header digests, selected Jim dynamic-library digests and process/status/stream hashes are retained. Full launched patchlevels, Jim revision/configure flags and original shell stdin-versus-file argument vector were not queried. Success-return bytes are separate from textual validation and do not grant physical native/CPP/compiler storage.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4 (capture association; full launched version was not queried). Build: Selected original shell SHA256 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: C Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	1	63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### tcl8.5

Status: `observed`. Version: 8.5 (capture association; full launched version was not queried). Build: Selected original shell SHA256 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: C Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	1	63616e2774206372656174652070726f63656475726520223a702220696e206e6f6e2d676c6f62616c206e616d6573706163652077697468206e616d65207374617274696e67207769746820223a22
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### tcl8.6

Status: `observed`. Version: 8.6 (capture association; full launched version was not queried). Build: Selected original shell SHA256 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: C Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	0	
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### tcl9.0

Status: `observed`. Version: 9.0 (capture association; full launched version was not queried). Build: Selected original shell SHA256 cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: C Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	0	
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### tcl9.1

Status: `observed`. Version: 9.1 (capture association; full launched version was not queried). Build: Selected original shell SHA256 d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: C Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	0	
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### jim

Status: `observed`. Version: jim (capture association; full launched version was not queried). Build: Selected original shell SHA256 d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0; recorded native-input and loaded-library digests are in the provider receipt. They supply no absent source excerpt or current build equivalence; compiler/configure and Jim revision are unrecorded.. Channel: ASCII self-presenting shell source; original stdin-versus-file invocation unrecorded. Dialect: Jim Tcl.

Exact original case/completion/result-character hex:

```text
local-colon	0	3a70
```
The shell process exits0; the middle column is the caught guest completion, independently from process status. Success return text is preserved separately from declaration validation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance declaration capture for this exact ASCII source is attached; C/Jim holder validation and return text do not establish BIG-IP load or event outcomes.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/authored_procedure_names/source.tcl](../../../../rust/tcl-registry/tests/data/authored_procedure_names/source.tcl). SHA-256 `76f251fec535cf38634e72c7d1d79909242888b045c0333c53f38d29d25c4b04`. Exact three original self-presenting declaration controls; no invocation of the created body.
- `receipt` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. Actual six source/shell/status/stream/native-input associations.
- `projection` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/native.tsv](../../../../rust/tcl-registry/tests/data/authored_procedure_names/native.tsv). SHA-256 `4878d0fc670387ad794a8bca9f6dd8b144a95781cdc6bb7a4bd9118d1a7e030d`. Exact eighteen rows including authored holder/name annotations independently checked against the source and original stdout.
- `provider-tcl8.4` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/0`. This exact selected shell association, source/header digest references and status.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/8.4.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/8.4.txt). SHA-256 `8b14197afedbbcca0a966ac750c7f478fbfa99343b4088f89982657f4164bc32`. Lines 1–1. Original caught declaration completion/result stream.
- `provider-tcl8.5` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/1`. This exact selected shell association, source/header digest references and status.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/8.5.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/8.5.txt). SHA-256 `8b14197afedbbcca0a966ac750c7f478fbfa99343b4088f89982657f4164bc32`. Lines 1–1. Original caught declaration completion/result stream.
- `provider-tcl8.6` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/2`. This exact selected shell association, source/header digest references and status.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/8.6.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/8.6.txt). SHA-256 `f5f9f2aeaff367ef4d0e45e47c964f6d1ca7222c05eef90e0384a68e15423176`. Lines 1–1. Original caught declaration completion/result stream.
- `provider-tcl9.0` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/3`. This exact selected shell association, source/header digest references and status.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/9.0.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/9.0.txt). SHA-256 `f5f9f2aeaff367ef4d0e45e47c964f6d1ca7222c05eef90e0384a68e15423176`. Lines 1–1. Original caught declaration completion/result stream.
- `provider-tcl9.1` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/4`. This exact selected shell association, source/header digest references and status.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/9.1.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/9.1.txt). SHA-256 `f5f9f2aeaff367ef4d0e45e47c964f6d1ca7222c05eef90e0384a68e15423176`. Lines 1–1. Original caught declaration completion/result stream.
- `provider-jim` (provider): [rust/tcl-registry/tests/data/authored_procedure_names/manifest.json](../../../../rust/tcl-registry/tests/data/authored_procedure_names/manifest.json). SHA-256 `e69a52fda2314c95787f3ace31fbcb25c38b6a24158525204a30f1c22d66d47e`. JSON pointer `/engines/5`. This exact selected shell association, source/header digest references and status.
- `rows-jim` (observation): [rust/tcl-registry/tests/data/authored_procedure_names/jim.txt](../../../../rust/tcl-registry/tests/data/authored_procedure_names/jim.txt). SHA-256 `3b7a95a4c9d2a6779e6835e72e3db256e8d28d84352b8d71194abff7a6cb5c6f`. Lines 1–1. Original caught declaration completion/result stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `procedure_name_creation_error_for_policy`: Selected authored/native textual procedure-name validation; acceptance supplies no physical procedure/header/CPP allocation or native success return value.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::tests::procedure_creation_name_policy_matches_18_native_slots_without_header_grants` (linked): Compares selected naming-policy acceptance/error bytes for all18 provider/holder/name annotations, including this case; native success-result bytes remain separate observed data and are not manufactured by the validation query.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact complete self-presenting source and six stdout streams are retained with original source/stream digests. The receipt lacks the original shell invocation argument vector, and raw stderr files are absent despite recorded empty-stderr digests; no exact original-channel runner is claimed. A reconfirmation must explicitly select and identify each provider, record stdin versus file consumption, hash the unchanged source, and retain process status and separate streams. Authored naming policy queries are Rust text-validation contracts and cannot donate native execution or physical procedure ownership.
