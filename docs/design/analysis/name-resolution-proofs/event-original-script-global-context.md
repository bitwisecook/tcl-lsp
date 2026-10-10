# naming.event.original-script-global-context

Kind: `native-observation`

## Problem statement

The old event adapters rendered script operands and evaluated callbacks in the caller frame.

## Question

In the retained basic after controls, which variable frame receives callbacks registered inside a namespace or procedure, and what do the selected concat/List scripts return?

## Conclusion

All six captured providers execute these callbacks against global variables: namespace-local x remains LOCAL while ::x becomes GLOBAL, and procedure-local x remains LOCAL while ::x becomes NEW. The concat and pure-List script controls return 1 and A B. Source inspection separates C single-script retention from Jim concat construction.

## Scope

Four exact ASCII counted source controls per provider in v106, plus the pinned callback/constructor source windows. Public code/result and available C return-options objects only; no custom object conversion, command observer, original header/cache or callback-free claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/GLOBAL_FROM_NAMESPACE: GLOBAL_FROM_NAMESPACE: code 0, bytes b'LOCAL GLOBAL' v106/GLOBAL_FROM_PROC: GLOBAL_FROM_PROC: code 0, bytes b'LOCAL NEW' v106/CONCAT_TRIM: CONCAT_TRIM: code 0, bytes b'1' v106/PURE_LIST_SCRIPT: PURE_LIST_SCRIPT: code 0, bytes b'A B'

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance build attached.. Channel: not exercised. Dialect: bigip.

No BIG-IP provider was executed or inspected for this Event question.

