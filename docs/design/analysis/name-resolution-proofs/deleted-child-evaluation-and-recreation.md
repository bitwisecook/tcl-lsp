# naming.interpreter.deleted-child-evaluation-and-recreation

Kind: `native-observation`

## Problem statement

A strong activation lease preserves an old interpreter allocation while its deleted flag still forbids later evaluation.

## Question

What completion does a deleted active child produce, and does a same-name recreated child retain the old allocation or variables?

## Conclusion

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

## Scope

Exact counted ASCII source passed to Tcl_EvalEx or original Jim_NewStringObj/Jim_EvalObj on a fresh interpreter per source control. C public active-allocation control preserves Tcl_GetSlave actual interpreter through deletion. Finite cases: v1:active-child-script-recreation, v2:active-child-continuation-errors, v2:active-child-replacement-survives-error, v1:active-original-allocation, v2:active-original-allocation. Result primary/resident fields precede the public getter; retained fields do not independently prove object release/callback closure. No Rust execution or whole lifecycle Normal is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 509624f56dea942795cd07b532ea4613cd0363997110bcfd30985424b2ffc013 linked to library SHA256 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47 and header SHA256 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 41a74a750080e2791043e268b93669648560ddf9aa2725980106f78d1b4f34b3 linked to library SHA256 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc and header SHA256 c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 f0538fc24be66e978ea8415681f801cce18ab296144df4bfe2e4cda86e506071 linked to library SHA256 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb and header SHA256 aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 785e48c945aab19ea93ade52750dabf662265ba78c1b3a8f8a62997589eb5496 linked to library SHA256 dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4 and header SHA256 eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 9c84565330cbb35ae4b38b92e5d3caf11c5815a720f2a16ca8a3ebb361eb29de linked to library SHA256 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db and header SHA256 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; compiled/exited0.. Channel: Counted ASCII Tcl_EvalEx source plus separately preserved C public child API control. Dialect: Tcl.

All five C providers return error 1 and attempt to call eval in deleted interpreter when the retired child continues. The parent owns a distinct recreated child whose before variable is absent. The preserved C public API control independently records old deleted=1, replacement present=1, different allocation=1 and after-return result 1 0. C8.4 exposes no public GetReturnOptions here;85+ retain the exact error options. An activation lease grants storage lifetime only.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 fd015db448138b2bdfe3a5375caf4694ec48ca3851042ba63f6043333384431a linked to library SHA256 a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da and header SHA256 d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; compiled/exited0.. Channel: Original counted Jim string object passed to Jim_EvalObj; C API explicitly not tested. Dialect: Jim Tcl.

Actual C-shaped interp source controls each return1/wrong # args: should be "interp". The C public active-allocation API is explicitly not tested; no corresponding child behavior is inferred.

### bigip

Status: `not-tested`. Version: not recorded. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance observation.

## Exact evidence

