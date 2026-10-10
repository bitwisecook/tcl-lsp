# naming.ensemble-worker.structured-compiler-collision

Kind: `native-observation`

## Problem statement

A maker namespace component a: and ordinary a display ambiguously when a compiled ensemble maps member to w. Source-selected structured context and reconstructed printed target can select different commands.

## Question

Which command result does compiled E mem VALUE select from a: compared with directly written ::a:::w?

## Conclusion

C8.5–9.1 return collision for both compiled ensemble call and directly written target, while namespace current displays ::a:. This finite compiler-selected mapping does not retain the structured a: worker merely from its presentation.

## Scope

Original initialized C interpreter, ASCII Tcl_Eval ensemble setup, actual native ENSEMBLE_COMPILE flags, and compiled procedure call. Cached control independently selects the actual a: command table through private native fields and attaches TclSetCmdNameObj before mapping. Result strings are printed after reached calls; no refcounts, pointer values or allocation/lifetime facts are sampled. C8.4 and uncompiled cached branches are explicit no-measurement limits; Jim has a separate setup question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: C Tcl.

This control is excluded by the probe compile-time release branch. Retained stdout: 'unsupported\tC8.4 has no namespace ensemble compiler'. That is not a measured guest API rejection.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association). Build: Archive SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; observer SHA256 4d519ae937d111e3a3270237308c59244c9c18afadfd04541868c7bb5a3decef; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

compiled-collision	collision collision ::a:

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association). Build: Archive SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; observer SHA256 252cc3c7bab48634f55c4a8cc949827f0772598d75ce9bd5dddefea7d1d4a415; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

compiled-collision	collision collision ::a:

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association). Build: Archive SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; observer SHA256 b3273d652d241b7e04bf74636d70a879d98599c28322d94934a9da0e61799a1e; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

compiled-collision	collision collision ::a:

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association). Build: Archive SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; observer SHA256 7cee62fef0c8f49e3f6509414d1a68483533a2071ff53dad19490755d8184d08; process0, empty stderr. Compiler/configure/header identity and full launched patchlevel unrecorded.. Channel: Native ensemble compiler/mapping and original Tcl_Eval procedure call; cached control uses independently selected private command primary.. Dialect: C Tcl.

compiled-collision	collision collision ::a:

### jim

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: Jim Tcl.

This exact original selected C ensemble worker control was not run for this provider.

### bigip

Status: `not-tested`. Version: not measured. Build: not measured. Channel: not tested. Dialect: F5 iRules.

This exact original selected C ensemble worker control was not run for this provider.

## Exact evidence

- `input` (input): [rust/tcl-registry/tests/data/native_selected_worker_names/probe.c](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/probe.c). SHA-256 `0c97c11f3e6f5c0a26ab2eb856b780341cd1e22026b6115d6f6e73a4f35b7ea6`. Exact original compiled/cached mapping controls and native primary producer.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. Original selected C archive/observer/process associations.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.4.20.tsv). SHA-256 `07a6bb6fe1348b1b13e26621dcea7585b17763ca4b0bb0a53980a35e88d3a517`. Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.4` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. JSON pointer `/engines/0`. Original archive/observer/launch identity.
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.5.19.tsv). SHA-256 `7382004903bf91b8c2c5db759e80f58eb41bf5684963e382e1f411a4d7a30551`. Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.5` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. JSON pointer `/engines/1`. Original archive/observer/launch identity.
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/8.6.18.tsv). SHA-256 `287fead75ab26dd5242ca97066a51d4246cc620a9838778375705617a37e69b9`. Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl8.6` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. JSON pointer `/engines/2`. Original archive/observer/launch identity.
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.0.4.tsv). SHA-256 `287fead75ab26dd5242ca97066a51d4246cc620a9838778375705617a37e69b9`. Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl9.0` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. JSON pointer `/engines/3`. Original archive/observer/launch identity.
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/9.1.0.tsv). SHA-256 `287fead75ab26dd5242ca97066a51d4246cc620a9838778375705617a37e69b9`. Exact original reached output or compile-time skipped-branch marker.
- `provider-tcl9.1` (provider): [rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_selected_worker_names/manifest.json). SHA-256 `61a1b93314beccb3b69a21779d43c89fa716197ec22c96c37f2adeb1e98cca6b`. JSON pointer `/engines/4`. Original archive/observer/launch identity.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact source and release stdout tables are attached. Original archive/observer hashes and process0/empty stderr are recorded; stdout hashes, full launched patchlevel, compiler/configure/header identities are absent. Reconfirmation must keep compiled-collision and independently installed cmdName mapping controls distinct and retain selected configured private-header closure. No native run/Rust parity is claimed.