## Exact evidence

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e82bde6ea86354ca82c9b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `c9bb3d3237e07f468105f45ee54765683d4eda6b01eef418e794e42b037b6f43`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eab0ebdce23d2b8fb2e3d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `a89c6d1cb8ae90fecc0130c919e70bc8b090e17c22d9e4e363f794dbc521a586`. Unmodified original stdout, including partial rows when externally timed out.
- `e9a8002705e1a2b64d2a8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef4b434db4305c93fa04e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/receipt.json). SHA-256 `9259f39ddbf0ffb6182fdf32475e75e097dafa39ec2e45a50d24efc2c2bc746f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edc1e83aca47cc9392f36` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `ec584b2da5ae1bad016dbba80c19914cf5efe95d8d4b9edfdad44d0fb784300d`. Unmodified original stdout, including partial rows when externally timed out.
- `e640d06939660d2393b74` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ee1b7f7617169073897af` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/receipt.json). SHA-256 `4d06c0a040d1f84457e1f27eaccc12ffd3a029d7cc20b4fe0ad8de2d692714cd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e79205d2ae998d5a38673` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stdout.tsv). SHA-256 `95136ab28b008419347fa2447edac8db0c12af078f69dba789aa7f431dd38d3a`. Unmodified original stdout, including partial rows when externally timed out.
- `e3e06ba4dd754f84e7e64` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e29e3f7504be1e6186b5f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/receipt.json). SHA-256 `f0ca4661b3f175df84a2e79e5f4efb6d8122aab2dbffc902add86c7b9e90c356`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee08b9a64f471521ea7d6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `72b8194931f89f423409df205265f41d956c15875abe12bf87676a9a35ccb80d`. Unmodified original stdout, including partial rows when externally timed out.
- `e3e64357e5d9edc9603e2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e4da0dad00b144f2d84b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `69cb2b1e9c3b8b6f663403e31a7806cab666e3063b69039b03e780ecf2012051`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb458bd4525034a789ada` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `98cff810202805c4ac66b4dcc7a7a2ea1ec1f5b22e471c8372b23a6eaca38051`. Unmodified original stdout, including partial rows when externally timed out.
- `e03c642460973a4a71bf1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4cddff13a14363486cd6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/receipt.json). SHA-256 `44f66906998be36335d3ab91bf6b17cb151b35b207855dec5a64d12cfdc97f05`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e083744aa190aa6b11a05` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `4074052564d22e872b3cc63df8c5012842497161b4b9356700cb238bd32f5b3f`. Unmodified original stdout, including partial rows when externally timed out.
- `e328169da8d6e71512e9a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e00089d0781440c179788` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/receipt.json). SHA-256 `d8e8eb4103fade3f1434c155a4f88e679b8240ef541b32912fadec6850da9763`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ede77607a6e3e5b6fe12f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stdout.tsv). SHA-256 `b290e81b243c5c7fea638c28e6b2075417fcbb06c5a54e93a4d3ce593f3a967b`. Unmodified original stdout, including partial rows when externally timed out.
- `e51802ab802d88a1b81f0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eee602952d4b918098df6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/receipt.json). SHA-256 `8eba2f5f6a5c12c2422c8c31e37a29d087cc05de53949bea2a036da9c1e780e5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e65d8386702d49432e609` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `f7259975fbff26bd5d862f35af061a1e1fb8c230c6792e55524659e22f883736`. Unmodified original stdout, including partial rows when externally timed out.
- `e3d293e66456f98648886` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e3bf64c9cc0e463891bf7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `d4102a35655e703ba24d676f4e4376b9b2e34470981dd9b640fb2b6b4d27acbb`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e462e52fe279b3c913d8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `d78b0898e52bb8f2d4048e6668c4435a026d87d2421e50218d755a76cb56c661`. Unmodified original stdout, including partial rows when externally timed out.
- `ed1577e1a62eafcdb0bdb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eed23a20a0104b1261f17` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/receipt.json). SHA-256 `cfec91acc211b9c2e9ff11600e745a67e0fd935172883e0a844b6931a659d000`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e791339df6453253e3d7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `5858751924bd0f0897ca999c5238a5093aa90a28d7cc7f2287dfb56e35b99151`. Unmodified original stdout, including partial rows when externally timed out.
- `e20f480d24b4dc4267ff5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e9c2d2284dd4d892c5646` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/receipt.json). SHA-256 `fe5648bef42e86e45b8eec26a1505549dc38c59902f0825ac7800402cdd7a2da`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e31eb883b262af10420a2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stdout.tsv). SHA-256 `7e2388f059c1fa8b9198b151484d1378ab485121addef200d7a354e35ec19bfa`. Unmodified original stdout, including partial rows when externally timed out.
- `e976aff5ad43f905d94bf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e25f775b9b31952683c47` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/receipt.json). SHA-256 `d6c1b896bbe69252cb7f9401e69adfe2705792e9e39731706979008829971ebd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e00ad21bce1bde9752240` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `f2f769d85b1fbfb88f7fcc2d5da6ce78817cf3c688ea424dfd6c7428fb6a35b9`. Unmodified original stdout, including partial rows when externally timed out.
- `ee9fd72d3cf3f25042c64` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ead267513fde61d3a4089` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `8410cf510d37dd052539f3963df5674ad46f1f7dfab23c5cb07c6d815301d11d`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e78e165002788f2fae147` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `d79adf7cfe95b98dcb7924725db048542ece0aaf4533cb878e67484940522d7e`. Unmodified original stdout, including partial rows when externally timed out.
- `ed484834653d0e4f0ace8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e18bc58805088eae322ff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/receipt.json). SHA-256 `88cb506c0fbe899b17ded9a82ffe7bd69458967249570549d3bfd88ac5536639`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef22b225d0907df5e8f26` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `62573b2a7f5edc27b724199838e92228be596330dea8cb9931dcf61dc3148009`. Unmodified original stdout, including partial rows when externally timed out.
- `e893b9b51f2461fb33262` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e88a09d8915be94a2c28b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/receipt.json). SHA-256 `6a6bbbd77b6653ebace992da5dd203d34dce481813eb783da8a88e1b10eec924`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5473775471ca1fcdc422` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stdout.tsv). SHA-256 `b300c04d0ee734475e4e518a8ab88ac350f3f6606ff01d4278d80429175efe2c`. Unmodified original stdout, including partial rows when externally timed out.
- `eb4c690475a43a7f4e9ec` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e6eda56329e5344e77920` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/receipt.json). SHA-256 `d1ec83088cb32f153c22a375bd51657ec00b91deaeee521a5aa03c30a41d2335`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebe44981b65f5c67066ac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `c609f1283eae0901fbff3c17871098dc75069d2a2ca696b2a56b76cf06f21db6`. Unmodified original stdout, including partial rows when externally timed out.
- `ea3d77b738c2eb78ea1be` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e40b200d1cb7f0a117120` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `74583c62a9cded227d7f48d0fdddf35dc8c8b5bbb163caa6d4427a6cda3f2875`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e89934a3483a811078b8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `acb61aec08e5ddf10e7cc0b6ef6784c59fca8df7e92b67b9f228241115e7e1d9`. Unmodified original stdout, including partial rows when externally timed out.
- `e3a128d3c675c0b71d777` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ebbacadbe86729017ee83` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/receipt.json). SHA-256 `fecba330debf87bff0da13de7869471c97ca62c9455fba596951938d41f03ead`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e138c17fa020177b0236f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `417975325c65d2f7106b7bf47adfc1d05d49f1c956a8361a271cfc6ef6db3ebb`. Unmodified original stdout, including partial rows when externally timed out.
- `ec87f97a725ac3962e667` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1ccb3a915087d24f788d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/receipt.json). SHA-256 `fad3dead65742a3c742443013a9a43877a0ead20f14e7e6d4a579d5f48a2833f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e346125b0898fb3210e6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stdout.tsv). SHA-256 `a32136ba4a21681bd7b12ab8102189ebb744ff23be14fe471476016a5bfd5c3d`. Unmodified original stdout, including partial rows when externally timed out.
- `ede16036b2674c4976026` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e16c0a7f6b9fcc92dbd25` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/receipt.json). SHA-256 `7ea1581008c668ffac2f5ef1cbbb0c3ffec010314c26f3f4249c86cba1ca7ebc`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e207806f45de34a350562` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `a221bb6411f49cd7909bd064f8cabb1a02d8936ec54efefa665d76e8bf5b9c1f`. Unmodified original stdout, including partial rows when externally timed out.
- `e0e6ef567ac12a2a70bfe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e34c3b6b3352510dcef42` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/receipt.json). SHA-256 `052e94272bf006c86a9ad7337392bebfa5dfd18d3df7c83bfc7bbab002a3fb38`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec0aa600a32f71a8aba32` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stdout.tsv). SHA-256 `ebfc70360ae64be6fbeee574ce1d9d48bcffc0f8f64d2517d09e6f9ca9827445`. Unmodified original stdout, including partial rows when externally timed out.
- `e1a94d3ed5a18d3a8a3a0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea73d42750afc50e5d1fa` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/receipt.json). SHA-256 `4423963569ceb12bb8d785e2ae3a1def9c40a4053967d57ce092555209dde3a8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5da3ced5266ce42e8fff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stdout.tsv). SHA-256 `498f7b01792124fc81946d568bc4e48cd00560e1ec18d612272a4f61f95fcd40`. Unmodified original stdout, including partial rows when externally timed out.
- `e3c5e025f16621d82decf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e6fbc574f1a76a96d31c3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/receipt.json). SHA-256 `f44dadc0ac1ce03a5f7fcd2324f04928d2dc10a9fccf6001e8db70f3dfb453d5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e43776fb771dfd241990d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stdout.tsv). SHA-256 `f5e575d6a8a8ae4ae460961165162532e5bc5dff893ef940e253e1eb39e9ded2`. Unmodified original stdout, including partial rows when externally timed out.
- `e012199e98b3a08793a66` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5295b320aea9635c0347` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/receipt.json). SHA-256 `29ff76c88817ec995b6cb61d59235d89fb9b22711ec1edd0e49a69cc97d5acc8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1ee05298aff9dce615c2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stdout.tsv). SHA-256 `a6b3ecf6e31a00a83a681efd163356d6c07e0e21b26790a37ae133cb9a8f8570`. Unmodified original stdout, including partial rows when externally timed out.
- `e7ac7eda60b44ddcb74dd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 735–933. Full-source SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`; snippet SHA-256 `d585c94ea4dc36fc9759d20db15e4818509d5e7ca3cbd44a26ffb8dcd7e641fb`; retained evidence `e7ca34f2d6fd9344d4610`.

