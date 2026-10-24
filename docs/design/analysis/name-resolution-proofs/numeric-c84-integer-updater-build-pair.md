# naming.numeric.c84-integer-updater-build-pair

Kind: `native-observation`

## Problem statement

Integer updater authority depends on separately pinned native producer builds and reached objects, not a release label or shared header alone. The complete finite output and the actual archive versus loaded-image producer must be retained independently.

## Question

Does the unchanged original C8.4 integer-updater probe produce byte-identical complete 812-row observations from the separately pinned archive and its independently linked/loaded image?

## Conclusion

All four actual link/compile/archive-probe/loaded-image-probe commands exit 0 with empty stderr. Both complete 812-row outputs are byte-identical to the unchanged observations.tsv SHA cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c. For each exact fresh long/wide specimen the measured type name stays unchanged, the exposed string-bytes flag changes0→1, errno is unchanged, and the complete counted byte result remains recorded. The archive SHA 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47, header SHA 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf, linked image SHA 66b79b3754bae00916073872f0eb2e812c45dd21cb24be0d092d6e17820c53c2 and probe SHA ec96f46e8f86c1dc7ca87506214fa33c18c30f4afdd0365452dade027e4f1c24 remain distinct identities. Equal finite outputs do not establish universal integer formatting, another archive/image, ABI compatibility, native object identity across producers, compiler/frame/admission or an executed Rust pass. C8.5/C8.6/C9.0/C9.1/Jim/BIG-IP are not tested.

A separately measured GNU17 build has archive SHA `dc67eba353eeb83cbfb9ce5e0f6ff52972297374799156c09cef1ac06cc5cfff`, the same exact header SHA `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf`, configuration SHA `33d929abbd1102f01624ca3aa217af1bcdc7e6023be2ea094e82e5228f8880e9`, and independently linked/loaded image SHA `b381688d8da3cee9d558eef7a38503980a10635782a28f56141c7e5bc503892f`. Its archive and loaded-image probes each preserve all 812 original rows byte-for-byte. This directly measured pair is eligible for exact build admission; the Rust loader/updater test remains a separate execution obligation. The older `44b069f1` archive is a different build, its mismatch cause remains unknown, and neither that build nor a future different build is admitted by source/configuration agreement.

A marked pure software control links all three complete digest pairs and mismatch/empty refusals to the formatter build-admission owner.

## Scope

The unchanged original probe.c SHA 2c1e656d0a1bdc6a86f84a08a8601cb414eef16ce7f53ec9b1d882c99f91ff6e creates fresh public Tcl long/wide objects, attaches each to its interpreter result, reads exposed Tcl_Obj type/string fields around reached Tcl_GetStringFromObj, and records pre/post errno and counted result hex. One probe uses linked archive functions; the other independently dlopens the exact image and obtains the stated public symbols with original flags. The version label identifies the pinned source/header/build; this original probe does not query a runtime version. No Tcl_Eval source/channel or private bytecode claim is involved.

This definition performs no native linking, ABI loading or getter/updater call; source linkage proves neither library availability nor passing assertion. The two original812-row updater operations and their separate build identities remain the native evidence. Admission-table membership alone supplies no modern release, foreign image, numeric formatter or default C ABI authority.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual original link/compile commands and all required archive/header/source/build/input pins verified; separately captured loaded image/probe bytes and digests retained. Original216 compiler version unrecorded; the separately measured GNU17 build records GCC 15.2.0-16ubuntu1. Channel: Original C API fresh Tcl_NewLongObj/Tcl_NewWideIntObj and reached Tcl_GetStringFromObj; direct archive versus original independently loaded image symbols. Dialect: Pinned original provider API/source purpose.

