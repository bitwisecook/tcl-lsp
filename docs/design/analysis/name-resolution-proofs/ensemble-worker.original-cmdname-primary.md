# naming.ensemble-worker.original-cmdname-primary

Kind: `native-observation`

## Problem statement

An ensemble mapping prefix can contain an original cmdName object already bound to the a: table. Reconstructing its displayed ::a:::w as a fresh name would lose that native selected command primary.

## Question

Does an explicitly original cached cmdName mapping prefix select the same command as a fresh written ::a:::w?

## Conclusion

C8.6/9.0/9.1 return selected from the original cached mapping and collision from the fresh written command. The original selected command object and equal displayed bytes are distinct input authorities; no source producer or future command lifetime is granted.

## Scope

Original initialized C interpreter, ASCII Tcl_Eval ensemble setup, actual native ENSEMBLE_COMPILE flags, and compiled procedure call. Cached control independently selects the actual a: command table through private native fields and attaches TclSetCmdNameObj before mapping. Result strings are printed after reached calls; no refcounts, pointer values or allocation/lifetime facts are sampled. C8.4 and uncompiled cached branches are explicit no-measurement limits; Jim has a separate setup question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: C Tcl.

This control is excluded by the probe compile-time release branch. Retained stdout: 'unsupported\tC8.4 has no namespace ensemble compiler'. That is not a measured guest API rejection.

### tcl8.5

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: C Tcl.

This control is excluded by the probe compile-time release branch. Retained stdout: 'compiled-collision\tcollision collision ::a:'. That is not a measured guest API rejection.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Archive; observer; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

cached-selected	selected collision

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Archive; observer; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

cached-selected	selected collision

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Archive; observer; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

cached-selected	selected collision

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

This exact original selected C ensemble worker control was not run for this provider.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

This exact original selected C ensemble worker control was not run for this provider.

## Historical references

- `input` (input): [rust/tcl-registry/tests/data/native_selected_worker_names/probe.c](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/probe.c). Exact original compiled/cached mapping controls and native primary producer.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). Original selected C archive/observer/process associations.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv). Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). JSON pointer `/engines/0`. Original archive/observer/launch identity.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv). Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). JSON pointer `/engines/1`. Original archive/observer/launch identity.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv). Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). JSON pointer `/engines/2`. Original archive/observer/launch identity.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv). Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). JSON pointer `/engines/3`. Original archive/observer/launch identity.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv). Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). JSON pointer `/engines/4`. Original archive/observer/launch identity.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source and release stdout tables are attached. Original archive/observer hashes and process0/empty stderr are recorded; stdout hashes, full launched patchlevel, compiler/configure/header identities are absent. Reconfirmation must keep compiled-collision and independently installed cmdName mapping controls distinct and retain selected configured private-header closure. No native run/Rust parity is claimed.
