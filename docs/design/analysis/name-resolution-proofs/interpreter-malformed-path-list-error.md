# naming.interpreter.malformed-path-list-error

Kind: `native-observation`

## Problem statement

A malformed path can be accidentally treated as a literal child name if a consumer falls back after list parsing fails. The native path API instead has operation-specific error handling: exists consumes the guest lookup error while create/eval/delete retain it. These fixed malformed-list observations do not authorize consuming host/refusal errors.

## Question

What do create, exists, children, eval and delete do with the original unmatched-open-brace path?

## Conclusion

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table. Jim rejects the named API independently. A malformed list never becomes a child key; host protocol or object-getter refusal remains independent and cannot be converted to false from these guest observations.

## Scope

Seven fixed paths: counted raw-zero/encoded C080/opaque FF/D800, singleton braced spelling, explicit empty and unmatched brace. Five C releases and current Jim startup/unsupported control. Excludes nested parent paths, options, command-name qualification, hidden commands, aliases, compilation and unknown/custom object getter behavior.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 614e443fc4e7d5635a1ffaf7d9a65dfdeb2823dded3fb81dea9ab4ad1fb04505; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 8a8c368e21ac4af8f22bc2786a97296f35988c82504cc7873d6166a6dbd67a02; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 7a30ba05d5cbca5772f6ca1ad6710bd82298d61770df0235d2e6cb206e3f158a; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 43f1f05205e0c48f580d6fcea6af72da3180df1cca55be224e5e5d66a8e8bef0; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 412a96917933d1c27c442ec1d2204819a6b1a5a1e790bf3319dfcc9002e85521; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds reject create/eval/delete with unmatched-open-brace list errors, return false from exists and retain an empty children table.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 78e5a685007093131a3848b62adb97a94c2c6ae71a352af42575d3ec1f10d89e; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Jim Tcl.

Named C-style path API is rejected with the original wrong-argument outcome.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this API question.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_interpreter_paths/probe.c](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/probe.c). SHA-256 `7c2492c3662c5f5a379bf7cea7fc4a542591099d0b6ce950b64d3173f7e460fd`. Exact counted native constructor/argv and public API input program, including genuine raw zero versus encoded C080; original create identity checked independently of string reporting.
- `runner` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/capture.py](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/capture.py). SHA-256 `bb6cd992ae6dba121a4ec4c8adb3d516ab4f6c28018dd5dd56f700329f3f8228`. Exact original build/launch procedure; original captured paths are retained, not a portable replayer.
- `aggregate` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/receipt.json). SHA-256 `3fe010a2e330915214d518db68dcb8379ebe81d8a3d23334ae0deae107cc4973`. All-six actual compile/process/header/library/Makefile/source-owner/executable/stream joins and reported versions.
- `tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/receipt.json). SHA-256 `19015d5f80a454e682c329911032609935ea7f3ae365c693fdcb1b590b36b09b`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stdout.tsv). SHA-256 `07d9ed0ce6be88965093d4fd1c5872404694db60f2bdb148941258503fbb6846`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/receipt.json). SHA-256 `7ce969c3a913b7792040c7995edeb4f985d089f87b763dd70d2d457a8b12aa77`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stdout.tsv). SHA-256 `e7524bb5000d1079592702ae1b4da7d86b804d674a9a670f41d6951e8ae7233b`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/receipt.json). SHA-256 `589d45cea310b4218c26e7c2baadd21a3eb0c9d2eea26b3f625c626aa5d805a6`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stdout.tsv). SHA-256 `e46b8e14af259b0c93f466556f12a6cc1c03e92cead6254a9c2f8fda00537bb8`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/receipt.json). SHA-256 `8b413fe08935a8b24e2f7533da2148cd44b4c0a69b72467661e0fa766a8cec1d`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stdout.tsv). SHA-256 `9938902d767b8ce2eca09592eb2d4b55a0a78c3d25370a3ff2418f9afc4f51ee`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/receipt.json). SHA-256 `210a4fb809d004572825718678aa0358717b41399629b2e5a0e8fc0225fcf1cb`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stdout.tsv). SHA-256 `b0fbf7c471851e60710f823ef3071e44cae20b8ac77e659d065c80d5aef564bc`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/receipt.json). SHA-256 `5dd3e30f3d40ce7cb6970d449f5a07ec6d0ef63bed624e64401a3f70493a7905`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/stdout.tsv). SHA-256 `c001133564e67872c99ccc779b8913ab84a44511e0f74548150eb8b5085c0ba5`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::interpreter_child_input`: Pure independently selected CString child-key extent; does not parse command/namespace addresses or prove child existence.
- [rust/tcl-vm/src/interp/native_interpreter_paths.rs](../../../../rust/tcl-vm/src/interp/native_interpreter_paths.rs), `Vm::resolve_interp_path_original`: Actual selected original object-list getter and current alive child-table traversal.
- [rust/tcl-vm/src/interp/native_interpreter_paths.rs](../../../../rust/tcl-vm/src/interp/native_interpreter_paths.rs), `interp::native_interpreter_paths::tests::original_interpreter_paths_keep_counted_objects_and_child_cstring_keys` (linked): Original object-vector C path inputs retain byte keys/result identity and singleton/malformed geometry. No Rust execution or full native handler/CPP grant is inferred.
- [runtime/rust/src/cmd_alias.rs](../../../../runtime/rust/src/cmd_alias.rs), `cmd_alias::tests::original_interpreter_paths_preserve_child_keys_and_create_result_objects` (linked): Original counted C path objects preserve create-result allocation, CString child selection, singleton source spelling and malformed-list errors through actual runtime tables; this is a binding, not an executed comparison receipt.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original build runner retains workspace paths and new-output-directory requirement. Exact original source/build/executable/status/stdout/stderr hashes are retained, but no portable replay or Rust/native launch is claimed by this publication.
