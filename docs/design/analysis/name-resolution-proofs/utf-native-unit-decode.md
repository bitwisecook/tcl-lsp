# naming.utf.native-unit-decode

Kind: `native-observation`

## Problem statement

Forward decoding, isolated unit encoding and reverse boundary movement need not agree with strict Unicode geometry or each other for malformed and surrogate inputs. Source spans cannot borrow an unmeasured native byte boundary.

## Question

Which units and consumed widths does public Tcl_UtfToUniChar select for fourteen original counted byte sequences?

## Conclusion

C8.4/8.5 accept the measured overlong sequences and treat four-byte input as individual old units. C8.6 yields a surrogate pair for valid astral bytes; C9 yields one scalar and maps selected invalid standalone bytes such as80 to20AC. Raw zero decodes as unit0. These actual widths/units differ from strict UTF8 document validation.

## Scope

Fourteen exact native byte arrays passed directly to public C character APIs, including raw00, malformed continuations, overlong sequences, surrogate, astral/max/out-of-range scalar and truncated sequence. The probe bounds its loop by retained counted byte length but individual public decoders see the source's terminator. Five output/source/library/executable digests and run0 captured; headers/compiler status/flags/runtime patch query absent. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=c687ffcada782d2ca85ec61f922129773c9c263f846eedba3d74eb0a936da204; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays.. Dialect: C Tcl.

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=80:1,78:1 encoded=c28078 boundaries=0,1
case=3 input=9f78 units=9f:1,78:1 encoded=c29f78 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=1:2 encoded=01 boundaries=0,0
case=6 input=c1bf units=7f:2 encoded=7f boundaries=0,0
case=7 input=e08080 units=0:3 encoded=c080 boundaries=0,0,0
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=f0:1,9f:1,98:1,80:1 encoded=c3b0c29fc298c280 boundaries=0,0,0,3
case=10 input=f48fbfbf units=f4:1,8f:1,bf:1,bf:1 encoded=c3b4c28fc2bfc2bf boundaries=0,0,0,3
case=11 input=f4908080 units=f4:1,90:1,80:1,80:1 encoded=c3b4c290c280c280 boundaries=0,0,0,3
case=12 input=e282 units=e2:1,82:1 encoded=c3a2c282 boundaries=0,0
case=13 input=418080 units=41:1,80:1,80:1 encoded=41c280c280 boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=555da224d66004868ca3e3c95cf12e9af794f8726fa52301dc9d3e0985008adb; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays.. Dialect: C Tcl.

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=80:1,78:1 encoded=c28078 boundaries=0,1
case=3 input=9f78 units=9f:1,78:1 encoded=c29f78 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=1:2 encoded=01 boundaries=0,0
case=6 input=c1bf units=7f:2 encoded=7f boundaries=0,0
case=7 input=e08080 units=0:3 encoded=c080 boundaries=0,0,0
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=f0:1,9f:1,98:1,80:1 encoded=c3b0c29fc298c280 boundaries=0,0,0,3
case=10 input=f48fbfbf units=f4:1,8f:1,bf:1,bf:1 encoded=c3b4c28fc2bfc2bf boundaries=0,0,0,3
case=11 input=f4908080 units=f4:1,90:1,80:1,80:1 encoded=c3b4c290c280c280 boundaries=0,0,0,3
case=12 input=e282 units=e2:1,82:1 encoded=c3a2c282 boundaries=0,0
case=13 input=418080 units=41:1,80:1,80:1 encoded=41c280c280 boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=acc9768e9bb101b5a99e7738d42d2b374505ff2cc222a8cc2a2e896d143ef69c; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays.. Dialect: C Tcl.

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=80:1,78:1 encoded=c28078 boundaries=0,1
case=3 input=9f78 units=9f:1,78:1 encoded=c29f78 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=c0:1,81:1 encoded=c380c281 boundaries=0,1
case=6 input=c1bf units=c1:1,bf:1 encoded=c381c2bf boundaries=0,1
case=7 input=e08080 units=e0:1,80:1,80:1 encoded=c3a0c280c280 boundaries=0,1,2
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=d83d:1,de00:3 encoded=eda0bdedb880 boundaries=0,1,2,3
case=10 input=f48fbfbf units=dbff:1,dfff:3 encoded=edafbfedbfbf boundaries=0,1,2,3
case=11 input=f4908080 units=f4:1,90:1,80:1,80:1 encoded=c3b4c290c280c280 boundaries=0,1,2,3
case=12 input=e282 units=e2:1,82:1 encoded=c3a2c282 boundaries=0,0
case=13 input=418080 units=41:1,80:1,80:1 encoded=41c280c280 boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=b10513118f673c9ca9c855b005e22367f1d4c211d5371058992c76f404f1e6d6; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays.. Dialect: C Tcl.

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=20ac:1,78:1 encoded=e282ac78 boundaries=0,1
case=3 input=9f78 units=178:1,78:1 encoded=c5b878 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=c0:1,81:1 encoded=c380c281 boundaries=0,1
case=6 input=c1bf units=c1:1,bf:1 encoded=c381c2bf boundaries=0,1
case=7 input=e08080 units=e0:1,20ac:1,20ac:1 encoded=c3a0e282ace282ac boundaries=0,1,2
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=1f600:4 encoded=f09f9880 boundaries=0,0,0,0
case=10 input=f48fbfbf units=10ffff:4 encoded=f48fbfbf boundaries=0,0,0,0
case=11 input=f4908080 units=f4:1,90:1,20ac:1,20ac:1 encoded=c3b4c290e282ace282ac boundaries=0,1,2,3
case=12 input=e282 units=e2:1,201a:1 encoded=c3a2e2809a boundaries=0,0
case=13 input=418080 units=41:1,20ac:1,20ac:1 encoded=41e282ace282ac boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=031b08c9fec2cef84f570f5569bec6b1719bd09345d8a3c98800ee4f0d58af36; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays.. Dialect: C Tcl.

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=20ac:1,78:1 encoded=e282ac78 boundaries=0,1
case=3 input=9f78 units=178:1,78:1 encoded=c5b878 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=c0:1,81:1 encoded=c380c281 boundaries=0,1
case=6 input=c1bf units=c1:1,bf:1 encoded=c381c2bf boundaries=0,1
case=7 input=e08080 units=e0:1,20ac:1,20ac:1 encoded=c3a0e282ace282ac boundaries=0,1,2
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=1f600:4 encoded=f09f9880 boundaries=0,0,0,0
case=10 input=f48fbfbf units=10ffff:4 encoded=f48fbfbf boundaries=0,0,0,0
case=11 input=f4908080 units=f4:1,90:1,20ac:1,20ac:1 encoded=c3b4c290e282ace282ac boundaries=0,1,2,3
case=12 input=e282 units=e2:1,201a:1 encoded=c3a2e2809a boundaries=0,0
case=13 input=418080 units=41:1,20ac:1,20ac:1 encoded=41e282ace282ac boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/tcl-character-units-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/tcl-character-units-probe.c). SHA-256 `c6ac9da843f7f584a9f324879eba8c5070c1546ff9b10956bdfa7cbeb4a414cd`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-manifest.json). SHA-256 `dd298ee27c000f15669f25260924e997aeebaf985934864c8055d9e078a9e2d8`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.4.20.txt). SHA-256 `6a83ceeff99204e5605dc0967f54d04993c1be4681155f0609bb63aa813d79b0`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.5.19.txt). SHA-256 `6a83ceeff99204e5605dc0967f54d04993c1be4681155f0609bb63aa813d79b0`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.6.18.txt). SHA-256 `6bc4c59684366dfe6aa32bc792d1804c445cb277c7f9679ba8e615fd3cd7cbdd`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.0.4.txt). SHA-256 `2294bb7ac85826ef3c0d81720847c8d5545a2b54750249e97209492d26ddf307`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.1.0.txt). SHA-256 `2294bb7ac85826ef3c0d81720847c8d5545a2b54750249e97209492d26ddf307`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_tcl_utf.rs](../../../../rust/tcl-syntax/src/native_tcl_utf.rs), `NativeTclUtf`: Owns canonical release-selected native unit decoding/encoding and reverse boundaries separately from document UTF.
- [rust/tcl-syntax/src/native_tcl_utf.rs](../../../../rust/tcl-syntax/src/native_tcl_utf.rs), `native_tcl_utf::tests::canonical_native_units_encoding_and_previous_boundaries_match_five_releases` (linked): Checks70 exact native decoder widths/unit/isolated encoder and every reverse boundary, retaining each independent field.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