- `v1-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/probe.c). SHA-256 `5463f0f9a49c2ab6bca0629260fb25fb5cc98942c970d2033d1709037217b04b`. Exact retained v1 protocol/probe.c; each guest code is independent of compile/process exit.
- `v1-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/inputs.json). SHA-256 `d0cd61c12de20a0e1c9a1a400ef684ef48a34057a954f231ee288c2729576f3b`. Exact retained v1 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v1-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/protocol/cases.h). SHA-256 `9aee94cdb78c410a747d6ab88d2785530cd9d34ca3f793b8cb04903379fdbaa0`. Exact retained v1 protocol/cases.h; each guest code is independent of compile/process exit.
- `v1-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/receipt.json). SHA-256 `c7f20c3bab500fd90ba0b3fafae616f9c8e6827e02f6f551e8d06ecf90b138bc`. Exact retained v1 capture/receipt.json; each guest code is independent of compile/process exit.
- `v2-protocol-probe.c` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/probe.c). SHA-256 `5463f0f9a49c2ab6bca0629260fb25fb5cc98942c970d2033d1709037217b04b`. Exact retained v2 protocol/probe.c; each guest code is independent of compile/process exit.
- `v2-protocol-inputs.json` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/inputs.json). SHA-256 `6dc151218728f50a00252ea2dab4274e40119f4c030d00d037b55231428d3af8`. Exact retained v2 protocol/inputs.json; each guest code is independent of compile/process exit.
- `v2-protocol-cases.h` (input): [runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/protocol/cases.h). SHA-256 `5aded0675a2b98fd13f29f4f7336fdf4615336850408e1b34318dc3185ebb36a`. Exact retained v2 protocol/cases.h; each guest code is independent of compile/process exit.
- `v2-capture-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/receipt.json). SHA-256 `8e924859f976ccc7c1e4e453e5954d5ca99b360d9ac995ab84364fc0e04c9eca`. Exact retained v2 capture/receipt.json; each guest code is independent of compile/process exit.
- `v1-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/receipt.json). SHA-256 `b07faf62c88aade5b504df6a76e036bcdc5806b39c1ce4fc9c9144322978066b`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stdout.tsv). SHA-256 `6ca2d0323499b81e5b0ed115f86338a0de27be2abbd9839e32df96a660b5f064`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/receipt.json). SHA-256 `6e2f488c1087f2588b41da174a63c51d9af3441b8a07bcde43e11e09215e2dbb`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stdout.tsv). SHA-256 `68f1ba0fa17be64272f8aa15670e503447c14b5e5335722ea2440fe7b0aabf1e`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.4-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/receipt.json). SHA-256 `5686776c1b523407dbab8a21d02cd4968c7503a0369a6386d42e3a6937dca42c`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stdout.tsv). SHA-256 `0d1f4f8fa4f4d891bcc870295274bfdbe4c7918b31b0d0567fdac6d4f077c606`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/receipt.json). SHA-256 `41d750abf06c81d6a217c7a6cbc1d0327acc29a6b223f3f9cfc15977d396ec1f`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stdout.tsv). SHA-256 `61eb1c57f65ba1675789bf3db2b7aab4ee79bc9f082b32650b596d3669b6634b`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.5-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/receipt.json). SHA-256 `baa4851657c450720005c7ad4de6583d99896f5d25fb3f136eca9a3865b8c5ca`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stdout.tsv). SHA-256 `f7732d84297c80bf046ea41ebafd4db31fb4467cfed61d7fe1f1652dd2a99655`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/receipt.json). SHA-256 `20d2e9cfeb866c5311438c40cefb94c4a0f91908e7a7056bffb39a2df1b78fcf`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stdout.tsv). SHA-256 `6c863d9d8fa2aa94a384ad988da585fc6234dc408523261a03b147ce843218b7`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl8.6-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/receipt.json). SHA-256 `995767a2b44029bebd1a7ea920c3b6f7ee578298e1b076596badd009ec5ad3fd`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stdout.tsv). SHA-256 `e7e21eb4eddbff5feacaa083629ed0fa56d3b68dab2ba21940a0563f216074f1`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/receipt.json). SHA-256 `6aed0c39ed050a2888da8187b526442565b35cc9c5a7b1fe9bea3143176760ee`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stdout.tsv). SHA-256 `4276877573d851659dbfde07c5ac18f28f3369dd81c3329ecc6221fb0a87a773`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.0-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/receipt.json). SHA-256 `ee8889ac77f13992d05d329124288d8023db67e773b277d548615983af2fb02c`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stdout.tsv). SHA-256 `b32896666068f647d6b0fad812a204aeb03adcc7cfce0e0438a620f2deb715b9`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/receipt.json). SHA-256 `a272b31e65048d6e6809b86e2495f97c1dc5ed18de9515a3c92bef4e8a08e576`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stdout.tsv). SHA-256 `f992a3ca450d11422b0583a6a6cceb4d5b1529a0bb259d0afeaa5c81e675e479`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-tcl9.1-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/receipt.json). SHA-256 `263fdc06d03fcd5fe037ce5ecaf0ab0f0155afe56e0b8eaa47db55b62f298dab`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stdout.tsv). SHA-256 `7873460480ac94b55acabd91f2094a5548ee79f10a2667c812de6b9f03c7e319`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v1-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v1/capture/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-receipt.json` (provider): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/receipt.json). SHA-256 `9d7a544bd2e6c67684814470aeb6f773c8a681294afca630255511621a76b55e`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stdout.tsv` (observation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stdout.tsv). SHA-256 `bebe4ef00ab531c00595e3e3a4fa4ee4fca155aa0040dbd8950095b8d10b5e00`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `v2-jim-stderr` (limitation): [runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr](../../../../runtime/rust/tests/data/native_child_command_lifetime/v2/capture/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual provider-associated capture. The receipt pins probe, input, header, static library, Makefile, compiler command and executable; TSV retains separate guest code/result/options and allocation booleans.
- `tcl8.4-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.4-basic-source.json). SHA-256 `360d8a0e8068b1f01fda3c228e6503d0c92ea663bf2508b950f366d8c2ff81c3`. JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.5-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.5-basic-source.json). SHA-256 `038c65927f8425528e82b4c151de2084c634f72beac70010404ec1105df7e5d1`. JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl8.6-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl8.6-basic-source.json). SHA-256 `7735f85379e09a724812bda9b9748bdadaf3e870e3a854ae1a9c098c9ec030e2`. JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.0-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.0-basic-source.json). SHA-256 `4f9d6f14cbef7fcf3b8a0c3b52377638ff31cdb3420226445f3a8b1822fcb303`. JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.
- `tcl9.1-deleted-eval` (source-anchor): [runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json](../../../../runtime/rust/tests/data/native_child_command_lifetime/tcl9.1-basic-source.json). SHA-256 `179ce33f3a1d96373679011fe69c74511e73fccee7389b6022e13d4904ad0a1f`. JSON pointer `/windows/1/snippet`. Pinned source-only C API publication or deleted evaluator ready check; no extra execution is inferred from this excerpt.