```text
Tcl_AfterObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Unused */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int ms;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    char *argString;
    int index;
    char buf[16 + TCL_INTEGER_SPACE];
    static CONST char *afterSubCmds[] = {
	"cancel", "idle", "info", (char *) NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter,
     * if it doesn't already exist.  
     */

    assocPtr = Tcl_GetAssocData( interp, "tclAfter", NULL );
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *) ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc,
		(ClientData) assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType) {
	ms = (int) objv[1]->internalRep.longValue;
	goto processInteger;
    }
    argString = Tcl_GetStringFromObj(objv[1], &length);
    if (argString[0] == '+' || argString[0] == '-'
	|| isdigit(UCHAR(argString[0]))) {	/* INTL: digit */
	if (Tcl_GetIntFromObj(interp, objv[1], &ms) != TCL_OK) {
	    return TCL_ERROR;
	}
processInteger:
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    Tcl_Sleep(ms);
	    return TCL_OK;
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
 	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	/*
	 * The variable below is used to generate unique identifiers for
	 * after commands.  This id can wrap around, which can potentially
	 * cause problems.  However, there are not likely to be problems
	 * in practice, because after commands can only be requested to
	 * about a month in the future, and wrap-around is unlikely to
	 * occur in less than about 1-10 years.  Thus it's unlikely that
	 * any old ids will still be around when wrap-around occurs.
	 */
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = Tcl_CreateTimerHandler(ms, AfterProc,
		(ClientData) afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	sprintf(buf, "after#%d", afterPtr->id);
	Tcl_AppendResult(interp, buf, (char *) NULL);
	return TCL_OK;
    }

    /*
     * If it's not a number it must be a subcommand.
     */

    if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "argument",
            0, &index) != TCL_OK) {
	Tcl_AppendResult(interp, "bad argument \"", argString,
		"\": must be cancel, idle, info, or a number",
		(char *) NULL);
	return TCL_ERROR;
    }
    switch ((enum afterSubCmds) index) {
        case AFTER_CANCEL: {
	    Tcl_Obj *commandPtr;
	    char *command, *tempCommand;
	    int tempLength;

	    if (objc < 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "id|command");
		return TCL_ERROR;
	    }
	    if (objc == 3) {
		commandPtr = objv[2];
	    } else {
		commandPtr = Tcl_ConcatObj(objc-2, objv+2);;
	    }
	    command = Tcl_GetStringFromObj(commandPtr, &length);
	    for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		tempCommand = Tcl_GetStringFromObj(afterPtr->commandPtr,
			&tempLength);
		if ((length == tempLength)
		        && (memcmp((void*) command, (void*) tempCommand,
			        (unsigned) length) == 0)) {
		    break;
		}
	    }
	    if (afterPtr == NULL) {
		afterPtr = GetAfterEvent(assocPtr, commandPtr);
	    }
	    if (objc != 3) {
		Tcl_DecrRefCount(commandPtr);
	    }
	    if (afterPtr != NULL) {
		if (afterPtr->token != NULL) {
		    Tcl_DeleteTimerHandler(afterPtr->token);
		} else {
		    Tcl_CancelIdleCall(AfterProc, (ClientData) afterPtr);
		}
		FreeAfterPtr(afterPtr);
	    }
	    break;
	}
	case AFTER_IDLE:
	    if (objc < 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "script script ...");
		return TCL_ERROR;
	    }
	    afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	    afterPtr->assocPtr = assocPtr;
	    if (objc == 3) {
 		afterPtr->commandPtr = objv[2];
	    } else {
		afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	    }
	    Tcl_IncrRefCount(afterPtr->commandPtr);
	    afterPtr->id = tsdPtr->afterId;
	    tsdPtr->afterId += 1;
	    afterPtr->token = NULL;
	    afterPtr->nextPtr = assocPtr->firstAfterPtr;
	    assocPtr->firstAfterPtr = afterPtr;
	    Tcl_DoWhenIdle(AfterProc, (ClientData) afterPtr);
	    sprintf(buf, "after#%d", afterPtr->id);
	    Tcl_AppendResult(interp, buf, (char *) NULL);
	    break;
	case AFTER_INFO: {
	    Tcl_Obj *resultListPtr;

	    if (objc == 2) {
		for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		     afterPtr = afterPtr->nextPtr) {
		    if (assocPtr->interp == interp) {
			sprintf(buf, "after#%d", afterPtr->id);
			Tcl_AppendElement(interp, buf);
		    }
		}
		return TCL_OK;
	    }
	    if (objc != 3) {
		Tcl_WrongNumArgs(interp, 2, objv, "?id?");
		return TCL_ERROR;
	    }
	    afterPtr = GetAfterEvent(assocPtr, objv[2]);
	    if (afterPtr == NULL) {
		Tcl_AppendResult(interp, "event \"", Tcl_GetString(objv[2]),
			"\" doesn't exist", (char *) NULL);
		return TCL_ERROR;
	    }
	    resultListPtr = Tcl_GetObjResult(interp);
 	    Tcl_ListObjAppendElement(interp, resultListPtr, afterPtr->commandPtr);
 	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
 		(afterPtr->token == NULL) ? "idle" : "timer", -1));
	    Tcl_SetObjResult(interp, resultListPtr);
	    break;
	}
	default: {
	    panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
	}
    }
    return TCL_OK;
}
```

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `AfterProc`, lines 1005–1052. Full-source SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`; snippet SHA-256 `b85d2e4ddfa43e470924a2b351b99d9c9b53ef9bdbd3472b0170fafe736cfe66`; retained evidence `e7ca34f2d6fd9344d4610`.

```text
AfterProc(clientData)
    ClientData clientData;	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;
    char *script;
    int numBytes;

    /*
     * First remove the callback from our list of callbacks;  otherwise
     * someone could delete the callback while it's being executed, which
     * could cause a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    script = Tcl_GetStringFromObj(afterPtr->commandPtr, &numBytes);
    result = Tcl_EvalEx(interp, script, numBytes, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundError(interp);
    }
    Tcl_Release((ClientData) interp);
    
    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 766–969. Full-source SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`; snippet SHA-256 `17c85107615cc7e8eb6659b952e1213906e96a11b54e2da7fc0e81de1217eceb`; retained evidence `eb83569215d92fa37da10`.

```text
Tcl_AfterObjCmd(
    ClientData clientData,	/* Unused */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    Tcl_WideInt ms;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    int index;
    char buf[16 + TCL_INTEGER_SPACE];
    static CONST char *afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *) ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc,
		(ClientData) assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType
