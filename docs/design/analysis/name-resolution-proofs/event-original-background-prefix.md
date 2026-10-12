# naming.event.original-background-prefix

Kind: `native-observation`

## Problem statement

Rendering a configured background-error prefix drops original arguments and omits the selected options operand.

## Question

With the correctly shaped handler {prefix message options}, what do these configured background-error callbacks return, and where does setup fail on unsupported providers?

## Conclusion

C85/C86/C90/C91 return PRE BOOM 1 for the separately pinned v107 handler, which receives the original fixed prefix, message and options. C84 rejects interp bgerror during setup; Jim rejects the interp invocation during setup, before reaching the callback. The wrong-arity v106 handler does not establish a callback failure law: modern C processes do not complete before the external deadline.

## Scope

Two bounded source setups per provider, modern C v107 callback observation and older/Jim setup rejection. Wrong-arity external timeouts are scoped separately. No default Tcl library bgerror equivalence, missing-handler fallback, C84 configured-prefix availability or Jim prefix-options claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'bad option "bgerror": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer' v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 1, bytes b'bad option "bgerror": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/PREFIX_BGERROR: external deadline; no guest completion row v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 0, bytes b'PRE BOOM 1'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/PREFIX_BGERROR: external deadline; no guest completion row v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 0, bytes b'PRE BOOM 1'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/PREFIX_BGERROR: external deadline; no guest completion row v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 0, bytes b'PRE BOOM 1'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/PREFIX_BGERROR: external deadline; no guest completion row v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 0, bytes b'PRE BOOM 1'

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/PREFIX_BGERROR: PREFIX_BGERROR: code 1, bytes b'wrong # args: should be "interp"' v107/GLOBAL_PREFIX_BGERROR: GLOBAL_PREFIX_BGERROR: code 1, bytes b'wrong # args: should be "interp"'

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance build attached.. Channel: not exercised. Dialect: bigip.

No BIG-IP provider was executed or inspected for this Event question.

