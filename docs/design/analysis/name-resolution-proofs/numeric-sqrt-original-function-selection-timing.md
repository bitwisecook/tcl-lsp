# naming.numeric.sqrt-original-function-selection-timing

Kind: `native-observation`

## Problem statement

An operand update/free callback can replace sqrt itself while the entered expression is preparing its original argument. The chosen function may already be selected; a later global replacement is not proof that this same call used the new worker.

## Question

After original numeric operand callbacks replace sqrt with a handler returning99, which sqrt result is returned by compiled and generic expr routes?

## Conclusion

All five C releases return their original sqrt(3) result rather than99 while keep confirms UPDATED/FREED. Both compiled and alias expr routes therefore preserve the observed original selected function for this call despite the reached replacement. C8.4 replaces its registered math function and later C replaces ::tcl::mathfunc::sqrt. This narrow timing observation does not imply general command selection order, no callback effects, or a successful Rust rewrite.

## Scope

Four original custom operand route/hook windows per C release with exact replacement-handler99 controls. C8.4 public Tcl_CreateMathFunc replacement versus C8.5+ object command delete/create are distinct source branches. Full embedded captured source/output/library/executable associations/run0 retained; header/compiler/configure/runtime patch query absent. Jim and BIG-IP are not tested for this exact replacement-timing question.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=a294211c3298d0deebcce58fcd8864516f968b22137f2fac716b90db159556f1; library_sha=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.73205080757 UPDATED
route=compiled hook=free code=0 result=1.73205080757 FREED
route=generic hook=update code=0 result=1.73205080757 UPDATED
route=generic hook=free code=0 result=1.73205080757 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=dd64fa4cb0aaeda7301bbb23a514e763df134920edc158952cf901fd966b8aa4; library_sha=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.7320508075688772 UPDATED
route=compiled hook=free code=0 result=1.7320508075688772 FREED
route=generic hook=update code=0 result=1.7320508075688772 UPDATED
route=generic hook=free code=0 result=1.7320508075688772 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=10feeae675ed218bee4a3315c6c9c807a0fb03f49586045c15fb0851680c3ea3; library_sha=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.7320508075688772 UPDATED
route=compiled hook=free code=0 result=1.7320508075688772 FREED
route=generic hook=update code=0 result=1.7320508075688772 UPDATED
route=generic hook=free code=0 result=1.7320508075688772 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=1df8e619a1a3dd18aba82d654757d61b6dc0998f9adf09a64dbd11edee887c9e; library_sha=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.7320508075688772 UPDATED
route=compiled hook=free code=0 result=1.7320508075688772 FREED
route=generic hook=update code=0 result=1.7320508075688772 UPDATED
route=generic hook=free code=0 result=1.7320508075688772 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: binary_sha=9851ba0a69ee06444e0bd8665819daf86b5c9bbc463efdc0644c8dfdae6e9b05; library_sha=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.7320508075688772 UPDATED
route=compiled hook=free code=0 result=1.7320508075688772 FREED
route=generic hook=update code=0 result=1.7320508075688772 UPDATED
route=generic hook=free code=0 result=1.7320508075688772 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing.c). SHA-256 `51836eea19f0e4a025a169f06c368cbf7da6fff39c8e1471c7269e0682ba2a90`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-manifest.json). SHA-256 `f7edbd8b16833aaafb6031e6c07eca23b080cf1566f5bc6ecf757ed779508c36`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.4.20.txt). SHA-256 `53d9eae361301f97342daf050a779bf87131c7889ad27d652a894019fe5926b9`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.5.19.txt). SHA-256 `c7a60be13eb7fa16a0c22db2ca6b2f18fae66e6475a24a33e2edc6e72d503256`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.6.18.txt). SHA-256 `c7a60be13eb7fa16a0c22db2ca6b2f18fae66e6475a24a33e2edc6e72d503256`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.0.4.txt). SHA-256 `c7a60be13eb7fa16a0c22db2ca6b2f18fae66e6475a24a33e2edc6e72d503256`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.1.0.txt). SHA-256 `c7a60be13eb7fa16a0c22db2ca6b2f18fae66e6475a24a33e2edc6e72d503256`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `scalar_number`: Separates original operand conversion effects from the independently selected entered function recipe.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