#ifndef NO_WIDE_TYPE
	|| objv[1]->typePtr == &tclWideIntType
#endif
	|| objv[1]->typePtr == &tclBignumType
	|| ( Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, 
				 &index) != TCL_OK )) {
	index = -1;
	if (Tcl_GetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	    Tcl_AppendResult(interp, "bad argument \"",
			     Tcl_GetString(objv[1]),
			     "\": must be cancel, idle, info, or an integer",
			     NULL);
	    return TCL_ERROR;
	}
    }

    /* 
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
 	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += (long)(ms / 1000);
	wakeup.usec += ((long)(ms % 1000)) * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup, AfterProc,
							(ClientData) afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	char *command, *tempCommand;
	int tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);;
	}
	command = Tcl_GetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = Tcl_GetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && (memcmp((void*) command, (void*) tempCommand,
			    (unsigned) length) == 0)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, (ClientData) afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script script ...");
	    return TCL_ERROR;
	}
	afterPtr = (AfterInfo *) ckalloc((unsigned) (sizeof(AfterInfo)));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, (ClientData) afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO: {
	Tcl_Obj *resultListPtr;

	if (objc == 2) {
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    sprintf(buf, "after#%d", afterPtr->id);
		    Tcl_AppendElement(interp, buf);
		}
	    }
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
	    Tcl_AppendResult(interp, "event \"", TclGetString(objv[2]),
		    "\" doesn't exist", NULL);
	    return TCL_ERROR;
	}
	resultListPtr = Tcl_NewObj();
	Tcl_ListObjAppendElement(interp, resultListPtr, afterPtr->commandPtr);
	Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
 		(afterPtr->token == NULL) ? "idle" : "timer", -1));
	Tcl_SetObjResult(interp, resultListPtr);
	break;
    }
    default:
	Tcl_Panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
    }
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `AfterProc`, lines 1114–1158. Full-source SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`; snippet SHA-256 `b97779c183f626115a786a6ca0ddc79e6f97b68765168df34dcab2f2e1243e2b`; retained evidence `eb83569215d92fa37da10`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *) clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve((ClientData) interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	TclBackgroundException(interp, result);
    }
    Tcl_Release((ClientData) interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree((char *) afterPtr);
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 781–992. Full-source SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`; snippet SHA-256 `79c14177342deec6b5c34df5c79e5546d2d773c320ffd306c569a156ccd9c201`; retained evidence `e5cad7b4101332ff96f7c`.

```text
Tcl_AfterObjCmd(
    ClientData clientData,	/* Unused */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    int length;
    int index;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmds {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = ckalloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc, assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (objv[1]->typePtr == &tclIntType
#ifndef TCL_WIDE_INT_IS_LONG
	    || objv[1]->typePtr == &tclWideIntType
#endif
	    || objv[1]->typePtr == &tclBignumType
	    || (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0,
		    &index) != TCL_OK)) {
	index = -1;
	if (Tcl_GetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
            const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
                    "bad argument \"%s\": must be"
                    " cancel, idle, info, or an integer", arg));
            Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
                    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = ckalloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += (long)(ms / 1000);
	wakeup.usec += ((long)(ms % 1000)) * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup,
		AfterProc, afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	const char *command, *tempCommand;
	int tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	command = TclGetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = TclGetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && !memcmp(command, tempCommand, length)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	afterPtr = ckalloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO:
	if (objc == 2) {
	    Tcl_Obj *resultObj;

	    TclNewObj(resultObj);
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    Tcl_ListObjAppendElement(NULL, resultObj, Tcl_ObjPrintf(
			    "after#%d", afterPtr->id));
		}
	    }
            Tcl_SetObjResult(interp, resultObj);
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
            const char *eventStr = TclGetString(objv[2]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
                    "event \"%s\" doesn't exist", eventStr));
            Tcl_SetErrorCode(interp, "TCL","LOOKUP","EVENT", eventStr, (char *)NULL);
	    return TCL_ERROR;
	} else {
	    Tcl_Obj *resultListPtr;

	    TclNewObj(resultListPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr,
		    afterPtr->commandPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
		    (afterPtr->token == NULL) ? "idle" : "timer", -1));
            Tcl_SetObjResult(interp, resultListPtr);
	}
	break;
    default:
	Tcl_Panic("Tcl_AfterObjCmd: bad subcommand index to afterSubCmds");
    }
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `AfterProc`, lines 1168–1212. Full-source SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`; snippet SHA-256 `e42c4864ba8f37bb0160602f5bbe7f3bf18ce07e1d90076d6f3a6b8ffbac2c7b`; retained evidence `e5cad7b4101332ff96f7c`.

```text
AfterProc(
    ClientData clientData)	/* Describes command to execute. */
{
    AfterInfo *afterPtr = clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    ckfree(afterPtr);
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 776–981. Full-source SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`; snippet SHA-256 `1ea787e4cfe06b7dd2d6a894ff7141d74143f1f26c2dc67d2069bacf8bf85ae7`; retained evidence `e4d5b46a62af04d177edb`.

```text
Tcl_AfterObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    Tcl_Time wakeup;
    AfterInfo *afterPtr;
    AfterAssocData *assocPtr;
    Tcl_Size length;
    int index = -1;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmdsEnum {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    ThreadSpecificData *tsdPtr = InitTimer();

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * Create the "after" information associated for this interpreter, if it
     * doesn't already exist.
     */

    assocPtr = (AfterAssocData *)Tcl_GetAssocData(interp, "tclAfter", NULL);
    if (assocPtr == NULL) {
	assocPtr = (AfterAssocData *)Tcl_Alloc(sizeof(AfterAssocData));
	assocPtr->interp = interp;
	assocPtr->firstAfterPtr = NULL;
	Tcl_SetAssocData(interp, "tclAfter", AfterCleanupProc, assocPtr);
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (TclGetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, &index)
		!= TCL_OK) {
	    const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad argument \"%s\": must be"
		    " cancel, idle, info, or an integer", arg));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
		    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	if (ms < 0) {
	    ms = 0;
	}
	if (objc == 2) {
	    return AfterDelay(interp, ms);
	}
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	Tcl_GetTime(&wakeup);
	wakeup.sec += ms / 1000;
	wakeup.usec += ms % 1000 * 1000;
	if (wakeup.usec > 1000000) {
	    wakeup.sec++;
	    wakeup.usec -= 1000000;
	}
	afterPtr->token = TclCreateAbsoluteTimerHandler(&wakeup,
		AfterProc, afterPtr);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL: {
	Tcl_Obj *commandPtr;
	const char *command, *tempCommand;
	Tcl_Size tempLength;

	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|command");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    commandPtr = objv[2];
	} else {
	    commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	command = TclGetStringFromObj(commandPtr, &length);
	for (afterPtr = assocPtr->firstAfterPtr;  afterPtr != NULL;
		afterPtr = afterPtr->nextPtr) {
	    tempCommand = TclGetStringFromObj(afterPtr->commandPtr,
		    &tempLength);
	    if ((length == tempLength)
		    && !memcmp(command, tempCommand, length)) {
		break;
	    }
	}
	if (afterPtr == NULL) {
	    afterPtr = GetAfterEvent(assocPtr, commandPtr);
	}
	if (objc != 3) {
	    Tcl_DecrRefCount(commandPtr);
	}
	if (afterPtr != NULL) {
	    if (afterPtr->token != NULL) {
		Tcl_DeleteTimerHandler(afterPtr->token);
	    } else {
		Tcl_CancelIdleCall(AfterProc, afterPtr);
	    }
	    FreeAfterPtr(afterPtr);
	}
	break;
    }
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);
	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = NULL;
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_DoWhenIdle(AfterProc, afterPtr);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	break;
    case AFTER_INFO:
	if (objc == 2) {
	    Tcl_Obj *resultObj;

	    TclNewObj(resultObj);
	    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
		    afterPtr = afterPtr->nextPtr) {
		if (assocPtr->interp == interp) {
		    Tcl_ListObjAppendElement(NULL, resultObj, Tcl_ObjPrintf(
			    "after#%d", afterPtr->id));
		}
	    }
	    Tcl_SetObjResult(interp, resultObj);
	    return TCL_OK;
	}
	if (objc != 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	afterPtr = GetAfterEvent(assocPtr, objv[2]);
	if (afterPtr == NULL) {
	    const char *eventStr = TclGetString(objv[2]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "event \"%s\" doesn't exist", eventStr));
	    Tcl_SetErrorCode(interp, "TCL","LOOKUP","EVENT", eventStr, (char *)NULL);
	    return TCL_ERROR;
	} else {
	    Tcl_Obj *resultListPtr;

	    TclNewObj(resultListPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr,
		    afterPtr->commandPtr);
	    Tcl_ListObjAppendElement(interp, resultListPtr, Tcl_NewStringObj(
		    (afterPtr->token == NULL) ? "idle" : "timer", -1));
	    Tcl_SetObjResult(interp, resultListPtr);
	}
	break;
    default:
	TCL_UNREACHABLE();
    }
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `AfterProc`, lines 1147–1191. Full-source SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`; snippet SHA-256 `6728d838a062e26667d01ff42be2bf848b2ec127e60c0b8c787ba5ed3255c3fb`; retained evidence `e4d5b46a62af04d177edb`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 1018–1171. Full-source SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`; snippet SHA-256 `0362d72ed16ba6db40cc90bb91d04e0d28a5caba1016e8f124db60ae0e7d3a05`; retained evidence `eaf331dbfda81fff6c4db`.

