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

Status: `observed`. Version: 8.4.20. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't read "::a(k)": no such element in array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

Exact decoded OBSERVATION result: 1 {can't set "::a(k)": upvar refers to element in deleted array} 100 READ 100. SETUP succeeds; the separately captured new read sees 100 and logs READ before the old callback returns. The old increment reports catch code 1; outer process status is 0 and stderr is empty.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Jim Tcl.

Trace SETUP catch code 1 returns invalid command name "trace". No OBSERVATION row exists, so member recreation, new read execution and old increment completion are unanswered.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_recreated_element_read_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/probe.tcl). Exact original complete ASCII LF source-file input; no object-vector or native compilation admission is recorded.
- `capture-runner` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/capture.py](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/capture.py). Exact retained Root capture runner; its original absolute queue path is preserved. No replay is launched by this review.
- `provider-queue` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/queue.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/queue.json). Exact provider queue consumed by the retained runner. Only provider process receipts actually present in this fixture establish a run.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.4.20/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.5.19/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/8.6.18/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.0.4/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/9.1.0/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/receipt.json). Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stdout). Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_recreated_element_read_original/jim/stderr). Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.

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
