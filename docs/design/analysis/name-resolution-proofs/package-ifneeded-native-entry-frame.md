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

Status: `observed`. Version: 8.4.20. Build: Selected executable SHA-256 b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.4.20
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected executable SHA-256 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.5.19
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected executable SHA-256 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|8.6.18
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected executable SHA-256 f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|9.0.4
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected executable SHA-256 c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: C Tcl.

The observed stdout is:

```text
VERSION|9.1.0
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; compiler, linked library/header digests and configure flags are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: Jim Tcl.

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

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_package_future_frame/probe.tcl](../../../../rust/tcl-registry/tests/data/native_package_future_frame/probe.tcl). SHA-256 `c1cfac561e3048f78ae444d4ed03b0b8d19cc9c158ab104386b601bbbc56593a`. Exact complete original ASCII source program.
- `capture` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/receipt.json). SHA-256 `94d6573be4bafbda54faf6b9c5dae0c4f144120684c6d18b9ac788ed77d41a00`. Original capture association, executable/source/stream digests and process statuses for all six selected processes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/84/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/receipt.json). SHA-256 `2be56abeaca93e4d41790f87e70aa9981be29cdedfd499c7b4536703a5232222`. Exact selected process argument vector, digests and process status.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/84/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/stdout). SHA-256 `a003d751ceb5118487a6a0bb1b6c3688c1dce699af8cd9052770db002cfd04ec`. Exact complete process stdout bytes.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/84/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete process stderr bytes.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/85/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/receipt.json). SHA-256 `ccb635e224226f360db1d47cf88c14dceaf5c9e2640692a306006f38cae7e3ce`. Exact selected process argument vector, digests and process status.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/85/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/stdout). SHA-256 `34d5468066244fc7e35fd6585842a3cf1c205b7d94ca1ccd2042515b0cbddad5`. Exact complete process stdout bytes.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/85/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete process stderr bytes.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/86/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/receipt.json). SHA-256 `48ddb3222781c4d4ddfc713979a17ffe4e98ee604f5ceed674df3ba232c3d9ee`. Exact selected process argument vector, digests and process status.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/86/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/stdout). SHA-256 `896274555ac35af3b12595ec9c4d52e7f20282d2763af7605518c44b410681ec`. Exact complete process stdout bytes.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/86/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete process stderr bytes.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/90/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/receipt.json). SHA-256 `184305fb1059d18644e6114d557ecdc0bec1467c45f8aa8d3311540a84413cd4`. Exact selected process argument vector, digests and process status.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/90/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/stdout). SHA-256 `48df8de9a13170df3271690ae2ae15a9b6e384ad88ce652129280b88bff99fe6`. Exact complete process stdout bytes.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/90/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete process stderr bytes.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/91/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/receipt.json). SHA-256 `1ac0e3d0406f67274c52e5919c0e5d96ca915432ceb81df2642e4564c541aa0d`. Exact selected process argument vector, digests and process status.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/91/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/stdout). SHA-256 `1c3b5e15f82eb398c60be479228d5f8ee3cac7a79dfed4ab3e863c0e50f96ec1`. Exact complete process stdout bytes.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/91/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete process stderr bytes.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_package_future_frame/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/receipt.json). SHA-256 `db256a7a8f2ee4a87374513e36b8a6127fcde368ac7b170e4064247acf19b8f7`. Exact selected process argument vector, digests and process status.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/jim/stdout](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/stdout). SHA-256 `feb37b34eea5e2705844d5448aff71be77c0bb72bb3ad3432d50563c634f3103`. Exact complete process stdout bytes.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_package_future_frame/jim/stderr](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim/stderr). SHA-256 `80134d9830429065438659c29793f5151848250dda8b1e03f6b4a791c25c0324`. Exact complete process stderr bytes.

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