```text
Tcl_AfterObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_WideInt ms = 0;		/* Number of milliseconds to wait */
    int index = -1;
    static const char *const afterSubCmds[] = {
	"cancel", "idle", "info", NULL
    };
    enum afterSubCmdsEnum {AFTER_CANCEL, AFTER_IDLE, AFTER_INFO};
    Tcl_Obj *cmdObj;
    int res;

    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    /*
     * First lets see if the command was passed a number as the first argument.
     */

    if (TclGetWideIntFromObj(NULL, objv[1], &ms) != TCL_OK) {
	if (Tcl_GetIndexFromObj(NULL, objv[1], afterSubCmds, "", 0, &index)
		!= TCL_OK) {
	    const char *arg = TclGetString(objv[1]);

	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad argument \"%s\": must be"
		    " cancel, idle, info, or an integer", arg));
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "INDEX", "argument",
		    arg, (char *)NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * At this point, either index = -1 and ms contains the number of ms
     * to wait, or else index is the index of a subcommand.
     */

    switch (index) {
    case -1: {
	AfterInfo *afterPtr;
	AfterAssocData *assocPtr;
	ThreadSpecificData *tsdPtr;
	long long microSeconds;

	if (ms < 0) {
	    ms = 0;
	}

	long long wakeupUS;
	wakeupUS = Tcl_GetMonotonicTime();

	if (ms >= LLONG_MAX / US_PER_MS) {
	    TimeTooFarError(interp);
	    return TCL_ERROR;
	}
	microSeconds = ms * US_PER_MS;

	if (LLONG_MAX - microSeconds < wakeupUS) {
	    TimeTooFarError(interp);
	    return TCL_ERROR;
	}

	wakeupUS += microSeconds;

	if (objc == 2) {
	    /*
	     * No command given: wait the given monotonic time.
	     */

	    return TimerDelayMonotonic(interp, wakeupUS);
	}

	/*
	 * Invoke command after given monotonic time distance.
	 */

	assocPtr = TimerAssocDataGet(interp);
	tsdPtr = InitTimer();
	afterPtr = (AfterInfo *)Tcl_Alloc(sizeof(AfterInfo));
	afterPtr->assocPtr = assocPtr;
	if (objc == 3) {
	    afterPtr->commandPtr = objv[2];
	} else {
	    afterPtr->commandPtr = Tcl_ConcatObj(objc-2, objv+2);
	}
	Tcl_IncrRefCount(afterPtr->commandPtr);

	/*
	 * The variable below is used to generate unique identifiers for after
	 * commands. This id can wrap around, which can potentially cause
	 * problems. However, there are not likely to be problems in practice,
	 * because after commands can only be requested to about a month in
	 * the future, and wrap-around is unlikely to occur in less than about
	 * 1-10 years. Thus it's unlikely that any old ids will still be
	 * around when wrap-around occurs.
	 */

	afterPtr->id = tsdPtr->afterId;
	tsdPtr->afterId += 1;
	afterPtr->token = CreateTimerHandler(wakeupUS,
		AfterProc, afterPtr, true);
	afterPtr->nextPtr = assocPtr->firstAfterPtr;
	assocPtr->firstAfterPtr = afterPtr;
	Tcl_SetObjResult(interp, Tcl_ObjPrintf("after#%d", afterPtr->id));
	return TCL_OK;
    }
    case AFTER_CANCEL:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "id|script ?script?");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    res = TimerCancelDo(interp, objv[2], false);
	} else {
	    cmdObj = Tcl_ConcatObj(objc-2, objv+2);
	    res = TimerCancelDo(interp, cmdObj, false);

	    /*
	     * When Tcl_ConcatObj was used, the created object is only
	     * decremented in this case, not in the other 3 cases in this
	     * function. I don't know why.
	     */

	    Tcl_DecrRefCount(cmdObj);
	}
	return res;
    case AFTER_IDLE:
	if (objc < 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "script ?script ...?");
	    return TCL_ERROR;
	}
	if (objc == 3) {
	    cmdObj = objv[2];
	} else {
	    cmdObj = Tcl_ConcatObj(objc-2, objv+2);
	}
	return TimerIdleDo(interp, cmdObj);
    case AFTER_INFO:
	if (objc < 2 || objc > 3) {
	    Tcl_WrongNumArgs(interp, 2, objv, "?id?");
	    return TCL_ERROR;
	}
	return TimerInfoDo(interp, objc-1, objv+1, true);
    default:
	TCL_UNREACHABLE();
    }
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `AfterProc`, lines 1533–1577. Full-source SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`; snippet SHA-256 `6728d838a062e26667d01ff42be2bf848b2ec127e60c0b8c787ba5ed3255c3fb`; retained evidence `eaf331dbfda81fff6c4db`.

