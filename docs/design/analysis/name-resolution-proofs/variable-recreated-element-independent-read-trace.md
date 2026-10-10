# naming.variable.recreated-element-independent-read-trace

Kind: `native-observation`

## Problem statement

An old array-element read callback destroys the entire array, recreates the same written member and installs a new read observer before the old callback returns. The new read observation and the old pending increment completion cannot be inferred from matching key text.

## Question

For the exact original ASCII array probe, can the newly created same-key member run its new read observer before the old callback returns, and what completion does the old captured increment report in each measured C release?

## Conclusion

All five C captures observe the new member value 100 and new READ callback before the old callback returns, then retain final value 100. C8.4 reports catch code 1 with can't read "::a(k)": no such element in array. C8.5 through C9.1 report catch code 1 with can't set "::a(k)": upvar refers to element in deleted array. Current Jim rejects trace registration at SETUP and supplies no entered-callback or increment answer.

## Scope

One fixed ASCII source-file CLI script in each of five actual C releases and current Jim. The new read, callback log, old increment error and final value are measured as one guest result. No native pointer/header, compiled receiver recipe, dictionary key allocation identity, generic physical stale-handle claim, scalar-retirement generalization or appliance behavior follows. Linked C port controls replay the identical complete original source and compare the full observation output after the independently identified PATCHLEVEL line. Linked Jim controls execute only the identical original source prefix through trace SETUP and compare the stored setup code/message with the original SETUP bytes; the unrelated binary formatter and callback body are not replayed. The original complete native source and stdout remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't read "::a(k)": no such element in array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Jim Tcl.

Trace SETUP catch code 1 returns invalid command name "trace". No OBSERVATION row exists, so member recreation, new read execution and old increment completion are unanswered.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_recreated_element_read_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/probe.tcl). SHA-256 `6332cbe0cc8ea8128016174dad8719410b89ed844f0234232ca6fbcf34b34e32`. Exact original complete ASCII LF source-file input; no object-vector or native compilation admission is recorded.
- `capture-runner` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/capture.py](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/capture.py). SHA-256 `7d517f1ab2deb50c65b14582cb5b517562f65ec7acfd274ccd88ea5c7305af6b`. Exact retained Root capture runner; its original absolute queue path is preserved. No replay is launched by this review.
- `provider-queue` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/queue.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact provider queue consumed by the retained runner. Only provider process receipts actually present in this fixture establish a run.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/receipt.json). SHA-256 `e37b61610e871712e8cc6c36b37d0c9dc8cf9b8adfa19b3278ae7e47746a0008`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stdout). SHA-256 `c870bd1bba73299339c1b8bfa9df1cfecf288ae2dbf07a0ae94198696371ce91`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/receipt.json). SHA-256 `fc145a0b305a030c866b124c932ce3e52ec7b31d876cf65c82e256d6a8e4ac40`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stdout). SHA-256 `89f2e08358d4a89316cecde48dc410c56188bc14381437b89f00f4e495054dd1`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/receipt.json). SHA-256 `711762132324b4f631bbbdf9aba28550ff10d7bfc0c3b3df08cbdc5d4189b103`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stdout). SHA-256 `d08a1df787dc902dc058e5800f1af053b80af90702ff8d6e1c4ceda0e11ec39d`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/receipt.json). SHA-256 `6fb217296abac180487a98fd3086c670aaea7838cc4332f0c1fdfac3c610eecf`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stdout). SHA-256 `c6582b09d11f8777fece8d7bc7bf09f0aa6753ce9c55637e4c4c785d22db4362`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/receipt.json). SHA-256 `1788e7ef3b3606dcc002d3bbe044015cb24fbea04544ce0b92f1442b7716ecb5`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stdout). SHA-256 `b5c4c04b123285dd468e1d4638a4de32c9233bcc122241f56e3450e6ef3912b5`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/receipt.json). SHA-256 `5af46436e2256c20eed3ee78c28c787e4a87e7721a4a65898acfe94fecbb4d17`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stdout). SHA-256 `c8a5f039671d85b72826dba888e0de69630a6c7cdb44e385f7898155b81beab5`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_variable_trace.rs](../../../../rust/tcl-runtime-api/src/native_variable_trace.rs), `NativeVariableTraceOperation::uses_destroyed_cell_callbacks`: Select reached destroyed-cell unset notification separately from active read/write suppression; actual source capture establishes guest behavior only.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `VarTable::native_trace_element_identity`: Join the actual captured root VarId to an existing current member VarId; duplicate display names cannot select another root.
- [runtime/rust/src/vars.rs](../../../../runtime/rust/src/vars.rs), `trace_element_identity`: Forward the same exact root/member identity query without issuing a key-string allocation receipt.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::variable_trace_scope`: Retain actual root and member allocation identities across all Runtime trace guard producers; array-scoped guards keep their own root purpose.
- [runtime/rust/src/cmd_trace.rs](../../../../runtime/rust/src/cmd_trace.rs), `VarTraceScope`: Keep reached cell and array guard identities separate, independently of written receiver labels.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `VarTraceCell`: Retain the actual reached element VarId independently from its array VarId during active trace suppression.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `report_native_c_variable_value_name`: Preserve original compound reporting parts independently of the actual root/member receiver selection; additional Unicode extent controls are Rust-only.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::recreated_element_read_trace_matches_five_original_source_controls` (linked): Compare all five C complete original array-recreation source outputs after the separate PATCHLEVEL row: new READ/value 100 before old return and the independently captured C8.4 versus C8.5+ old increment errors. This does not infer private native cell identities.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::recreated_element_trace_probe_preserves_actual_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP and compare stored setup code/message to the captured unsupported SETUP bytes. No entered callback or increment answer exists.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::recreated_element_read_trace_matches_five_original_source_controls` (linked): Compare all five C complete original array-recreation source outputs after the separate PATCHLEVEL row: new READ/value 100 before old return and the independently captured C8.4 versus C8.5+ old increment errors. This does not infer private native cell identities.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::recreated_element_trace_preserves_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP and compare stored setup code/message to the captured unsupported SETUP bytes. No entered callback or increment answer exists.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::trace_member_identity_selects_original_duplicate_root_allocations` (linked): Rust allocator/API coverage only: two duplicate declaration display names mint distinct actual root/member VarIds, and each exact root ID selects its own current member while an absent key remains absent. The independent original source142 capture supplies no private native duplicate-slot layout proof.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::c_value_report_name_keeps_original_compound_parts` (linked): The separate captured recreated-element source supplies the exact ::a(k) error-name obligation. Authored Rust controls retain original whole versus separate compound parts and supplementary-character extents across five C recipe selections; those additional extent controls measure no native opaque name or private receiver layout.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/native-recreated-element142/capture.py"
]
```

The original capture runner and process argument vectors retain capture-machine absolute paths. The archived source, runner and provider queue are byte-identical. Replay requires independently verified matching SDK/environment/executable pins and fresh output locations. This review verifies immutable bytes without launching a process. ASCII source-file completion/results do not establish counted object-vector admission, internal object identity, header/cache state, compiler recipes, arbitrary callback scheduling or appliance behavior.
