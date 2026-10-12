# naming.expression.original-entered-command-error-stage

Kind: `native-observation`

## Problem statement

A direct expression API and an entered expr command can normalize error state at different boundaries even for the same original variable value. Result bytes alone cannot establish when the seeded code was replaced.

## Question

What result/code/options remain after entered expr {$v+1} inside the callback and after outer propagation?

## Conclusion

The six original values x,1-zero-X,1.5-zero-X,NaN,08,true reach distinct arithmetic/Boolean/entered-command outcomes by release. The matrix independently retains inside result, inside error code/return-options and later propagated code. C8.4 direct invalid arithmetic can retain PROBE BEFORE until outer evaluation changes it to NONE, while NaN domain state is already published. Full provider rows bound this conclusion to the selected API rather than the expression spelling alone.

## Scope

One retained original counted variable value per case enters the exact API indicated by route0/1/2. The callback saves its inside result and options/code before returning; outer Tcl_Eval then produces the propagated code. Eighteen rows per C release have exact source/library/output hashes, but compile status/executable/header/compiler/configure/runtime-version query are absent. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=1 code=1 inside=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=2 code=1 inside=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=2 case=4 code=1 inside=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=5 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=1 inside=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d insideOptionsCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d result=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d
route=2 case=5 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=1 inside=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d insideOptionsCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d result=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d
route=2 case=5 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=0 inside=39 insideCode=414253454e54 insideOptionsCode=414253454e54 result=39 propagatedCode=414253454e54
route=2 case=5 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation.. Dialect: C Tcl.

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=0 inside=39 insideCode=414253454e54 insideOptionsCode=414253454e54 result=39 propagatedCode=414253454e54
route=2 case=5 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/probe.c). SHA-256 `4d5a47a092dbfec1a2b810bad562757885de86f36510c839ed7b3500b416b4f4`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/manifest.json). SHA-256 `bb47d188ccf347a678bf3a4c9087579b30d93606a41c73fd5f094b6ae2b72924`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.4.20.txt). SHA-256 `b5b7a67079a66678e952dc4f31facbb2e1c45bc1c9f6bee014075ff934467835`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.5.19.txt). SHA-256 `d8d193480f3c3763898958cafc80d4d3d208a0a4d872e7f4ef5bb2c837188b8b`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.6.18.txt). SHA-256 `ec1be4791eb560e0f7a5c9267b8f4191ce8bf5f61929645d9d01dfb9d1c6cb36`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.0.4.txt). SHA-256 `afef4104c313d2d67a680d0eef5dacbdd8ff26b38617f3f5062d95986eecc229`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.1.0.txt). SHA-256 `afef4104c313d2d67a680d0eef5dacbdd8ff26b38617f3f5062d95986eecc229`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `report_cmd_error`: Keeps primitive/direct expression error publication separate from later entered script error normalization.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
