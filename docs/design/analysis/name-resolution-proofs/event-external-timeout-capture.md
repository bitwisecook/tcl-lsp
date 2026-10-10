# naming.event.external-timeout-capture

Kind: `native-observation`

## Problem statement

A blocked notifier or wrong-arity handler cannot be recorded as a returned guest error.

## Question

Which isolated NO_SOURCES and wrong-arity PREFIX_BGERROR processes actually complete, and what evidence exists for the abandoned v105 runner?

## Conclusion

v106 NO_SOURCES hits the two-second external deadline on all five C providers; no guest completion/result row exists. Its Jim process returns OK with an empty result. PREFIX_BGERROR hits the external deadline on C85/C86/C90/C91; C84/Jim complete with setup errors. v105 retains only its inputs, runner and C84 compile streams: the runner did not preserve TimeoutExpired streams or a process receipt, so it supplies no observed guest answer.

## Scope

External subprocess observations and their retained partial streams, deadlines, return status and setup results. The timeout limit is outside the interpreter. No would-wait-forever guest result, native unavailability or observed callback completion is inferred. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/NO_SOURCES: external deadline; no guest completion row v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'bad option "bgerror": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/NO_SOURCES: external deadline; no guest completion row v106/PREFIX_BGERROR: external deadline; no guest completion row

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/NO_SOURCES: external deadline; no guest completion row v106/PREFIX_BGERROR: external deadline; no guest completion row

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/NO_SOURCES: external deadline; no guest completion row v106/PREFIX_BGERROR: external deadline; no guest completion row

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/NO_SOURCES: external deadline; no guest completion row v106/PREFIX_BGERROR: external deadline; no guest completion row

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/NO_SOURCES: NO_SOURCES: code 0, bytes b'' v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'wrong # args: should be "interp"'

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance build attached.. Channel: not exercised. Dialect: bigip.

No BIG-IP provider was executed or inspected for this Event question.