## Source inspection

tcl8.4 8.4.20, revision `8.4.20`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `TclInterpReady`, lines 3062–3072. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `2c1753fda0256dfceac01a939c3bb09adceed83c90503093e777e5d1d4139b05`; retained evidence `tcl8.4-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */
    
    if (iPtr->flags & DELETED) {
	Tcl_ResetResult(interp);
	Tcl_AppendToObj(Tcl_GetObjResult(interp),
	        "attempt to call eval in deleted interpreter", -1);
	Tcl_SetErrorCode(interp, "CORE", "IDELETE",
	        "attempt to call eval in deleted interpreter",
		(char *) NULL);
	return TCL_ERROR;

```

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `TclInterpReady`, lines 3498–3507. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `93983b91f5a054202be355660585540da832113b10a9a55782ab2e0ee0121165`; retained evidence `tcl8.5-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp,
		"attempt to call eval in deleted interpreter", NULL);
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", NULL);
	return TCL_ERROR;

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclInterpReady`, lines 3935–3943. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `d7c8a1e93d0e5966bbbd6246ffac2fce069def4f4a6e19482849bf6671c7aa04`; retained evidence `tcl8.6-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", -1));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclInterpReady`, lines 4161–4169. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `dedbce4cb5c861b3f1cb1ec93546ab316bfdd2fd54bf2c281bca06f007790823`; retained evidence `tcl9.0-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclInterpReady`, lines 4131–4139. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `dedbce4cb5c861b3f1cb1ec93546ab316bfdd2fd54bf2c281bca06f007790823`; retained evidence `tcl9.1-deleted-eval`.

```text
     * If the interpreter has been deleted, return an error.
     */

    if (iPtr->flags & DELETED) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"attempt to call eval in deleted interpreter", TCL_INDEX_NONE));
	Tcl_SetErrorCode(interp, "TCL", "IDELETE",
		"attempt to call eval in deleted interpreter", (char *)NULL);
	return TCL_ERROR;

```


## Consumer bindings

- [rust/tcl-registry/src/native_eval_object.rs](../../../../rust/tcl-registry/src/native_eval_object.rs), `NativeEvalObjectProtocol::deleted_interpreter_error`: Select only the C ready-check error message/code for a independently deleted actual interpreter.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `Interp::evaluation_is_live`: Keep actual activation lifetime independent from permission for further evaluation.
- [runtime/rust/src/interp/native_children.rs](../../../../runtime/rust/src/interp/native_children.rs), `interp::native_children::tests::child_source_controls_match_native_creation_and_retirement_windows` (linked): Compare exact deleted active child code/result plus85+ errorcode options, and distinct recreated child liveness/variable absence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_child_command_lifetime/replay.py",
  "--verify-only"
]
```

Offline exact source/input/stream/receipt verification only. For new native launches use the retained original compile commands with fresh output directory and the exact source/header/static-library association. C8.4 return-options getter unavailable; Jim C public child API not tested; no BIGIP execution. Original v1 errors remain separate from v2 intermediate observations.
