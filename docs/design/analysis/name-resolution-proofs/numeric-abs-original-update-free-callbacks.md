# naming.numeric.abs-original-update-free-callbacks

Kind: `native-observation`

## Problem statement

An expression function may evaluate an original custom numeric operand whose representation callback changes the caller cell and replaces a later worker. The function result and callback effects are independent; treating conversion as quiet would lose the latter.

## Question

Do abs expression routes enter the original update/free callbacks and expose the changed caller cell and later worker?

## Conclusion

Every measured abs route returns code0, the retained native function result, UPDATED/FREED keep and CHANGED worker in C and Jim. C compiled expr and runtime alias are separate from Jim direct/alias expr. The actual function result formatting is release-specific and retained verbatim. These operand callback effects do not prove actual selected-handler replacement during this same function call or an optimizer eligibility check.

## Scope

Original custom object emits3 from updater or already owns resident3 with a freer; callbacks write keep and replace worker, not the math function. One fixed abs operation has four exact route/hook windows per provider within full16-row C/Jim captures. Distinct TUs/source/program/library/executable/full outputs retained; runtime patchlevel, Jim revision, headers/compiler/configure details not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=2b9cbd2e040e2afe1435ba373e06ee74fdb84dfa424c212b3c7c28ea84672a4a; library_sha=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations.. Dialect: C Tcl.

Exact relevant original rows:

```text
operation=abs route=compiled hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=compiled hook=free code=0 result=3 FREED CHANGED
operation=abs route=generic hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=generic hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=afd8608b821a70eb595e1ec392bc7717aa86e78c7ae58d131d394a23170ea931; library_sha=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations.. Dialect: C Tcl.

Exact relevant original rows:

```text
operation=abs route=compiled hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=compiled hook=free code=0 result=3 FREED CHANGED
operation=abs route=generic hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=generic hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=419f89dd54cf5e68422bea12764e59ec4e422af42771898787e8a631627657f3; library_sha=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations.. Dialect: C Tcl.

Exact relevant original rows:

```text
operation=abs route=compiled hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=compiled hook=free code=0 result=3 FREED CHANGED
operation=abs route=generic hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=generic hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=c0b1e833a8922346697edf539bcc9cb655b7ae850e3195dc8fd41c5a03e70488; library_sha=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations.. Dialect: C Tcl.

Exact relevant original rows:

```text
operation=abs route=compiled hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=compiled hook=free code=0 result=3 FREED CHANGED
operation=abs route=generic hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=generic hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=b5f8143cbae8c86b99d30ebf61162a231a1013e5dad8bab9f20e5481715f5d79; library_sha=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations.. Dialect: C Tcl.

Exact relevant original rows:

```text
operation=abs route=compiled hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=compiled hook=free code=0 result=3 FREED CHANGED
operation=abs route=generic hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=generic hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `observed`. Version: Jim0.84 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=cc9792eb67caf1aa4b62ebaac363136519e843893bb0b48570893dee4b60e5b7; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original custom Jim object in direct or alias expr function operand; following caller variable/worker source observations.. Dialect: Jim Tcl.

Exact relevant original rows:

```text
operation=abs route=direct hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=direct hook=free code=0 result=3 FREED CHANGED
operation=abs route=alias hook=update code=0 result=3 UPDATED CHANGED
operation=abs route=alias hook=free code=0 result=3 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks.c). SHA-256 `4c25d07c8527043cfa982c2b0452d19aa9d144091a5e222a7033e1a274c5fa64`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-manifest.json). SHA-256 `1812590a1b0025e65eaa82f9cb78aff148c373683c62ca28529cbfab090ecdb4`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.4.20.txt). SHA-256 `95dee75b304d18c1d8dcade994240a6967038eb475987c3f08134ef0d2d495a8`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.5.19.txt). SHA-256 `75f0144c6ed60704a655a521a0f224bbb249882094ce5e7be73dd77130f98909`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-8.6.18.txt). SHA-256 `75f0144c6ed60704a655a521a0f224bbb249882094ce5e7be73dd77130f98909`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-9.0.4.txt). SHA-256 `75f0144c6ed60704a655a521a0f224bbb249882094ce5e7be73dd77130f98909`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-hooks-9.1.0.txt). SHA-256 `75f0144c6ed60704a655a521a0f224bbb249882094ce5e7be73dd77130f98909`. Original retained full stream; provider answer selects only this question's relevant windows.
- `probe-7` (input): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks.c). SHA-256 `62015c23986f1b04726599a367a4988817f821a87936da6228c1878cc0d57d73`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-7` (provider): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks-manifest.json). SHA-256 `e4e46bed7b5016ad10780ab5913664dc2194649641c0be0f6384558788450b5b`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/jim-math-hooks.txt). SHA-256 `f90f2e106c90f9db8ae2fe642fc71c2c67b64293b5c805c3ce1d362757607ea2`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `scalar_number`: Original numeric preparation must preserve reached custom operand callbacks independently from the later math result.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
