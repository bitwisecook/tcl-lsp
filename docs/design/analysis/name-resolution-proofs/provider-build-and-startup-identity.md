# naming.native-replay.provider-build-and-startup-identity

Kind: `native-observation`

## Problem statement

A launcher filename or surviving transcript hash cannot identify the native executable that actually runs a proof. Modern compiler compatibility flags and Jim UTF-8 configuration can also change the build. A fresh build/version record must identify the actual provider separately from its naming observations.

## Question

Which exact native C Tcl and pinned Jim providers, configurations and original CLI version/patchlevel outputs are retained for reconfirming naming questions?

## Conclusion

All five C processes returned their recorded release patchlevels and the pinned Jim process returned 0.84 / 0.84-9-g5bac7c9 with UTF-8 enabled. The compile configuration, original probe bytes and separate status/stdout/stderr hashes are retained. This validates only the build/startup identity; it proves no naming behavior or parity with an older binary.

## Scope

These are actual current provider build/version captures under the recorded GCC14 configuration. Earlier failed configure/build attempts are not successful guest observations. C84 environment CFLAGS and prefix remain in its exact selected recipe. BIG-IP was not built or launched for this question. No Rust tests are executed by this record.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-tcl8.4. Channel: CLI stdin. Dialect: tcl8.4.

Actual process exit 0; stdout '8.4\n8.4.20\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["env", "CFLAGS=-O2 -Wno-error=implicit-int -Wno-error=implicit-function-declaration -Wno-error=incompatible-pointer-types", "./configure", "--cache-file=/dev/null", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/gcc14-compatibility/8.4.20/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-tcl8.5. Channel: CLI stdin. Dialect: tcl8.5.

Actual process exit 0; stdout '8.5\n8.5.19\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--cache-file=/dev/null", "--disable-shared", "--enable-threads", "CFLAGS=-O2 -Wno-error=implicit-int -Wno-error=implicit-function-declaration -Wno-error=incompatible-pointer-types", "--prefix=/workspace/.proofs/native-providers/gcc14-compatibility/8.5.19/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-tcl8.6. Channel: CLI stdin. Dialect: tcl8.6.

Actual process exit 0; stdout '8.6\n8.6.18\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/8.6.18/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-tcl9.0. Channel: CLI stdin. Dialect: tcl9.0.

Actual process exit 0; stdout '9.0\n9.0.4\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/9.0.4/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-tcl9.1. Channel: CLI stdin. Dialect: tcl9.1.

Actual process exit 0; stdout '9.1\n9.1.0\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--disable-shared", "--enable-threads", "--prefix=/workspace/.proofs/native-providers/9.1.0/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact retained configure/make logs and source/header/library/executable hashes at provider-jim. Channel: CLI stdin. Dialect: jim.

Actual process exit 0; stdout '0.84\n0.84-9-g5bac7c9\n'; stderr is empty. Compiler: cc (Debian 14.2.0-19) 14.2.0. Configure argv: ["./configure", "--prefix=/workspace/.proofs/native-providers/jimtcl/install"]. Make argv: ["make", "-j2"]. Source header/configure/library/executable identities remain exactly those recorded in the selected receipt. Jim revision 5bac7c99ad65864c87da513e22e2f01703fa4e03; UTF-8 enabled True; retained config hash d092046fe07c2ec14b77da74304135da24725d766e1c9fd74ee32eeb07702221. This startup identity is independent of any older Jim artifact hash.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This record did not build or launch the closed-source appliance. Its independent appliance reports are separate question evidence.

## Exact evidence

