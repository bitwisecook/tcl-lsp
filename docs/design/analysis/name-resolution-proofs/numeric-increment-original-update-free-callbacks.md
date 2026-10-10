# naming.numeric.increment-original-update-free-callbacks

Kind: `native-observation`

## Problem statement

Incrementing an original custom object can enter its updateString or freeIntRep callback, which modifies the caller frame and replaces a later worker. Treating scalar conversion as a quiet value getter would ignore observable effects.

## Question

Do compiled/direct and aliased generic incr routes enter original update/free callbacks before the following caller-cell and worker observations?

## Conclusion

All five C and the original Jim capture return4 and observe UPDATED/FREED caller keep plus the replacement worker CHANGED in both tested routes. C uses compiled incr versus runtime alias; Jim uses direct versus alias. This proves reached native callback effects for the exact custom operands, not a source compiler completion certificate, general closed handler footprint or an emitted optimizer rewrite.

## Scope

Original custom type has either missing bytes/updateString producing3 or resident3/freeIntRep. Callbacks set keep and delete/recreate worker; the procedure then lists result/keep/worker. Four rows per provider from two routes×two hooks, distinct exact C and Jim TUs, full source/build/output associations retained. C initializes its selected library; Jim registers core commands. Runtime patchlevel/revision/header/compiler/configure metadata is unqueried; BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=e36064383a58baa19ba0055aa47d109335b98574d8500d85b945b2bc4c843ce8; library_sha=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=3edd69a9e4adde0f3f5356933a072289d35355c30891301ef79f635df304c607; library_sha=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=bd7fe83c1f9dbe2ce0c309fefe8b783e9f787621e13eab423340e575af6a9326; library_sha=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=57b66be307f6510b03798f2f48cd5235f506a5651ee241120d535f3334bd7ca1; library_sha=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=c0181a077aafd3b8e39275321b2079bd9aaf5f88d4a9e43223a85a7a1b592a5b; library_sha=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `observed`. Version: Jim0.84 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=d8a772110953228a99da8ffe68c8083b47bc7789d9541ab75f07aaaca796bf1e; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original Jim_Obj updateString/freeIntRep reached by direct or alias incr; callbacks enabled only during entered source.. Dialect: Jim Tcl.

Exact relevant original rows:

```text
route=direct hook=update code=0 result=4 UPDATED CHANGED
route=direct hook=free code=0 result=4 FREED CHANGED
route=alias hook=update code=0 result=4 UPDATED CHANGED
route=alias hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/probe.c). SHA-256 `7a4c78f826d78fc4a7f229311edd5971bf3129965b6e1dd1d932446a3cbd4548`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/manifest.json). SHA-256 `1fcd6050b8820dd07d75cd05b1fd36855b176afbf5baa191944616dfc92f9a8b`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.4.20.txt). SHA-256 `dbe76149c3b8c316e805d51d6e402eb8474a5e2aee05af706a302dc54ab875de`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.5.19.txt). SHA-256 `dbe76149c3b8c316e805d51d6e402eb8474a5e2aee05af706a302dc54ab875de`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/8.6.18.txt). SHA-256 `dbe76149c3b8c316e805d51d6e402eb8474a5e2aee05af706a302dc54ab875de`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/9.0.4.txt). SHA-256 `dbe76149c3b8c316e805d51d6e402eb8474a5e2aee05af706a302dc54ab875de`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/9.1.0.txt). SHA-256 `dbe76149c3b8c316e805d51d6e402eb8474a5e2aee05af706a302dc54ab875de`. Original retained full stream; provider answer selects only this question's relevant windows.
- `probe-7` (input): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-probe.c). SHA-256 `2fceee0127a18505c754ab255b89bb33d14abcce0485adb1885d07152d81428d`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-7` (provider): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-increment-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-increment-manifest.json). SHA-256 `cff27e95a2ff43b49e77c99f934797242bd0a1a3955a07595423b20a33451b11`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim.txt). SHA-256 `8ccd214792b0b576bb9fdc8f4e3095a5ad7f2b99eb97a5aa3ec626470b1c7988`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Selected getter conversion must retain reached original callback effects and independent result/cache obligations.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