```text
AfterProc(
    void *clientData)		/* Describes command to execute. */
{
    AfterInfo *afterPtr = (AfterInfo *)clientData;
    AfterAssocData *assocPtr = afterPtr->assocPtr;
    AfterInfo *prevPtr;
    int result;
    Tcl_Interp *interp;

    /*
     * First remove the callback from our list of callbacks; otherwise someone
     * could delete the callback while it's being executed, which could cause
     * a core dump.
     */

    if (assocPtr->firstAfterPtr == afterPtr) {
	assocPtr->firstAfterPtr = afterPtr->nextPtr;
    } else {
	for (prevPtr = assocPtr->firstAfterPtr; prevPtr->nextPtr != afterPtr;
		prevPtr = prevPtr->nextPtr) {
	    /* Empty loop body. */
	}
	prevPtr->nextPtr = afterPtr->nextPtr;
    }

    /*
     * Execute the callback.
     */

    interp = assocPtr->interp;
    Tcl_Preserve(interp);
    result = Tcl_EvalObjEx(interp, afterPtr->commandPtr, TCL_EVAL_GLOBAL);
    if (result != TCL_OK) {
	Tcl_AddErrorInfo(interp, "\n    (\"after\" script)");
	Tcl_BackgroundException(interp, result);
    }
    Tcl_Release(interp);

    /*
     * Free the memory for the callback.
     */

    Tcl_DecrRefCount(afterPtr->commandPtr);
    Tcl_Free(afterPtr);
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_EvalObjBackground`, lines 112–148. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `7c1e41da33a7663e8745126d78eb7578f28301335188e4f1a7fa6f48e157c0b2`; retained evidence `e4521e47b782c5165c471`.

