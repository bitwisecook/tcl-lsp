# naming.interpreter.singleton-create-original-spelling

Kind: `native-observation`

## Problem statement

The C creator treats fewer than two list elements differently from its later path resolver: singleton braces remain part of the child name, while GetInterp decodes list elements. An explicit empty path also differs from omitting the path. The check bounds the original spelling/result and subsequent addressing behavior; it does not justify auto-naming an explicitly empty operand.

## Question

For singleton braced and explicit empty original path objects, what name is created, which original/alternate paths address it, and is the original create result retained?

## Conclusion

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child. Jim named-path creation is unavailable. No auto-name, command-publication or arbitrary list equivalence follows.

## Scope

Seven fixed paths: counted raw-zero/encoded C080/opaque FF/D800, singleton braced spelling, explicit empty and unmatched brace. Five C releases and current Jim startup/unsupported control. Excludes nested parent paths, options, command-name qualification, hidden commands, aliases, compilation and unknown/custom object getter behavior.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 614e443fc4e7d5635a1ffaf7d9a65dfdeb2823dded3fb81dea9ab4ad1fb04505; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 8a8c368e21ac4af8f22bc2786a97296f35988c82504cc7873d6166a6dbd67a02; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 7a30ba05d5cbca5772f6ca1ad6710bd82298d61770df0235d2e6cb206e3f158a; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 43f1f05205e0c48f580d6fcea6af72da3180df1cca55be224e5e5d66a8e8bef0; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 412a96917933d1c27c442ec1d2204819a6b1a5a1e790bf3319dfcc9002e85521; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Tcl.

All five C builds create the whole singleton original spelling, return that original object, and require the additional braced address to reach its braced child key. An explicit empty operand creates an empty-key child; an empty lookup still names the current interpreter, while the one-empty-element path reaches the child.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 78e5a685007093131a3848b62adb97a94c2c6ae71a352af42575d3ec1f10d89e; source/header/Makefile/library/probe/build command and process stream hashes are retained.. Channel: Public counted Tcl_NewStringObj/Tcl_EvalObjv path/argv and original ListObjIndex result children; Jim control uses its own direct ASCII evaluator.. Dialect: Jim Tcl.

Named C-style path API is rejected with the original wrong-argument outcome.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this API question.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_interpreter_paths/probe.c](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/probe.c). Exact counted native constructor/argv and public API input program, including genuine raw zero versus encoded C080; original create identity checked independently of string reporting.
- `runner` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/capture.py](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/capture.py). Exact original build/launch procedure; original captured paths are retained, not a portable replayer.
- `aggregate` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/receipt.json). All-six actual compile/process/header/library/Makefile/source-owner/executable/stream joins and reported versions.
- `tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.4.20/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.5.19/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/8.6.18/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.0.4/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/9.1.0/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/receipt.json). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/stdout.tsv). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_interpreter_paths/jim/stderr](../../../../rust/tcl-vm/tests/data/native_interpreter_paths/jim/stderr). Exact actual provider build/process association and complete result stream; guest errors and unsupported operations remain distinct from process success.

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
