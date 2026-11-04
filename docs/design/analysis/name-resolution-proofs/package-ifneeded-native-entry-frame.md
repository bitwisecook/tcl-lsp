# naming.package.ifneeded-native-entry-frame

Kind: `native-observation`

## Problem statement

A loader registered inside namespace Registrar is required from procedure Caller::run, which has a localSentinel variable. An analyser could wrongly give the deferred callback either the registration namespace or the require caller frame. Family-specific scheduling must be distinguished from the authored body role, and a Jim registration rejection cannot answer an entered callback frame.

## Question

On the fixed original ASCII source-file probe, what namespace, info level and caller-local visibility does package ifneeded use when package require invokes the loader from Caller::run, and does current Jim accept the registration?

## Conclusion

All five captured C providers invoke this loader in namespace :: at info level 0, with Caller::run localSentinel absent, and package require returns 1.0 with catch code 0. Current Jim 0.84-9-g5bac7c9 rejects ifneeded at registration before the Caller definition or callback entry, so it supplies no entered-frame answer. The shared source body role cannot establish scheduling in Jim or a hosted BIG-IP context.

## Scope

One complete fixed ASCII script in a fresh native shell process, with recorded shell argument vectors, source/executable digests and exact byte streams. Actual info patchlevel is printed before registration. No counted object-vector, native compilation admission, opaque/zero names, variable links, multiple deferred loaders or appliance execution is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.4.20
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.5.19
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.6.18
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|9.0.4
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|9.1.0
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected executable; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: Jim Tcl.

The observed stdout is:

```text
VERSION|0.84-9-g5bac7c9
```

Process status 1. Registration rejects ifneeded; no callback runs. Exact stderr is:

```text
/workspace/.proofs/captured-package-future-frame121/probe.tcl:4: Error: package, unknown command "ifneeded": should be forget, names, provide, require
Traceback (most recent call last):
  File "/workspace/.proofs/captured-package-future-frame121/probe.tcl", line 4
    package ifneeded R2286FutureFrame 1.0 {...
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No observation for this exact question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_package_future_frame/probe.tcl](../../../../rust/tcl-registry/tests/data/native_package_future_frame/probe.tcl). Exact complete original ASCII source program.
- `capture` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/receipt.json). Original capture association, executable/source/stream digests and process statuses for all six selected processes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/84/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/receipt.json). Exact selected process argument vector, digests and process status.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/84/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/stdout). Exact complete process stdout bytes.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/84/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/stderr). Exact complete process stderr bytes.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/85/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/receipt.json). Exact selected process argument vector, digests and process status.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/85/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/stdout). Exact complete process stdout bytes.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/85/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/stderr). Exact complete process stderr bytes.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/86/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/receipt.json). Exact selected process argument vector, digests and process status.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/86/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/stdout). Exact complete process stdout bytes.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/86/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/stderr). Exact complete process stderr bytes.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/90/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/receipt.json). Exact selected process argument vector, digests and process status.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/90/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/stdout). Exact complete process stdout bytes.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/90/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/stderr). Exact complete process stderr bytes.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/91/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/receipt.json). Exact selected process argument vector, digests and process status.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/91/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/stdout). Exact complete process stdout bytes.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/91/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/stderr). Exact complete process stderr bytes.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/receipt.json). Exact selected process argument vector, digests and process status.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/jim/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/stdout). Exact complete process stdout bytes.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/jim/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/stderr). Exact complete process stderr bytes.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/body_execution.rs](../../../../rust/tcl-registry/src/body_execution.rs), `body_execution::tests::deferred_global_entry_keeps_the_authored_native_family` (linked): Authored C-only deferred frame data covers C8.4–9.1 and refuses Jim and hosted profiles; it does not execute the retained native frame probe.
- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded` (linked): Genuine Jim-point catalogue contains no ifneeded row; no entered Jim callback frame or appliance behaviour is asserted.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "/workspace/.proofs/oracle-bin/tclsh8.4",
  "/workspace/.proofs/captured-package-future-frame121/probe.tcl"
]
```

The recorded original paths are capture-machine locations; the identical retained source is available in rust/tcl-registry/tests/data/native_package_future_frame/probe.tcl. Replay each separately verified provider executable in a fresh process and record its actual patchlevel and all streams. Executable digests are recorded, while linked library/header digests, compiler flags and build configuration are not. This CLI script observation supplies no counted object-vector, physical native header or compilation-admission receipt. No BIG-IP execution is attached.
