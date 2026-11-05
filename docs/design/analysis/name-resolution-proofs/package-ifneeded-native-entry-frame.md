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

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: tcl8.4 8.4.20.

The observed stdout is:

```text
VERSION|8.4.20
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.5

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: tcl8.5 8.5.19.

The observed stdout is:

```text
VERSION|8.5.19
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl8.6

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: tcl8.6 8.6.18.

The observed stdout is:

```text
VERSION|8.6.18
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.0

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: tcl9.0 9.0.4.

The observed stdout is:

```text
VERSION|9.0.4
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### tcl9.1

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: tcl9.1 9.1.0.

The observed stdout is:

```text
VERSION|9.1.0
RESULT 0 1.0 {{:: 0 0}}
```

Process status 0 and empty stderr. RESULT records catch code 0, returned version 1.0 and the one callback observation {:: 0 0}: global namespace, info level zero, and no visible caller-local localSentinel.

### jim

Status: `unsupported`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: jim 0.84-9-g5bac7c9.

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

Status: `not-tested`.

No observation for this exact question.

## Consumer bindings

- [rust/tcl-registry/src/body_execution.rs](../../../../rust/tcl-registry/src/body_execution.rs), `body_execution::tests::deferred_global_entry_keeps_the_authored_native_family` (linked): Authored C-only deferred frame data covers C8.4–9.1 and refuses Jim and hosted profiles; it does not execute the retained native frame probe.
- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded` (linked): Genuine Jim-point catalogue contains no ifneeded row; no entered Jim callback frame or appliance behaviour is asserted.

## Evidence

- [rust/tcl-registry/tests/data/native_package_future_frame](../../../../rust/tcl-registry/tests/data/native_package_future_frame)
- [rust/tcl-registry/tests/data/native_package_future_frame/84](../../../../rust/tcl-registry/tests/data/native_package_future_frame/84)
- [rust/tcl-registry/tests/data/native_package_future_frame/85](../../../../rust/tcl-registry/tests/data/native_package_future_frame/85)
- [rust/tcl-registry/tests/data/native_package_future_frame/86](../../../../rust/tcl-registry/tests/data/native_package_future_frame/86)
- [rust/tcl-registry/tests/data/native_package_future_frame/90](../../../../rust/tcl-registry/tests/data/native_package_future_frame/90)
- [rust/tcl-registry/tests/data/native_package_future_frame/91](../../../../rust/tcl-registry/tests/data/native_package_future_frame/91)
- [rust/tcl-registry/tests/data/native_package_future_frame/jim](../../../../rust/tcl-registry/tests/data/native_package_future_frame/jim)