## Exact evidence

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ee0dcc279531301092276` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/receipt.json). SHA-256 `82a055b4a90a1333e4b71b75f0652dfd54d19132815ffee3a8ceda6e57313a48`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea7880242849960bdac70` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/stdout.tsv). SHA-256 `ed06c85e7808c934e4817420f9a39934eacd62b0bf9a061bbec942f1d57b311c`. Unmodified original stdout, including partial rows when externally timed out.
- `e8397c32c06aec04cf2a1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e123724388f8662dc5479` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json). SHA-256 `b222d7179b1af0ae7a697be985efa7067efda94cb80520f53d279fc08036799c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4d552cae616020850bad` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv). SHA-256 `3fb0c0d1af8b826923ae5a8a1faccda23b3203b31cb5be2710175d85936dcd61`. Unmodified original stdout, including partial rows when externally timed out.
- `e1013c02b786ae4787640` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e39b4a84ed86ea708f157` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/receipt.json). SHA-256 `acd8140221e03457b163568a58d944205f5557df96b7f3de59a9da16e00c5005`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef9c99de6099a74ef3dbf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/stdout.tsv). SHA-256 `d25ff057a58b09197d9668a8e4e1896f713308049ce48652583707d8ca084573`. Unmodified original stdout, including partial rows when externally timed out.
- `ead67a8669f5e7a55b03e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e86127283c231113b4bd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json). SHA-256 `b860328965fc4644dbc64eb62c7f4bab2cd391ea788f7388a92ef8fff8d3c2c8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4c30cdcc3daab9ce078f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv). SHA-256 `9e00d09da240728cc6509a7a4374a20e4f197c5da5efdbd6ce9a804e7cf20ec0`. Unmodified original stdout, including partial rows when externally timed out.
- `e08d3ad19e836d289fe8a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr). SHA-256 `b9233ab447c7bd5efbce02cf8b2c91736ce25c74b4b4fe1af75d63d51a9e1cbe`. Unmodified original stderr for this isolated process.
- `e13b5f573945d4acf1029` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/receipt.json). SHA-256 `a80178b0652b37f5fcffe81ec54fd2b277cd9db8996f6e6a6fe67da588c1313a`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5b8fa42ef8c08cb5ff23` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/stdout.tsv). SHA-256 `164fc8880ce0548cdef20aee9da61fb97fb4d86cb29f6eec39aee873a87b7bdb`. Unmodified original stdout, including partial rows when externally timed out.
- `e235885ff1be275d1fdb4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1ea51c8f7a716eb60841` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json). SHA-256 `6ec36c8348631fe0dca31183a7b3536f05ce3dbb9fb82e4bc7c3d07d44c58157`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e86d502dd6b79bc0ae21e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv). SHA-256 `14eb5aa586ab830259b4056c005be8d15a53304c911996b6db609e090b3ff1c3`. Unmodified original stdout, including partial rows when externally timed out.
- `e656388044f19358644ca` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `e58ebef7edcf820744b55` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/receipt.json). SHA-256 `2cc5dadd1d099ac4d366cef982b269a43f3ecdccdadcabee8dfa579c0b03264f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6fc2c4e6e972f794ddf8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/stdout.tsv). SHA-256 `1ec3d11e7c1bec1b3e4c7f9f810ba547725d542236dd75430bfb4b921ae0fb74`. Unmodified original stdout, including partial rows when externally timed out.
- `e9f4a5fa28fe9aa2eef95` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e97f5cda7f0779ed6ba12` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json). SHA-256 `0e28b1add27cb25d7bbf008799b502da1f91be9ecb8950d30d4041f49579723e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2a808efc8b870155c485` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv). SHA-256 `535659defdf97c08694be8e5b5c46128088764d47e7940bc49287754d67e3923`. Unmodified original stdout, including partial rows when externally timed out.
- `e036464c5dfee454cff5d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `eac850ff53f0518e03b13` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/receipt.json). SHA-256 `5bbc9231ec30c801d3e6e3b6775834fa7a664b70c612c5fa7efa66ebd76694a2`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eac69e831c16c691787d6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/stdout.tsv). SHA-256 `9a419fe5ecd6b94a5809d5cdc1b8ef138ae6e140741595e044b6d9dc3962bd42`. Unmodified original stdout, including partial rows when externally timed out.
- `eccbdda804d0c3184d166` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ebf8040312905f78db7bb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json). SHA-256 `77ca937523991b45689fb560f5334228151adcf27a1244e89905ab32eb75f68f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eac1303463e81a05ff9bd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv). SHA-256 `f7c6035db04e0948ca179cda01bfd86c871c3604b3e1914c5d8c34fe54abc8b2`. Unmodified original stdout, including partial rows when externally timed out.
- `e2e121b714f2a48d1f22e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `eecb778a4ee73268e3f7c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/receipt.json). SHA-256 `1b142133ba1c688ac3282f667dfccd26a46b1215e8ee5098d791ae69ed996df5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1be8342f17dadd7ae3e9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/stdout.tsv). SHA-256 `e765b2987a3a0cc558535113b4920d404ea377fb068b7b793897164fbf1650c3`. Unmodified original stdout, including partial rows when externally timed out.
- `e708c69a75b329eed5794` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/NO_SOURCES/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4cc10151aeb0a283b3c4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json). SHA-256 `ab4ec434cfc827c5aff3a51019f1702be11a68c70a974d33e70bee847a601fa1`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3286a5ae533600d53a4e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv). SHA-256 `1fb872aab4063af110fa76408be98a248139ec0638ce7f9b30a4fc2e1b86c2e0`. Unmodified original stdout, including partial rows when externally timed out.
- `e470e3e985c3b2e7220a9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbedc1e534919177588f` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v105/queue.json). SHA-256 `7a546e10d86afdccd527af83c908c5ddd79f540392d528902ccc423e6e703e1b`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.
- `e2b697554d44420580ffd` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v105/probe.c). SHA-256 `734a4c4c90405b55fb745ad4af183fd67cabe51f0994a5e0094975b1fa281610`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.
- `eb08f5a89e1f1f8dcdbd3` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v105/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.
- `e73630e03d4833af78fee` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v105/capture.py). SHA-256 `9322ab4dde313046412cdd1476c99edf886373b57e75b2dfb19848fd6072e198`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.
- `ecddf35b92e7805f1acee` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/capture/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_event_original/v105/capture/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.
- `e67a55a5012933cb15660` (limitation): [rust/tcl-registry/tests/data/native_event_original/v105/capture/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v105/capture/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original abandoned v105 input/compile/harness artifact. No process receipt or TimeoutExpired stdout/stderr survived this runner.

## Source inspection

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1176–1216. Full-source SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`; snippet SHA-256 `1328a8f513834f5b7a5b16fc36f88138ec507eb04f53c43f8b8a9edda31a52ef`; retained evidence `e98d4bc726037374525cd`.

```text
Tcl_VwaitObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
        Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set
     * by event handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\":  would wait forever", (char *) NULL);
	return TCL_ERROR;
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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1321–1368. Full-source SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`; snippet SHA-256 `cf24836c48502fc46b580d223b5cf8b21de1d5cad5eef848cd57b5caa2d4b60f`; retained evidence `ebcd14ee056c41f642dc1`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int done, foundEvent;
    char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_LimitExceeded(interp)) {
	    break;
	}
    }
    Tcl_UntraceVar(interp, nameString,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, (ClientData) &done);

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
    if (!foundEvent) {
	Tcl_AppendResult(interp, "can't wait for variable \"", nameString,
		"\": would wait forever", NULL);
	return TCL_ERROR;
    }
    if (!done) {
	Tcl_AppendResult(interp, "limit exceeded", NULL);
	return TCL_ERROR;
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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1387–1447. Full-source SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`; snippet SHA-256 `6a720ec0b86320ee0c3eb9ac57fe7dda865390cee80306fd1844c2ccb65b85a7`; retained evidence `e1799f8f23cd143629768`.

