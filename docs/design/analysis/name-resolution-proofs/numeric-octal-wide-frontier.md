# naming.numeric.octal-wide-frontier

Kind: `native-observation`

## Problem statement

Invalid-octal wording depends on the reached primitive numeric state, not a generic string prefix. Raw-zero, sign, radix and trailing text controls must remain separate from expression parsing.

## Question

For twelve original sign/radix/invalid-octal spellings, what wide primitive/evaluated diagnostic and cache/errorCode frontier is reached?

## Conclusion

The twelve actual wide rows per release retain the exact invalid-octal/invalid-value/result differences, including source forms that fail before an implicit-octal interpretation and the counted raw-zero suffix. Only this selected primitive grammar and its propagated error are measured; no complete numeric parser equivalence is inferred.

## Scope

C public original-object wide getter is reached by one native callback from ASCII Tcl_Eval. Exact source bytes and selected storage/priming axes are retained; primitive result is duplicated before the callback returns, and final result/cache/errorCode/original input bytes are then sampled. Twelve fresh String cases, twelve rows per selected getter. Capture source/library/executable/row count/run0 are recorded; original C output checksums, headers/compiler status/configure/runtime patch query were not recorded by these receipts. Retained streams have current permanent evidence hashes. BIG-IP is not tested; Jim is only measured for seeded fresh Strings.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=7e033c9a994d416b57e1aae546ffe1ab88c74e4c88f45ed6a9c49f2c3fcf62b4; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=0 storage=0 case=0 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230387822 errorCode=4e4f4e45 input=303878 primitive=657870656374656420696e74656765722062757420676f74202230387822
getter=0 storage=0 case=1 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230382e307822 errorCode=4e4f4e45 input=30382e3078 primitive=657870656374656420696e74656765722062757420676f74202230382e307822
getter=0 storage=0 case=2 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f3822 errorCode=4e4f4e45 input=306f38 primitive=657870656374656420696e74656765722062757420676f742022306f3822
getter=0 storage=0 case=3 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f373822 errorCode=4e4f4e45 input=306f3738 primitive=657870656374656420696e74656765722062757420676f742022306f373822
getter=0 storage=0 case=4 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230784722 errorCode=4e4f4e45 input=307847 primitive=657870656374656420696e74656765722062757420676f74202230784722
getter=0 storage=0 case=5 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230623222 errorCode=4e4f4e45 input=306232 primitive=657870656374656420696e74656765722062757420676f74202230623222
getter=0 storage=0 case=6 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222d20303822 errorCode=4e4f4e45 input=2d203038 primitive=657870656374656420696e74656765722062757420676f7420222d20303822
getter=0 storage=0 case=7 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=4e4f4e45 input=2b303920 primitive=657870656374656420696e74656765722062757420676f7420222b3039202220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
getter=0 storage=0 case=8 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022303030387822 errorCode=4e4f4e45 input=3030303878 primitive=657870656374656420696e74656765722062757420676f742022303030387822
getter=0 storage=0 case=9 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038655822 errorCode=4e4f4e45 input=30386558 primitive=657870656374656420696e74656765722062757420676f7420223038655822
getter=0 storage=0 case=10 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038652b317822 errorCode=4e4f4e45 input=3038652b3178 primitive=657870656374656420696e74656765722062757420676f7420223038652b317822
getter=0 storage=0 case=11 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 errorCode=4e4f4e45 input=092d30390078 primitive=657870656374656420696e74656765722062757420676f742022092d30392220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=603b69848f9e3d2277cc2b5e6d2807f8771f50cedff2367343ec4a789ce52143; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=0 storage=0 case=0 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420696e74656765722062757420676f74202230387822
getter=0 storage=0 case=1 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420696e74656765722062757420676f74202230382e307822
getter=0 storage=0 case=2 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420696e74656765722062757420676f742022306f3822
getter=0 storage=0 case=3 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420696e74656765722062757420676f742022306f373822
getter=0 storage=0 case=4 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420696e74656765722062757420676f74202230784722
getter=0 storage=0 case=5 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420696e74656765722062757420676f74202230623222
getter=0 storage=0 case=6 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222d20303822 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420696e74656765722062757420676f7420222d20303822
getter=0 storage=0 case=7 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222b30392022 errorCode=54434c2056414c5545204e554d424552 input=2b303920 primitive=657870656374656420696e74656765722062757420676f7420222b30392022
getter=0 storage=0 case=8 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420696e74656765722062757420676f742022303030387822
getter=0 storage=0 case=9 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420696e74656765722062757420676f7420223038655822
getter=0 storage=0 case=10 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420696e74656765722062757420676f7420223038652b317822
getter=0 storage=0 case=11 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022092d303922 errorCode=54434c2056414c5545204e554d424552 input=092d30390078 primitive=657870656374656420696e74656765722062757420676f742022092d303922
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=aaab696bbf656aa36dce337a775fcadff6a923634f205c38a6692d2079ca3a5e; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=0 storage=0 case=0 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420696e74656765722062757420676f74202230387822
getter=0 storage=0 case=1 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420696e74656765722062757420676f74202230382e307822
getter=0 storage=0 case=2 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420696e74656765722062757420676f742022306f3822
getter=0 storage=0 case=3 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420696e74656765722062757420676f742022306f373822
getter=0 storage=0 case=4 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420696e74656765722062757420676f74202230784722
getter=0 storage=0 case=5 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420696e74656765722062757420676f74202230623222
getter=0 storage=0 case=6 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222d20303822 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420696e74656765722062757420676f7420222d20303822
getter=0 storage=0 case=7 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420222b30392022 errorCode=54434c2056414c5545204e554d424552 input=2b303920 primitive=657870656374656420696e74656765722062757420676f7420222b30392022
getter=0 storage=0 case=8 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420696e74656765722062757420676f742022303030387822
getter=0 storage=0 case=9 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420696e74656765722062757420676f7420223038655822
getter=0 storage=0 case=10 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420696e74656765722062757420676f7420223038652b317822
getter=0 storage=0 case=11 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022092d303922 errorCode=54434c2056414c5545204e554d424552 input=092d30390078 primitive=657870656374656420696e74656765722062757420676f742022092d303922
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=51f2f2d017a1be8cf7264a91559be37e9e274d5af87246e15ba57c6919097011; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=0 storage=0 case=0 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420696e74656765722062757420676f74202230387822
getter=0 storage=0 case=1 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420696e74656765722062757420676f74202230382e307822
getter=0 storage=0 case=2 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420696e74656765722062757420676f742022306f3822
getter=0 storage=0 case=3 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420696e74656765722062757420676f742022306f373822
getter=0 storage=0 case=4 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420696e74656765722062757420676f74202230784722
getter=0 storage=0 case=5 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420696e74656765722062757420676f74202230623222
getter=0 storage=0 case=6 code=1 cache=string message=657870656374656420696e74656765722062757420676f742061206c697374 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420696e74656765722062757420676f742061206c697374
getter=0 storage=0 case=7 code=0 cache=int message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=0 storage=0 case=8 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420696e74656765722062757420676f742022303030387822
getter=0 storage=0 case=9 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420696e74656765722062757420676f7420223038655822
getter=0 storage=0 case=10 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420696e74656765722062757420676f7420223038652b317822
getter=0 storage=0 case=11 code=0 cache=int message= errorCode=4e4f4e45 input=092d30390078 primitive=
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=ed465d42724dafce812e55a7da3490e3ed9cb0abfc5f9c3f54edfa0551aca3d6; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Public C original-object numeric getter reached from a native callback and ASCII Tcl_Eval; exact constructor and observer order in retained probe.. Dialect: C Tcl.