Archive and independently loaded image each return the unchanged complete 812 observations with process exit0/empty stderr. All type/string flag/errno/hex fields are retained byte-for-byte; finite equality grants no other build or universal formatting claim.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No execution answers this exact formatter build-pair question.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No execution answers this exact formatter build-pair question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No execution answers this exact formatter build-pair question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No execution answers this exact formatter build-pair question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No execution answers this exact formatter build-pair question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_integer_formatter216-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/archive.receipt.json). SHA-256 `717e5f374476bc2071889084279b1337a16d825197b4821e21b7f1222fc21e46`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/archive.stderr](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/archive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/archive.stdout](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/archive.stdout). SHA-256 `cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/compile.receipt.json). SHA-256 `3259f6f401cd5a53b540c759f43549eebb2f5cc88f50db29d2cce952ef9f9388`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/compile.stderr](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/compile.stdout](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-libtcl8.4-observed.so` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/libtcl8.4-observed.so](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/libtcl8.4-observed.so). SHA-256 `66b79b3754bae00916073872f0eb2e812c45dd21cb24be0d092d6e17820c53c2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-link.receipt.json` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/link.receipt.json](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/link.receipt.json). SHA-256 `78a42225995b57844f93570ded1bbd53a5f393a8d92015cc716750ae3e89f793`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-link.stderr` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/link.stderr](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/link.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-link.stdout` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/link.stdout](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/link.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-loaded.receipt.json` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/loaded.receipt.json](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/loaded.receipt.json). SHA-256 `a3d91f9c6cfd655a3e9b26ea9b906fce4f35e478cc19330af8c9e80aed00a4ac`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-loaded.stderr` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/loaded.stderr](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/loaded.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-loaded.stdout` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/loaded.stdout](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/loaded.stdout). SHA-256 `cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-original-observations.tsv` (observation): [rust/tcl-registry/tests/data/native_integer_formatter216/original-observations.tsv](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/original-observations.tsv). SHA-256 `cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-probe` (provider): [rust/tcl-registry/tests/data/native_integer_formatter216/probe](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/probe). SHA-256 `ec96f46e8f86c1dc7ca87506214fa33c18c30f4afdd0365452dade027e4f1c24`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-probe.c` (input): [rust/tcl-registry/tests/data/native_integer_formatter216/probe.c](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/probe.c). SHA-256 `2c1e656d0a1bdc6a86f84a08a8601cb414eef16ce7f53ec9b1d882c99f91ff6e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_integer_formatter216-request.json` (input): [rust/tcl-registry/tests/data/native_integer_formatter216/request.json](../../../../rust/tcl-registry/tests/data/native_integer_formatter216/request.json). SHA-256 `1b73ed27bbbbca2b1452a1795649538828a6fea9b692da599b5dc009b3dc742e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

### GNU17 archive and loaded-image evidence

The [complete original ZIP](https://api.github.com/repos/bitwisecook/tcl-lsp/actions/artifacts/12539375964/zip) from [run 40039950003](https://github.com/bitwisecook/tcl-lsp/actions/runs/40039950003), job `120708423833`, is 5,539,802 bytes with SHA-256 `a01b9f403fc3cf90e87551217bf2c9efcbdfcd485d81e8f5ec13e1d6a11076b1`. Its source is exactly `b52ff0b6c7ef223da6b920031be4d6b314346311`. This is one retained archive of all 62 original members; large maps are read in the ZIP without changing their bytes. The [unchanged original probe source](https://github.com/bitwisecook/tcl-lsp/blob/b52ff0b6c7ef223da6b920031be4d6b314346311/rust/tcl-host-c-abi/testdata/native_integer_formatter/probe.c) has SHA-256 `2c1e656d0a1bdc6a86f84a08a8601cb414eef16ce7f53ec9b1d882c99f91ff6e`.

| Original ZIP member | SHA-256 | Scope |
| --- | --- | --- |
| `native/provider-libtcl8.4.a` | `dc67eba353eeb83cbfb9ce5e0f6ff52972297374799156c09cef1ac06cc5cfff` | Actual archive, distinct from earlier builds |
| `native/provider-tcl.h` | `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf` | Exact public source header |
| `native/provider-tclConfig.sh` | `33d929abbd1102f01624ca3aa217af1bcdc7e6023be2ea094e82e5228f8880e9` | Actual provider configuration |
| `native/provider-tclObj.c` | `6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c` | Original updater source |
| `native/libtcl8.4-observed.so` | `b381688d8da3cee9d558eef7a38503980a10635782a28f56141c7e5bc503892f` | Independently linked and loaded image |
| `native/probe` | `6e920dd85a7de738463313be1fe801e60b2777d01e545b61624255d2b2120889` | Actual original probe executable |
| `archive.stdout` and `loaded.stdout` | `cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c` each | Each complete 812-row output, 50,944 bytes |
| `link.process.json` | `908e8e114cf174790fed16881e97ff2a79ae68788ec21c4c7a70ead7f0ae88d2` | Actual image producer, argv and lifecycle |
| `compile.process.json` | `a233cdbeb0ff952f6f74321a83d55e7643d88672a1b3638ecfa9bac199567f8a` | Actual probe producer, argv and lifecycle |
| `archive.process.json` | `80c0343e5deabed0524b8bb170b0e4fa227d8c5714b9e9bd3605c7bd44ea11af` | Original archive observation command |
| `loaded.process.json` | `e4805bf58abdf8d3a55f6079b3936732bdbe5ef247f99c5ede726e6f7bf2fe74` | Original independently loaded observation command |
| `provider-before.json` and `provider-after.json` | `e6815ffe105b7f7a3c0624328bb1bda3067ed7832bd0290cf9a8ed02dde7bb73` each | All 843 provider file identities unchanged |
| `tools-before.json` and `tools-after.json` | `6ff9704653e200fa5387bf2b234a3f711b39423520f1f4d71172923d90b52c9e` each | Actual tool identities unchanged |
| `tracked-source-before.json` | `65fa77d0d24c04317982748be31399d6aef027a345907c5853a6b6f19b6136df` | Complete 125,491-path source snapshot |
| `source-after.stdout` | `d740221ffd496cb864ab8e0e8da5991e8ffb296623106c9a0b87f1dcc1b8ad00` | Strict complete source verification |

All four original link, compile, archive-observation and loaded-image-observation commands exit zero with empty stderr; both native observations match the unchanged complete original fixture. All 62 ZIP members have CRC and streamed SHA checks, source membership joins the exact Git tree, and all 14 owned processes close with empty process groups and complete streams. The provider bootstrap separately retains 36,414 bytes of compiler warnings in `bootstrap.stderr`; the empty-stderr statement applies only to the four original experiment commands. Archive object timestamps and ownership fields are zero in this build, but this does not establish why an older archive differs. No Rust loader/updater pass, other native build, universal integer conversion, compiler admission, full runtime gate or performance result follows from these finite original observations.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `native_integer_formatter::tests::loaded_c84_updater_matches_812_original_archive_and_image_windows` (linked): Strict admitted archive/header pair and independently loaded image compare all 812 unchanged original type/string-flag/errno/counted-byte rows. Linked test source supplies no executed pass or authority for another build/header alone.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `admitted_reference_build`: Match the complete recorded archive/header pair rather than admitting a formatter from a release or either digest alone.
- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `load_pinned_c84_integer_formatter`: Independently verify complete build inputs, produce a linked image, recheck the inputs and supply that separate image digest to the actual ABI loader; table admission is only one prerequisite.
- [rust/tcl-test-support/src/native_integer_formatter.rs](../../../../rust/tcl-test-support/src/native_integer_formatter.rs), `native_integer_formatter::tests::formatter_admission_requires_a_complete_verified_build_pair` (linked): A pure admission-table control accepts each of three exact original archive/header digest pairs, rejects either substituted digest and rejects the empty pair. It does not run either812-row native updater experiment, link a formatter, load an image or call the ABI.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. The earlier Original216 inherited environment/compiler version are unrecorded; the GNU17 ZIP separately records its compiler and tool identities. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