## Exact evidence

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ef8bb2ebdccb8192ef78d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c). SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ea9bf8d7d5c098c478df8` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c). SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ea3de3c863d05700ccfc3` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c). SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e791ac2cd6ff96da72529` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c). SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e123724388f8662dc5479` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json). SHA-256 `b222d7179b1af0ae7a697be985efa7067efda94cb80520f53d279fc08036799c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4d552cae616020850bad` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv). SHA-256 `3fb0c0d1af8b826923ae5a8a1faccda23b3203b31cb5be2710175d85936dcd61`. Unmodified original stdout, including partial rows when externally timed out.
- `e1013c02b786ae4787640` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e86127283c231113b4bd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json). SHA-256 `b860328965fc4644dbc64eb62c7f4bab2cd391ea788f7388a92ef8fff8d3c2c8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4c30cdcc3daab9ce078f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv). SHA-256 `9e00d09da240728cc6509a7a4374a20e4f197c5da5efdbd6ce9a804e7cf20ec0`. Unmodified original stdout, including partial rows when externally timed out.
- `e08d3ad19e836d289fe8a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr). SHA-256 `b9233ab447c7bd5efbce02cf8b2c91736ce25c74b4b4fe1af75d63d51a9e1cbe`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1ea51c8f7a716eb60841` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json). SHA-256 `6ec36c8348631fe0dca31183a7b3536f05ce3dbb9fb82e4bc7c3d07d44c58157`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e86d502dd6b79bc0ae21e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv). SHA-256 `14eb5aa586ab830259b4056c005be8d15a53304c911996b6db609e090b3ff1c3`. Unmodified original stdout, including partial rows when externally timed out.
- `e656388044f19358644ca` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e97f5cda7f0779ed6ba12` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json). SHA-256 `0e28b1add27cb25d7bbf008799b502da1f91be9ecb8950d30d4041f49579723e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2a808efc8b870155c485` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv). SHA-256 `535659defdf97c08694be8e5b5c46128088764d47e7940bc49287754d67e3923`. Unmodified original stdout, including partial rows when externally timed out.
- `e036464c5dfee454cff5d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ebf8040312905f78db7bb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json). SHA-256 `77ca937523991b45689fb560f5334228151adcf27a1244e89905ab32eb75f68f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eac1303463e81a05ff9bd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv). SHA-256 `f7c6035db04e0948ca179cda01bfd86c871c3604b3e1914c5d8c34fe54abc8b2`. Unmodified original stdout, including partial rows when externally timed out.
- `e2e121b714f2a48d1f22e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr). SHA-256 `af2fa5a8e3b45df78d8f3338a26e1eb055adeffb7d9bed26c4b376c9058902ca`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4cc10151aeb0a283b3c4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json). SHA-256 `ab4ec434cfc827c5aff3a51019f1702be11a68c70a974d33e70bee847a601fa1`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3286a5ae533600d53a4e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv). SHA-256 `1fb872aab4063af110fa76408be98a248139ec0638ce7f9b30a4fc2e1b86c2e0`. Unmodified original stdout, including partial rows when externally timed out.
- `e470e3e985c3b2e7220a9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e6ed8c66cd0858b1fe2a4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `4c3730fd21bb5a4e22d3b6fadd034e03ba9a19868c9177fb9456f6dd38fec737`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee38af258aca8ad271b7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `e029064dfabba1580a6215ff920b03ceed933c5304219c89ae601707452447f5`. Unmodified original stdout, including partial rows when externally timed out.
- `e27c55a0804cd093d531c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). SHA-256 `08d5e613d23c9319712feb1d713b91a0f3e59e0418f527147a09d31df2dfc28f`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). SHA-256 `c52f03c8c20bab56f0a2e905d6c8c42d33235a2ede36b83649ffa5d35256b152`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). SHA-256 `f16fe8da41ee99ee01a2a1098d1b18d1a47decbdfaaabd13052ce18dd2e5508b`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). SHA-256 `e063fe10f9f6de3c8ab5fc1034cb42009c078044819104d2d1a635ff215ddbf2`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). SHA-256 `8625f645ffd0d2c22375052395dcb11ea7ae60a1b6d6d49fefca8bbcc4c927fb`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ed33acf513a9307d61b6e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `2c3b5817a07ab16e3c28c6abbbb1759850624cfac6e78e6af14eea303cada413`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e585c9206a7c664b9dbb4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `f5e4fe12ba9fb0aa6562e02cd8af0f3de0739c3c26deb51d31ea7f0f611d7e9c`. Unmodified original stdout, including partial rows when externally timed out.
- `e581229313b566bac24c8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). SHA-256 `49987b659897bfa0668cb0dfdf7417e6e3db4da8d6ac96d51435332e57bef843`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ee0f227382f1ee56c78f0` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `253cad20a2446bd2bafb500e354f2b2f27a4e00e7321b6b0c3886d285126be98`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed95b63d15133bdb11612` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `6801010be81ff85373a2641653c744d2b9ce3a7097dd176713cbdcffafd70b8f`. Unmodified original stdout, including partial rows when externally timed out.
- `eafca7a79840a7053ff9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). SHA-256 `288681099d036401a5bcbdd258b2738c8d0b9bbd4fc157a0ab71de83e8d15015`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e69e4d234d255693865f4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `c24b674a00f452d491fcf318b9af9f4fcf0894b81d8c5c661a779c62736e4830`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e949b294fef43a8891fc5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `76fb9fae9e31e4e5ecf89bd169442ea7a0d8196d125d427344d8a709a47e5c87`. Unmodified original stdout, including partial rows when externally timed out.
- `e3eccada9da550cd9faf8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). SHA-256 `b97df66ab92c5c474dcd2e84c6032e5580d67fc617a4bde0f0647848fbcec5db`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e833642caa9092df8aa07` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `2cc35e7364c62cc9def98b731127c4f306f8db2af0585b58abdf2edc414b50fd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e7be4aa3551c56b251b05` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `f704ef442b2ada812009af53ebfa63b24328c023df219de29564eec078dc1c2d`. Unmodified original stdout, including partial rows when externally timed out.
- `e00166f233b13cba954af` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). SHA-256 `605fc7740871917aabfb184678f9e0a2d77ecfc43c4e6b5c38a3b5a087916093`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e11603269e92a396c6e93` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/receipt.json). SHA-256 `3c2990fd882ff058de7a9610dffe31dd347b7da9b21ccce94223982d1268376c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e440393829c37fa4b823b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stdout.tsv). SHA-256 `de6ba10536aa469eef2fc0728a6667ed90a59fb2374cf05d3e4b5e2e9e6115e4`. Unmodified original stdout, including partial rows when externally timed out.
- `ef7480ddb114e209a4341` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). SHA-256 `56acfee2e4d15207ae30905aa25c737b89c678e5873e652697cfe4a2373c87b9`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveBgerror`, lines 2097–2116. Full-source SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`; snippet SHA-256 `a659ffa5a728750dff2eb23810275e482d923322945285fe8f73ba33665d2cc4`; retained evidence `ef8bb2ebdccb8192ef78d`.

```text
SlaveBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *slaveInterp,	/* Interp in which limit is set/queried. */
    int objc,			/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	int length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_AppendResult(interp, "cmdPrefix must be list of length >= 1",
		    NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(slaveInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(slaveInterp));
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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildBgerror`, lines 2268–2289. Full-source SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`; snippet SHA-256 `3fc49715c8939169f8518387d819d68d4624c8ef1e23dcb32b0a54c9349d3d34`; retained evidence `ea9bf8d7d5c098c478df8`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    int objc,			/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	int length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildBgerror`, lines 2342–2363. Full-source SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`; snippet SHA-256 `95f6464606cd1776ff7093b3f2684bb0526a7be04579c833c7d41d80c04434fc`; retained evidence `ea3de3c863d05700ccfc3`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    Tcl_Size objc,		/* Set or Query. */
    Tcl_Obj *const objv[])	/* Argument strings. */
{
    if (objc) {
	Tcl_Size length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildBgerror`, lines 2727–2748. Full-source SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`; snippet SHA-256 `b24ea4bbf172c6ec2d9616a8c1e45d57d1ccf1535b43124d30a3b57ce95215bc`; retained evidence `e791ac2cd6ff96da72529`.

```text
ChildBgerror(
    Tcl_Interp *interp,		/* Interp for error return. */
    Tcl_Interp *childInterp,	/* Interp in which limit is set/queried. */
    Tcl_Size objc,		/* Set or Query. */
    Tcl_Obj *const *objv)	/* Argument strings. */
{
    if (objc) {
	Tcl_Size length;

	if (TCL_ERROR == TclListObjLength(NULL, objv[0], &length)
		|| (length < 1)) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "cmdPrefix must be list of length >= 1", -1));
	    Tcl_SetErrorCode(interp, "TCL", "OPERATION", "INTERP",
		    "BGERRORFORMAT", (char *)NULL);
	    return TCL_ERROR;
	}
	TclSetBgErrorHandler(childInterp, objv[0]);
    }
    Tcl_SetObjResult(interp, TclGetBgErrorHandler(childInterp));
    return TCL_OK;
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


## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `report_bg_error`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `process_bg_errors`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