```text
int Jim_EvalObjBackground(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_CallFrame *savedFramePtr;
    int retval;

    savedFramePtr = interp->framePtr;
    interp->framePtr = interp->topFramePtr;
    retval = Jim_EvalObj(interp, scriptObjPtr);
    interp->framePtr = savedFramePtr;
    /* Try to report the error (if any) via the bgerror proc */
    if (retval != JIM_OK && retval != JIM_RETURN && !eventLoop->suppress_bgerror) {
        Jim_Obj *objv[2];
        int rc = JIM_ERR;

        objv[0] = Jim_NewStringObj(interp, "bgerror", -1);
        objv[1] = Jim_GetResult(interp);
        Jim_IncrRefCount(objv[0]);
        Jim_IncrRefCount(objv[1]);
        if (Jim_GetCommand(interp, objv[0], JIM_NONE) == NULL || (rc = Jim_EvalObjVector(interp, 2, objv)) != JIM_OK) {
            if (rc == JIM_BREAK) {
                /* No more bgerror calls */
                eventLoop->suppress_bgerror++;
            }
            else {
                /* Report the error to stderr. */
                Jim_MakeErrorMessage(interp);
                fprintf(stderr, "%s\n", Jim_String(Jim_GetResult(interp)));
                /* And reset the result */
                Jim_SetResultString(interp, "", -1);
            }
        }
        Jim_DecrRefCount(interp, objv[0]);
        Jim_DecrRefCount(interp, objv[1]);
    }
    return retval;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELAfterCommand`, lines 686–793. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `6d5da41915de98f55d9f2056a6f0809e0cfcba5b28b5d373e9e2386f6d731c06`; retained evidence `e4521e47b782c5165c471`.