```text
Tcl_VwaitObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int done, foundEvent;
    const char *nameString;

    if (objc != 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "name");
	return TCL_ERROR;
    }
    nameString = Tcl_GetString(objv[1]);
    if (Tcl_TraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done) != TCL_OK) {
	return TCL_ERROR;
    };
    done = 0;
    foundEvent = 1;
    while (!done && foundEvent) {
	foundEvent = Tcl_DoOneEvent(TCL_ALL_EVENTS);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    break;
	}
    }
    Tcl_UntraceVar2(interp, nameString, NULL,
	    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, &done);

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't wait for variable \"%s\": would wait forever",
		nameString));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	return TCL_ERROR;
    }
    if (!done) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */

	return TCL_ERROR;
    }

    /*
     * Clear out the interpreter's result, since it may have been set by event
     * handlers.
     */

    Tcl_ResetResult(interp);
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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1498–1867. Full-source SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`; snippet SHA-256 `51c2ed4fc19f0138783d2623b8c15f51f5485a04d976d7e8bbb2432d5a59c33b`; retained evidence `e30bb9be41c1824d28e5d`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int i, done = 0, timedOut = 0, foundEvent, any = 1, timeout = 0;
    int numItems = 0, extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    Tcl_Time before, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = -1;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = -1;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	Tcl_GetTime(&before);
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	   ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	Tcl_GetTime(&after);
	diff = after.sec * 1000 + after.usec / 1000;
	diff -= before.sec * 1000 + before.usec / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    int k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1536–1906. Full-source SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`; snippet SHA-256 `0f13359ad3fe496d90ff98f311f7951d3e6c13a770b2f9b2a8e4fdc9fd49d291`; retained evidence `eaa5f919dc246958bc5d1`.

```text
Tcl_VwaitObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Tcl_Size i, done = 0, numItems = 0, timedOut = 0;
    int foundEvent, any = 1, timeout = 0;
    int extended = 0, result, mode, mask = TCL_ALL_EVENTS;
    Tcl_InterpState saved = NULL;
    Tcl_TimerToken timer = NULL;
    long long before = -1, after;
    Tcl_Channel chan;
    Tcl_WideInt diff = -1;
    VwaitItem localItems[32], *vwaitItems = localItems;
    static const char *const vWaitOptionStrings[] = {
	"-all",	"-extended", "-nofileevents", "-noidleevents",
	"-notimerevents", "-nowindowevents", "-readable",
	"-timeout", "-variable", "-writable", "--", NULL
    };
    enum vWaitOptions {
	OPT_ALL, OPT_EXTD, OPT_NO_FEVTS, OPT_NO_IEVTS,
	OPT_NO_TEVTS, OPT_NO_WEVTS, OPT_READABLE,
	OPT_TIMEOUT, OPT_VARIABLE, OPT_WRITABLE, OPT_LAST
    } index;

    if ((objc == 2) && (strcmp(Tcl_GetString(objv[1]), "--") != 0)) {
	/*
	 * Legacy "vwait" syntax, skip option handling.
	 */
	i = 1;
	goto endOfOptionLoop;
    }

    if ((unsigned) objc - 1 > sizeof(localItems) / sizeof(localItems[0])) {
	vwaitItems = (VwaitItem *)Tcl_Alloc(sizeof(VwaitItem) * (objc - 1));
    }

    for (i = 1; i < objc; i++) {
	const char *name;

	name = TclGetString(objv[i]);
	if (name[0] != '-') {
	    break;
	}
	if (Tcl_GetIndexFromObj(interp, objv[i], vWaitOptionStrings, "option", 0,
		&index) != TCL_OK) {
	    result = TCL_ERROR;
	    goto done;
	}
	switch (index) {
	case OPT_ALL:
	    any = 0;
	    break;
	case OPT_EXTD:
	    extended = 1;
	    break;
	case OPT_NO_FEVTS:
	    mask &= ~TCL_FILE_EVENTS;
	    break;
	case OPT_NO_IEVTS:
	    mask &= ~TCL_IDLE_EVENTS;
	    break;
	case OPT_NO_TEVTS:
	    mask &= ~TCL_TIMER_EVENTS;
	    break;
	case OPT_NO_WEVTS:
	    mask &= ~TCL_WINDOW_EVENTS;
	    break;
	case OPT_TIMEOUT:
	    if (++i >= objc) {
	needArg:
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"argument required for \"%s\"", vWaitOptionStrings[index]));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "ARGUMENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    if (Tcl_GetIntFromObj(interp, objv[i], &timeout) != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (timeout < 0) {
		Tcl_ResetResult(interp);
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"timeout must be positive", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NEGTIME", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	    break;
	case OPT_LAST:
	    i++;
	    goto endOfOptionLoop;
	case OPT_VARIABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		    TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[numItems]);
	    if (result != TCL_OK) {
		goto done;
	    }
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = 0;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_READABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_READABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for reading",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_READABLE,
		    VwaitChannelReadProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_READABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	case OPT_WRITABLE:
	    if (++i >= objc) {
		goto needArg;
	    }
	    if (TclGetChannelFromObj(interp, objv[i], &chan, &mode, 0)
		    != TCL_OK) {
		result = TCL_ERROR;
		goto done;
	    }
	    if (!(mode & TCL_WRITABLE)) {
		Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			"channel \"%s\" wasn't open for writing",
			TclGetString(objv[i])));
		result = TCL_ERROR;
		goto done;
	    }
	    Tcl_CreateChannelHandler(chan, TCL_WRITABLE,
		    VwaitChannelWriteProc, &vwaitItems[numItems]);
	    vwaitItems[numItems].donePtr = &done;
	    vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	    vwaitItems[numItems].mask = TCL_WRITABLE;
	    vwaitItems[numItems].sourceObj = objv[i];
	    numItems++;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    }

  endOfOptionLoop:
    if ((mask & (TCL_FILE_EVENTS | TCL_IDLE_EVENTS |
	    TCL_TIMER_EVENTS | TCL_WINDOW_EVENTS)) == 0) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"can't wait: would block forever", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if ((timeout > 0) && ((mask & TCL_TIMER_EVENTS) == 0)) {
	Tcl_SetObjResult(interp, Tcl_NewStringObj(
		"timer events disabled with timeout specified", -1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_TIME", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    for (result = TCL_OK; i < objc; i++) {
	result = Tcl_TraceVar2(interp, TclGetString(objv[i]), NULL,
		TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		VwaitVarProc, &vwaitItems[numItems]);
	if (result != TCL_OK) {
	    break;
	}
	vwaitItems[numItems].donePtr = &done;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = objv[i];
	numItems++;
    }
    if (result != TCL_OK) {
	result = TCL_ERROR;
	goto done;
    }

    if (!(mask & TCL_FILE_EVENTS)) {
	for (i = 0; i < numItems; i++) {
	    if (vwaitItems[i].mask) {
		Tcl_SetObjResult(interp, Tcl_NewStringObj(
			"file events disabled with channel(s) specified", -1));
		Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_FILE_EVENT", (char *)NULL);
		result = TCL_ERROR;
		goto done;
	    }
	}
    }

    if (timeout > 0) {
	vwaitItems[numItems].donePtr = &timedOut;
	vwaitItems[numItems].sequence = TCL_INDEX_NONE;
	vwaitItems[numItems].mask = 0;
	vwaitItems[numItems].sourceObj = NULL;
	timer = Tcl_CreateTimerHandler(timeout, VwaitTimeoutProc,
		&vwaitItems[numItems]);
	before = Tcl_GetDayTime();
    } else {
	timeout = 0;
    }

    if ((numItems == 0) && (timeout == 0)) {
	/*
	 * "vwait" is equivalent to "update",
	 * "vwait -nofileevents -notimerevents -nowindowevents"
	 * is equivalent to "update idletasks"
	 */
	any = 1;
	mask |= TCL_DONT_WAIT;
    }

    foundEvent = 1;
    while (!timedOut && foundEvent &&
	    ((!any && (done < numItems)) || (any && !done))) {
	foundEvent = Tcl_DoOneEvent(mask);
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    break;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    Tcl_SetErrorCode(interp, "TCL", "EVENT", "LIMIT", (char *)NULL);
	    break;
	}
	if ((numItems == 0) && (timeout == 0)) {
	    /*
	     * Behavior like "update": clear interpreter's result because
	     * event handlers could have executed commands.
	     */
	    Tcl_ResetResult(interp);
	    result = TCL_OK;
	    goto done;
	}
    }

    if (!foundEvent) {
	Tcl_ResetResult(interp);
	Tcl_SetObjResult(interp, Tcl_NewStringObj((numItems == 0) ?
		"can't wait: would wait forever" :
		"can't wait for variable(s)/channel(s): would wait forever",
		-1));
	Tcl_SetErrorCode(interp, "TCL", "EVENT", "NO_SOURCES", (char *)NULL);
	result = TCL_ERROR;
	goto done;
    }

    if (!done && !timedOut) {
	/*
	 * The interpreter's result was already set to the right error message
	 * prior to exiting the loop above.
	 */
	result = TCL_ERROR;
	goto done;
    }

    result = TCL_OK;
    if (timeout <= 0) {
	/*
	 * Clear out the interpreter's result, since it may have been set
	 * by event handlers.
	 */
	Tcl_ResetResult(interp);
	goto done;
    }

    /*
     * When timeout was specified, report milliseconds left or -1 on timeout.
     */
    if (timedOut) {
	diff = -1;
    } else {
	after = Tcl_GetDayTime();
	diff = after / 1000;
	diff -= before / 1000;
	diff = timeout - diff;
	if (diff < 0) {
	    diff = 0;
	}
    }

  done:
    if ((timeout > 0) && (timer != NULL)) {
	Tcl_DeleteTimerHandler(timer);
    }
    if (result != TCL_OK) {
	saved = Tcl_SaveInterpState(interp, result);
    }
    for (i = 0; i < numItems; i++) {
	if (vwaitItems[i].mask & TCL_READABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelReadProc,
			&vwaitItems[i]);
	    }
	} else if (vwaitItems[i].mask & TCL_WRITABLE) {
	    if (TclGetChannelFromObj(interp, vwaitItems[i].sourceObj,
		    &chan, &mode, 0) == TCL_OK) {
		Tcl_DeleteChannelHandler(chan, VwaitChannelWriteProc,
			&vwaitItems[i]);
	    }
	} else {
	    Tcl_UntraceVar2(interp, TclGetString(vwaitItems[i].sourceObj),
		    NULL, TCL_GLOBAL_ONLY|TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
		    VwaitVarProc, &vwaitItems[i]);
	}
    }

    if (result == TCL_OK) {
	if (extended) {
	    Tcl_Size k;
	    Tcl_Obj *listObj, *keyObj;

	    TclNewObj(listObj);
	    for (k = 0; k < done; k++) {
		for (i = 0; i < numItems; i++) {
		    if (vwaitItems[i].sequence != k) {
			continue;
		    }
		    if (vwaitItems[i].mask & TCL_READABLE) {
			TclNewLiteralStringObj(keyObj, "readable");
		    } else if (vwaitItems[i].mask & TCL_WRITABLE) {
			TclNewLiteralStringObj(keyObj, "writable");
		    } else {
			TclNewLiteralStringObj(keyObj, "variable");
		    }
		    Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		    Tcl_ListObjAppendElement(NULL, listObj,
			    vwaitItems[i].sourceObj);
		}
	    }
	    if (timeout > 0) {
		TclNewLiteralStringObj(keyObj, "timeleft");
		Tcl_ListObjAppendElement(NULL, listObj, keyObj);
		Tcl_ListObjAppendElement(NULL, listObj,
			Tcl_NewWideIntObj(diff));
	    }
	    Tcl_SetObjResult(interp, listObj);
	} else if (timeout > 0) {
	    Tcl_SetObjResult(interp, Tcl_NewWideIntObj(diff));
	}
    } else {
	result = Tcl_RestoreInterpState(interp, saved);
    }
    if (vwaitItems != localItems) {
	Tcl_Free(vwaitItems);
    }
    return result;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELVwaitCommand`, lines 565–645. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `0309243c6a2a7cca9a072f5c5f7ed82248d291eae31886f251c35eb3f730c45a`; retained evidence `e4521e47b782c5165c471`.

```text
static int JimELVwaitCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    Jim_Obj *oldValue = NULL;
    Jim_Obj *scriptObjPtr = NULL;
    int rc;
    int signal = 0;

    if (argc > 2 && Jim_CompareStringImmediate(interp, argv[1], "-signal")) {
        signal++;
    }

    if (argc - signal == 3) {
        scriptObjPtr = argv[2 + signal];
    }
    else if (argc - signal != 2) {
        return JIM_USAGE;
    }

    oldValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);

    if (oldValue) {
        Jim_IncrRefCount(oldValue);
    }
    else {
        /* If a result was left, it is an error */
        if (Jim_Length(Jim_GetResult(interp))) {
            return JIM_ERR;
        }
    }

    eventLoop->suppress_bgerror = 0;

    while ((rc = Jim_ProcessEvents(interp, JIM_ALL_EVENTS)) >= 0) {
        Jim_Obj *currValue;

        if (signal && interp->sigmask) {
            /* vwait -signal and handled signals were received, so transfer them
             * to ignored signals so that 'signal check -clear' will return them.
             * It's possible that if signals aren't supported we shouldn't even
             * allow the -signal option.
             */
#ifdef jim_ext_signal
            Jim_SignalSetIgnored(interp->sigmask);
#endif
            interp->sigmask = 0;
            break;
        }

        currValue = Jim_GetGlobalVariable(interp, argv[1 + signal], JIM_NONE);
        /* Stop the loop if the vwait-ed variable changed value,
         * or if was unset and now is set (or the contrary)
         * or if a signal was caught
         */
        if ((oldValue && !currValue) ||
            (!oldValue && currValue) ||
            (oldValue && currValue && !Jim_StringEqObj(oldValue, currValue)) ||
            Jim_CheckSignal(interp)) {
            break;
        }
        if (scriptObjPtr) {
            /* Stop the loop if a provided script returns BREAK or ERR */
            int retval = Jim_EvalObj(interp, scriptObjPtr);
            if (retval == JIM_ERR || retval == JIM_BREAK) {
                if (retval == JIM_ERR) {
                    rc = -2;
                }
                break;
            }
        }
    }
    if (oldValue)
        Jim_DecrRefCount(interp, oldValue);

    if (rc == -2) {
        return JIM_ERR;
    }

    Jim_SetEmptyResult(interp);
    return JIM_OK;
}
```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