- `jim-config-h` (input): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-config.h](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-config.h). SHA-256 `d092046fe07c2ec14b77da74304135da24725d766e1c9fd74ee32eeb07702221`. Exact original provider build/version input, configuration or process stream.
- `jim-version-probe-tcl` (input): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-version-probe.tcl](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-version-probe.tcl). SHA-256 `84332730c215f593408e8fb3d47c5990d01f83cb721aa25bd43addafd703da16`. Exact original provider build/version input, configuration or process stream.
- `jim-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.configure.log). SHA-256 `8eca78c51d04546ef05afaaf9a8e9546dcf08771e0c70724529d6442e26f4b6d`. Exact original provider build/version input, configuration or process stream.
- `jim-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.make.log). SHA-256 `a276d7eb5ec1082a80a965218b0bf51e38559d493a69894c0b7542d7b5fc8218`. Exact original provider build/version input, configuration or process stream.
- `jim-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `jim-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim.stdout). SHA-256 `7998419aaf32a1309bdc3aae6f73bedcedc01fbd7e9048e10a1f17ecc4788e65`. Exact original provider build/version input, configuration or process stream.
- `receipt-json` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. Exact original provider build/version input, configuration or process stream.
- `tcl8-4-20-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.configure.log). SHA-256 `b2e5194736fa35965929852bc57bdff4f263c11f68d7fb65dec1998e703cbcb5`. Exact original provider build/version input, configuration or process stream.
- `tcl8-4-20-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.make.log). SHA-256 `8fb0a4bb03796f68d0b093c3b82d6c662239edc911e95455908879456d422320`. Exact original provider build/version input, configuration or process stream.
- `tcl8-4-20-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `tcl8-4-20-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.4.20.stdout). SHA-256 `18d2a10c48ef5e1b1590d78cf1b128abe21578d166c897c824737b66800bef9c`. Exact original provider build/version input, configuration or process stream.
- `tcl8-5-19-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.configure.log). SHA-256 `526ca4cb1997dd1951edd9c6f7a5a41aff539dd757455ee9e460df4084ad2987`. Exact original provider build/version input, configuration or process stream.
- `tcl8-5-19-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.make.log). SHA-256 `b7dddc05b34c8644d7e4aeecc346b0de2bc5373bfeec58011b1c8d56e1e3a78e`. Exact original provider build/version input, configuration or process stream.
- `tcl8-5-19-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `tcl8-5-19-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.5.19.stdout). SHA-256 `04473de8bb3aeaf3c38dfe932a8538588d405c14306da9756d988bacf0e124ad`. Exact original provider build/version input, configuration or process stream.
- `tcl8-6-18-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.configure.log). SHA-256 `705ba2ba4f569c027a540188e30e15b70759cf57815fb09f3635ae4564760f53`. Exact original provider build/version input, configuration or process stream.
- `tcl8-6-18-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.make.log). SHA-256 `90397ef9f59c453ee019a96c5f29cf57ac403e00505054ad8ca2606a152c368a`. Exact original provider build/version input, configuration or process stream.
- `tcl8-6-18-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `tcl8-6-18-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl8.6.18.stdout). SHA-256 `a29e02a9c1264e2dc6d2a8b6bdbfc0d633cd73e71cfbaaf8f0502f08122619e4`. Exact original provider build/version input, configuration or process stream.
- `tcl9-0-4-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.configure.log). SHA-256 `6eaf42effe93962033b5720d055e8904dd934359acdc0cef8390b91830f51891`. Exact original provider build/version input, configuration or process stream.
- `tcl9-0-4-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.make.log). SHA-256 `b367af1cb90b91e4b6495a28e30df18bff38ae3ac5b65d884b99527dad5f7881`. Exact original provider build/version input, configuration or process stream.
- `tcl9-0-4-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `tcl9-0-4-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.0.4.stdout). SHA-256 `76c2349a61397da4e30ef3c622a1903f980668373f78fe50cc0ce2d46d662976`. Exact original provider build/version input, configuration or process stream.
- `tcl9-1-0-configure-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.configure.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.configure.log). SHA-256 `33580369a26fbbe8448b361143d2bc6a0af06f08e883897027a7f11ed123f1f8`. Exact original provider build/version input, configuration or process stream.
- `tcl9-1-0-make-log` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.make.log](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.make.log). SHA-256 `41c66628c39dac2fd78114c97b50dac90aaba94b60fbe9a3b2d24273a809ad35`. Exact original provider build/version input, configuration or process stream.
- `tcl9-1-0-stderr` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.stderr](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider build/version input, configuration or process stream.
- `tcl9-1-0-stdout` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.stdout](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/tcl9.1.0.stdout). SHA-256 `ad656d15fcdc9c50a98e9c850f3c006258827324e2c2f18d239942e22d0a824d`. Exact original provider build/version input, configuration or process stream.
- `version-probe-tcl` (input): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/version-probe.tcl](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/version-probe.tcl). SHA-256 `ba81356322298d29b22f609d90a4afb9429c9d11058f2dc1196cb4c94f11b7ed`. Exact original provider build/version input, configuration or process stream.
- `provider-tcl8.4` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/0`. Exact build identity and actual CLI version/patchlevel query.
- `provider-tcl8.5` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/1`. Exact build identity and actual CLI version/patchlevel query.
- `provider-tcl8.6` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/2`. Exact build identity and actual CLI version/patchlevel query.
- `provider-tcl9.0` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/3`. Exact build identity and actual CLI version/patchlevel query.
- `provider-tcl9.1` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/4`. Exact build identity and actual CLI version/patchlevel query.
- `provider-jim` (provider): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/receipt.json). SHA-256 `07dc170bee43dd388c502a1420b052e20e29a8a11f76e48e33f000c1c1e9041b`. JSON pointer `/providers/5`. Exact build identity and actual CLI version/patchlevel query.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Reproduction requires the exact provider source trees/revision, selected compiler and configure/make argv. Run the retained version-probe input through the selected CLI stdin and independently compare status/stdout/stderr and build hashes. Configure/make logs are evidence, not a portable launcher script; no unrecorded native reconfirmation is inferred.