```text
static int JimELAfterCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    double ms = 0;
    jim_wide id;
    Jim_Obj *objPtr, *idObjPtr;
    static const char * const options[] = {
        "cancel", "info", "idle", NULL
    };
    enum
    { AFTER_CANCEL, AFTER_INFO, AFTER_IDLE, AFTER_RESTART, AFTER_EXPIRE, AFTER_CREATE };
    int option = AFTER_CREATE;
    if (Jim_GetDouble(interp, argv[1], &ms) != JIM_OK) {
        if (Jim_GetEnum(interp, argv[1], options, &option, "argument", JIM_ERRMSG) != JIM_OK) {
            return JIM_ERR;
        }
        Jim_SetEmptyResult(interp);
    }
    else if (argc == 2) {
        /* Simply a sleep */
        usleep(ms * 1000);
        return JIM_OK;
    }

    switch (option) {
        case AFTER_IDLE:
            if (argc < 3) {
                Jim_WrongNumArgs(interp, 2, argv, "script ?script ...?");
                return JIM_ERR;
            }
            /* fall through */
        case AFTER_CREATE: {
            Jim_Obj *scriptObj = Jim_ConcatObj(interp, argc - 2, argv + 2);
            Jim_IncrRefCount(scriptObj);
            id = Jim_CreateTimeHandler(interp, (jim_wide)(ms * 1000), JimAfterTimeHandler, scriptObj,
                JimAfterTimeEventFinalizer);
            objPtr = Jim_NewStringObj(interp, NULL, 0);
            Jim_AppendString(interp, objPtr, "after#", -1);
            idObjPtr = Jim_NewIntObj(interp, id);
            Jim_IncrRefCount(idObjPtr);
            Jim_AppendObj(interp, objPtr, idObjPtr);
            Jim_DecrRefCount(interp, idObjPtr);
            Jim_SetResult(interp, objPtr);
            return JIM_OK;
        }
        case AFTER_CANCEL:
            if (argc < 3) {
                Jim_WrongNumArgs(interp, 2, argv, "id|command");
                return JIM_ERR;
            }
            else {
                jim_wide remain = 0;

                id = JimParseAfterId(argv[2]);
                if (id <= 0) {
                    /* Not an event id, so search by script */
                    Jim_Obj *scriptObj = Jim_ConcatObj(interp, argc - 2, argv + 2);
                    id = JimFindAfterByScript(eventLoop, scriptObj);
                    Jim_FreeNewObj(interp, scriptObj);
                    if (id <= 0) {
                        /* Not found */
                        break;
                    }
                }
                remain = Jim_DeleteTimeHandler(interp, id);
                if (remain >= 0) {
                    Jim_SetResultInt(interp, remain);
                }
            }
            break;

        case AFTER_INFO:
            if (argc == 2) {
                Jim_TimeEvent *te = eventLoop->timeEventHead;
                Jim_Obj *listObj = Jim_NewListObj(interp, NULL, 0);
                char buf[30];
                const char *fmt = "after#%" JIM_WIDE_MODIFIER;

                while (te) {
                    snprintf(buf, sizeof(buf), fmt, te->id);
                    Jim_ListAppendElement(interp, listObj, Jim_NewStringObj(interp, buf, -1));
                    te = te->next;
                }
                Jim_SetResult(interp, listObj);
            }
            else if (argc == 3) {
                id = JimParseAfterId(argv[2]);
                if (id >= 0) {
                    Jim_TimeEvent *e = JimFindTimeHandlerById(eventLoop, id);
                    if (e && e->timeProc == JimAfterTimeHandler) {
                        Jim_Obj *listObj = Jim_NewListObj(interp, NULL, 0);
                        Jim_ListAppendElement(interp, listObj, e->clientData);
                        Jim_ListAppendElement(interp, listObj, Jim_NewStringObj(interp, e->initialus ? "timer" : "idle", -1));
                        Jim_SetResult(interp, listObj);
                        return JIM_OK;
                    }
                }
                Jim_SetResultFormatted(interp, "event \"%#s\" doesn't exist", argv[2]);
                return JIM_ERR;
            }
            else {
                Jim_WrongNumArgs(interp, 2, argv, "?id?");
                return JIM_ERR;
            }
            break;
    }
    return JIM_OK;
}
```


## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `cmd_after`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs), `after_cmd`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