Exact relevant original rows:

```text
getter=0 storage=0 case=0 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230387822 errorCode=54434c2056414c5545204e554d424552 input=303878 primitive=657870656374656420696e74656765722062757420676f74202230387822
getter=0 storage=0 case=1 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230382e307822 errorCode=54434c2056414c5545204e554d424552 input=30382e3078 primitive=657870656374656420696e74656765722062757420676f74202230382e307822
getter=0 storage=0 case=2 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f3822 errorCode=54434c2056414c5545204e554d424552 input=306f38 primitive=657870656374656420696e74656765722062757420676f742022306f3822
getter=0 storage=0 case=3 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022306f373822 errorCode=54434c2056414c5545204e554d424552 input=306f3738 primitive=657870656374656420696e74656765722062757420676f742022306f373822
getter=0 storage=0 case=4 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230784722 errorCode=54434c2056414c5545204e554d424552 input=307847 primitive=657870656374656420696e74656765722062757420676f74202230784722
getter=0 storage=0 case=5 code=1 cache=string message=657870656374656420696e74656765722062757420676f74202230623222 errorCode=54434c2056414c5545204e554d424552 input=306232 primitive=657870656374656420696e74656765722062757420676f74202230623222
getter=0 storage=0 case=6 code=1 cache=string message=657870656374656420696e74656765722062757420676f742061206c697374 errorCode=54434c2056414c5545204e554d424552 input=2d203038 primitive=657870656374656420696e74656765722062757420676f742061206c697374
getter=0 storage=0 case=7 code=0 cache=int message= errorCode=4e4f4e45 input=2b303920 primitive=
getter=0 storage=0 case=8 code=1 cache=string message=657870656374656420696e74656765722062757420676f742022303030387822 errorCode=54434c2056414c5545204e554d424552 input=3030303878 primitive=657870656374656420696e74656765722062757420676f742022303030387822
getter=0 storage=0 case=9 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038655822 errorCode=54434c2056414c5545204e554d424552 input=30386558 primitive=657870656374656420696e74656765722062757420676f7420223038655822
getter=0 storage=0 case=10 code=1 cache=string message=657870656374656420696e74656765722062757420676f7420223038652b317822 errorCode=54434c2056414c5545204e554d424552 input=3038652b3178 primitive=657870656374656420696e74656765722062757420676f7420223038652b317822
getter=0 storage=0 case=11 code=0 cache=int message= errorCode=4e4f4e45 input=092d30390078 primitive=
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-octal-stage-probe.c). SHA-256 `a56ade349a9bdf1ae4ed4003a107529c81a6df114f13badefc78db89d24b2c6d`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-manifest.json). SHA-256 `6370e62c3ec2c4efe44a1d32159f33afb5466eefb268ef6d2f0c244a3f636e54`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.4.20.txt). SHA-256 `3bfdc1822860f80b7187db6926a30b27f8150f5b00fc48d43f557a0346c516aa`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.5.19.txt). SHA-256 `bf5a0c422954995b28a506e5600236ad64cf3d94b11d677f87f1f347920c9b2d`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-8.6.18.txt). SHA-256 `bf5a0c422954995b28a506e5600236ad64cf3d94b11d677f87f1f347920c9b2d`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.0.4.txt). SHA-256 `fa5c2e88796e58ee5b80e96dc1fc0f5c325af4842ef461bce7aff179825f699f`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/octal-9.1.0.txt). SHA-256 `fa5c2e88796e58ee5b80e96dc1fc0f5c325af4842ef461bce7aff179825f699f`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter/errors.rs](../../../../rust/tcl-syntax/src/scalar_getter/errors.rs), `NativeScalarGetterProtocol::failure_presentation`: Retains primitive versus evaluated diagnostic and selected error-code update independently from representation conversion.
- [rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests/error_tests.rs), `scalar_getter::tests::error_tests::invalid_octal_and_native_character_units_match_actual_releases` (linked): Compares only the exact selected primitive/propgated/error-code fields across retained rows; no native callback dispatch authority is supplied.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.
