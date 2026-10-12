# naming.variable.unset-active-read-callback-retirement

Kind: `native-observation`

## Problem statement

An active scalar read callback unsets and recreates the same written scalar. Its old unset observers, old read registration and later read result have separate purposes; a trace setup rejection cannot answer any entered callback behavior.

## Question

For the exact original ASCII scalar probe, does unsetting during an active read invoke the old unset callback, retire the old registrations, and let the later read observe the recreated value without rerunning the old read callback?

## Conclusion

All five captured C releases produce the exact caught result 7 7 {read unset} {}: the old read and unset callbacks are observed once, both reads return the recreated value 7, and the final trace inventory is empty. Current Jim rejects trace registration with invalid command name "trace" at SETUP, so no OBSERVATION row or callback answer exists.

## Scope

One fixed scalar script per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim fresh CLI process. Only the recorded source setup/completion/result sequence is measured. No physical captured receiver, pointer/header epoch, compiler admission, array-element identity, generic increment/container claim or BIG-IP answer follows. Linked C port controls replay the identical complete original source and compare the full observation output after the independently identified PATCHLEVEL line. Linked Jim controls execute only the identical original source prefix through trace SETUP and compare the stored setup code/message with the original SETUP bytes; the unrelated binary formatter and callback body are not replayed. The original complete native source and stdout remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Tcl.

SETUP catch code 0; OBSERVATION catch code 0 with exact result 7 7 {read unset} {}. The two scalar reads return 7, the callback log is read then unset, and the final trace inventory is empty. Outer process status is 0 and stderr is empty.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Jim Tcl.

Trace registration has catch code 1 and exact result invalid command name "trace". There is no OBSERVATION row, so active callback execution and retirement are not answered.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_active_read_unset_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/probe.tcl). SHA-256 `f6f829d0e5be788f72196fbab3db9d5eb0e7db6bc0c288b57ebf8bdcfd0d58ed`. Exact original complete ASCII LF source-file input; no object-vector or native compilation admission is recorded.
- `capture-runner` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/capture.py](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/capture.py). SHA-256 `af789c1544b1f2c8c36907ad35ac155d010b2c8acb857f8ff1afb971d714fcc1`. Exact retained Root capture runner; its original absolute queue path is preserved. No replay is launched by this review.
- `provider-queue` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/queue.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact provider queue consumed by the retained runner. Only provider process receipts actually present in this fixture establish a run.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/receipt.json). SHA-256 `afa74a257e34732f64846404143cd74792482d074d9f26910e9f25f976768f42`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/stdout). SHA-256 `564a5811a5f2d66ed9b81300e8ffcc59f14dc139e409455ac998f26e9d58cc2a`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/receipt.json). SHA-256 `a5072e92de8f9c4461b11726fb30a0f80f7f411a40ed2448ceb000fcb02fc39e`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/stdout). SHA-256 `371ae5845636e65ee5d8829798a3c06b567b1d465fea30131c9fedd6be7a73f4`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/receipt.json). SHA-256 `a5c5b81086324fc1d732809c53e1eab4ce14ddeae890b3beb387c95571e7d7c7`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/stdout). SHA-256 `3f9e847c3d534d0588c62fcf5985ac19ee7133717a67393193949b9d734e10a8`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/receipt.json). SHA-256 `b797b421dd0f642087de2d4b37333258879eeabb01576fd41cb9561eae40cfb8`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/stdout). SHA-256 `6825bbf8a23ed8efd872e80c293ed3fcb80ee9afd12dcff171202f27d11ade89`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/receipt.json). SHA-256 `935de03349148ab168638faf7e4b566175d1b54c1b7e764de894936ae6283376`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/stdout). SHA-256 `0df2f141b034a0cf151260a5642cc0d6b55c61dd30d3764f904c0e73b59fcbde`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_active_read_unset_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/jim/receipt.json). SHA-256 `be0c4bb837d3130434988eca39493493205964fa0bce7d41d484b3fa87e52e0e`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/jim/stdout). SHA-256 `c8a5f039671d85b72826dba888e0de69630a6c7cdb44e385f7898155b81beab5`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_active_read_unset_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_active_read_unset_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_variable_trace_requires_name`: Keep actual destroyed-cell unset notification separate from active read/write suppression; the CLI result does not itself establish internal receiver identity.
- [rust/tcl-runtime-api/src/native_variable_trace.rs](../../../../rust/tcl-runtime-api/src/native_variable_trace.rs), `NativeVariableTraceOperation::uses_destroyed_cell_callbacks`: Select reached destroyed-cell unset notification separately from active read/write suppression; actual source capture establishes guest behavior only.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_retires_old_registrations_matches_five_native_source_controls` (linked): Compare all five C complete original scalar-source observation outputs after the separate PATCHLEVEL row. Old unset delivery, old trace retirement and later recreated value stay separate from private native receiver/header conclusions.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_probe_preserves_actual_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP, then compare stored setup code 1 and exact invalid-command message with the captured SETUP hex. No binary formatter replay, callback observation or full-script Rust success is claimed.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_matches_five_original_source_controls` (linked): Compare all five C complete original scalar-source observation outputs after the separate PATCHLEVEL row. Old unset delivery, old trace retirement and later recreated value stay separate from private native receiver/header conclusions.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::active_read_unset_preserves_jim_unsupported_setup` (linked): Execute only the identical original Jim source prefix through trace SETUP, then compare stored setup code 1 and exact invalid-command message with the captured SETUP hex. No binary formatter replay, callback observation or full-script Rust success is claimed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/native-active-unset138/capture.py"
]
```

The original capture runner and process argument vectors retain capture-machine absolute paths. The archived source, runner and provider queue are byte-identical. Replay requires independently verified matching SDK/environment/executable pins and fresh output locations. This review verifies immutable bytes without launching a process. ASCII source-file completion/results do not establish counted object-vector admission, internal object identity, header/cache state, compiler recipes, arbitrary callback scheduling or appliance behavior.
