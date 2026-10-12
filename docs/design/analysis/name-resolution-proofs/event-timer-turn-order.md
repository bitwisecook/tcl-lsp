# naming.event.timer-turn-order

Kind: `native-observation`

## Problem statement

Removing a whole copied batch before callbacks hides later cancellation and incorrectly admits new registrations into the same turn.

## Question

In these event controls, how do callback cancellation, callback registration, idle-only updates and a bare delay affect serviced scripts?

## Conclusion

All six captured BATCH_CANCEL controls return FIRST and BATCH_NEW returns FIRST SECOND NEW. UPDATE_IDLE ends at IDLE, with pinned source selecting C idle-only service and Jim timer service. DELAY_NO_EVENTS remains OLD. These are current live-entry and generation controls, not a microsecond timing guarantee.

## Scope

Four exact ASCII source controls per provider in v106/v107, including event-turn live cancellation and later registration. Does not establish external notifier inventory, asynchronous sources, concurrency, scheduler fairness or general event completion purity. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/UPDATE_IDLE: UPDATE_IDLE: code 0, bytes b'IDLE' v106/DELAY_NO_EVENTS: DELAY_NO_EVENTS: code 0, bytes b'OLD' v107/BATCH_CANCEL: BATCH_CANCEL: code 0, bytes b'FIRST' v107/BATCH_NEW: BATCH_NEW: code 0, bytes b'FIRST SECOND NEW'

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
- `e16abf8d36a4cd96fdedd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/receipt.json). SHA-256 `8c2642b1f471ebbccf3dd53d624242a65591a9140cec9533e0f4404bdffe424b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ecc7b88f2ba86b6a6d207` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stdout.tsv). SHA-256 `2636cf0a2b9b6bafe1968f58f58272a9bc8849d52cfef8868310f113fb7dfd82`. Unmodified original stdout, including partial rows when externally timed out.
- `ef13492b7b90ba099b904` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e9af9e52ab69d7fd45adf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/receipt.json). SHA-256 `95c0dc19e80516f528a89a076c0471d4477346e72440f58820a9eb7f3f83d549`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e15656cbf5f96a08b2f70` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `8f20f1d61a8831b3b2a255fff69520b444eba19687fd4d5464d2122cf0e8e1a2`. Unmodified original stdout, including partial rows when externally timed out.
- `e1033406b91429b3b25ee` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e0ba66e3f4e29bdad9c89` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/receipt.json). SHA-256 `006ae420f711e6fc0bface0f27b165137a79bbc03d92d5f54159505ab9ca3c54`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e432232a3f41aaf661a13` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stdout.tsv). SHA-256 `a17125f87d242528b1556bf67667cc2d7ca21e7db35a586b3069fd469ac30419`. Unmodified original stdout, including partial rows when externally timed out.
- `e614fe17b96901e8b9c30` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e157746e11ad028d93ed0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/receipt.json). SHA-256 `7c377843a7620781789e4573dbf84eeb1cc8861d8edc824407f96dc1b50c5a63`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef3da3504427efc398e9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `0b35d6e1e170a6c7f9ee1d20fdd7389854a4e540ed045f69fc5c893e2785e067`. Unmodified original stdout, including partial rows when externally timed out.
- `e939d9025bc2fa798fbe9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e4fc29bae82aaee45c43e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/receipt.json). SHA-256 `ffbc945279c0baefd2e463911eb5884b6d5b28adeac7b2e5d766e8667ecd3b7b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e7b86a4a15b410f2cb453` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stdout.tsv). SHA-256 `3b0830ad8154d16d8ac0c1f6dc55d39748125857b0d165d0b3ab6bf4fa342311`. Unmodified original stdout, including partial rows when externally timed out.
- `e0edb9dfab9a8548c9b4a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eb1faa95c6d0feeafa947` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/receipt.json). SHA-256 `a98659d674e94ae8618692ca52ad83fb9f9823c88ece4c4ce4c144698ae2c185`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea6ef2ce51d0357844c20` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `31d318d9e8fb983c6f650bf8cf568b956ac7a3a823d69470232db3aa6ac04415`. Unmodified original stdout, including partial rows when externally timed out.
- `e6bbc71a34a539cff8266` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efe8a70752fac9d2f45b1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/receipt.json). SHA-256 `2fb2359f813796cb286dc405c2070adddb219f146c1a22eb2f056357ad953ada`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9f5e5c9f29dd872c0fbb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stdout.tsv). SHA-256 `a4eabf2d1fe083804bba2a802758d4b03297341ee3f3cd0913fc735c58beef44`. Unmodified original stdout, including partial rows when externally timed out.
- `e7bdc99d73f78c5e33923` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea51409fa4a36ec27cbb9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/receipt.json). SHA-256 `fc724b87e64c2d6302e772b2a32d4669b77d07a78d0e6c6c0a70168d6e1a0eb2`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ead87b2028d3c70312c8e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `f9c4928c633c35908cac3b0a21e5dd8568d507919c2907874cdc11d818d55dc6`. Unmodified original stdout, including partial rows when externally timed out.
- `e119f894db4cdfe63c378` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ebd2d2e6ec4cb04eff758` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/receipt.json). SHA-256 `f07d60d25307a0e5963b68f63986b2d46682b7068a7acdd7c4ebc68c0cff417d`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e33dce9744f64c4a14234` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stdout.tsv). SHA-256 `47e1b977a86127fff04302b1392d268758217cdf5f4cb812f5f172b1b64a944c`. Unmodified original stdout, including partial rows when externally timed out.
- `ee0614cc6c827a86921e7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef938f1ce1764b6e5ca6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/receipt.json). SHA-256 `e6e9f43b5e4374e5779da64a8794c6736e38b27e7cb31449412478c10f0b6e24`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e36b8767c1a84886e124d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `6b6568724f27e61453aad5b99aa015ed4ca184dbc88bd4ca2326fc6d06b07af6`. Unmodified original stdout, including partial rows when externally timed out.
- `e9b760316ac0ffd76a77a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e405ee8549dc8c3f94ae5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/receipt.json). SHA-256 `b6b3d359cf63e610a8b410a22176f19813f41f7dcd3a1eb4b4674e54b37214a0`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e99c265c5eb4697ef73a7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stdout.tsv). SHA-256 `6c6f918d69d18a5ded3de0bd7268e7f046732d5a09a6937e948194c40a86f23c`. Unmodified original stdout, including partial rows when externally timed out.
- `ea1dc0a200cb7824f4763` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea57e7fab8a8e6bfff262` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/receipt.json). SHA-256 `f9cdaf669b408da4aef2cfaa32b3e188139cd4ecf24d4bec0562b6a03064b902`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e30f35d4091b85c3985f9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stdout.tsv). SHA-256 `769be7697db5e3eb6d909029320dd677d1b5c6e181213082977c48e2740937a7`. Unmodified original stdout, including partial rows when externally timed out.
- `e128f6324cbd19a3aa42d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eafef857662e635df0046` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/receipt.json). SHA-256 `2e8387c60a6eadfa3b4bfe7bdb2861daf3e3f98ecabfabaff714efc86ec48a61`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e83ffbcd7667167566326` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stdout.tsv). SHA-256 `8f8a58c88679cab4aec1feb5e207ea47fd46abb3d4072b0098db9d5946cfaf86`. Unmodified original stdout, including partial rows when externally timed out.
- `ea745d33ee0f23a2ae296` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). SHA-256 `08d5e613d23c9319712feb1d713b91a0f3e59e0418f527147a09d31df2dfc28f`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). SHA-256 `c52f03c8c20bab56f0a2e905d6c8c42d33235a2ede36b83649ffa5d35256b152`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). SHA-256 `f16fe8da41ee99ee01a2a1098d1b18d1a47decbdfaaabd13052ce18dd2e5508b`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). SHA-256 `e063fe10f9f6de3c8ab5fc1034cb42009c078044819104d2d1a635ff215ddbf2`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). SHA-256 `8625f645ffd0d2c22375052395dcb11ea7ae60a1b6d6d49fefca8bbcc4c927fb`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e0f4ade737b6fa4540810` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/receipt.json). SHA-256 `2c75281c5077409067524b19e5a6d86a8b7f5b7f5b197b29c8b2b3e6f6cb2689`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef861641ef2e7e5fce7cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stdout.tsv). SHA-256 `af8debbb525e03cdeac06401291e1505f3e032d7b251b5c9603660da285af226`. Unmodified original stdout, including partial rows when externally timed out.
- `ee2def82b0d1c58115709` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1b56c0fc185e889741f7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/receipt.json). SHA-256 `0b4ce513c4dc46afa34c35ebba9d8c2ee1400fca3d50264b764259cd14597b4e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e64285d3703241e67e276` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stdout.tsv). SHA-256 `f519da5e4f878f8e32325b0f7005c35bc8c9acdfe90ac5d24d5e1686cd659968`. Unmodified original stdout, including partial rows when externally timed out.
- `e8cf32fd9829794001e31` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). SHA-256 `49987b659897bfa0668cb0dfdf7417e6e3db4da8d6ac96d51435332e57bef843`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e7747ef839deab5013c55` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/receipt.json). SHA-256 `4d2ead8c9ee29e1faf1f6143be0bbc2e29f9ab14123c545cf815be32a0c3c688`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2a62d48715c44a14b2da` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stdout.tsv). SHA-256 `86fca0cc42ba3b64e5ebfc7373b7f7d2b3f92d4f74172331e2e08d217ca8813d`. Unmodified original stdout, including partial rows when externally timed out.
- `e55ab4851c32ea7c840eb` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09e98de05b86472833b6` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/receipt.json). SHA-256 `b789339ed812a3fa4957911fef7002bc722fafc4ff5e2066944903532f7c0259`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e37ce5f23df39410a235e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stdout.tsv). SHA-256 `150ebe2a37c0badfd1fe1da051f0eddefdcb40a1376408fab0662319b86b33da`. Unmodified original stdout, including partial rows when externally timed out.
- `ee04b418bee6135e8d624` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). SHA-256 `288681099d036401a5bcbdd258b2738c8d0b9bbd4fc157a0ab71de83e8d15015`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e8d9efddd9335e237ece4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/receipt.json). SHA-256 `8f1480c67b2e154b24ee910cc1de46828549159dc7d0e510eabedcb0d0b0ba8b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e56b5b4f35a4489239046` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stdout.tsv). SHA-256 `f6c8fa162e2ef35e56d61bb6b7d5e761b8c7dd0cc24f4162165ee89f80a1c1e4`. Unmodified original stdout, including partial rows when externally timed out.
- `e540f8b59a0b03a1b4448` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8c8f0d70afa56228b3a3` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/receipt.json). SHA-256 `027b3b01bd838ef2fc17599285b49f58739e24adc19573e8561041f3097fcd38`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea0272a7551bac69d0a97` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stdout.tsv). SHA-256 `c84701e134ee67f84a432a72a6e3f0fe033030c31b5d36df6b787be0ded5fec0`. Unmodified original stdout, including partial rows when externally timed out.
- `ed27ad988f74aaaded0c7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). SHA-256 `b97df66ab92c5c474dcd2e84c6032e5580d67fc617a4bde0f0647848fbcec5db`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ec7d73937c1fb9ece2ef9` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/receipt.json). SHA-256 `1fe2e2b73bcc39c448bbe140455deb38cd0b47da631b419bfe6d5b21ff5355aa`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb12444f96efc1dcd160b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stdout.tsv). SHA-256 `d4fa06b4d51a29fc8796f8ab422f9b212a97749c1b13dc8625047ff10ccfd2c4`. Unmodified original stdout, including partial rows when externally timed out.
- `e960c4f9354c6e35b3ae8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec43f65a980f16d062796` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/receipt.json). SHA-256 `e1e689e8a812a10658ac8fe6e0c0fe754f9300825501afeaafb4d48a2f3d6626`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8c45f1690171312814fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stdout.tsv). SHA-256 `bd7b09f3797e812a433258a256caef870f6902d8976547c5417b349cf4b915c7`. Unmodified original stdout, including partial rows when externally timed out.
- `ea147dbcac9d0f33e46db` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). SHA-256 `605fc7740871917aabfb184678f9e0a2d77ecfc43c4e6b5c38a3b5a087916093`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e7a06ea6585321890a732` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/receipt.json). SHA-256 `1e5b821ca772a4fa8af231958add2838fd85d69beada88686a31546284556bfe`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea1a8fb8f4e3a1a712b2b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stdout.tsv). SHA-256 `c72ddfad6b387df7d7c47118982b92fc346720efcac0124e147782974b632af0`. Unmodified original stdout, including partial rows when externally timed out.
- `edaf242a10c2278197cfe` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ed8535c87807ff404fe3e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/receipt.json). SHA-256 `9c50c67e46f9813a97407880d357da0ecebb6b8c0458d48ce0bad77a31faca6f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb65a60180ea7caadcbc9` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stdout.tsv). SHA-256 `8ebec0efa23fb3bed48ec1d0db995be30408fc922291f38386da00cc991b2c25`. Unmodified original stdout, including partial rows when externally timed out.
- `e0ce4fc941a14062c9c54` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). SHA-256 `56acfee2e4d15207ae30905aa25c737b89c678e5873e652697cfe4a2373c87b9`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e9dd8cbefc412c3e219f1` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/receipt.json). SHA-256 `fd5361e51c1fd249e781702e2bafc3c6d189b6bc4bf468e14f6b45474e2193fa`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e207fc2c5c3d4b5901696` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stdout.tsv). SHA-256 `b2fea189e0f66690136006bff3a21a1c018dfd7a158a02031cfabaa620b873e6`. Unmodified original stdout, including partial rows when externally timed out.
- `e9e21e2de9c7189b0a8f4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 467–549. Full-source SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`; snippet SHA-256 `8eee6fce0ddf7e7a411f6f9e896cc7bbb1d45e1895ee7eece64c05aae59e843b`; retained evidence `e7ca34f2d6fd9344d4610`.

```text
TimerHandlerEventProc(evPtr, flags)
    Tcl_Event *evPtr;		/* Event to service. */
    int flags;			/* Flags that indicate what events to
				 * handle, such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled.  This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called
     * with timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following
     * reasons:
     *
     * 1. New handlers can get added to the list while the current
     *    one is being processed.  If new ones get added, we don't
     *    want to process them during this pass through the list to avoid
     *	  starving other event sources.  This is implemented using the
     *	  token number in the handler:  new handlers will have a
     *    newer token than any of the ones currently on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove
     *    the handler from the list before calling it. Otherwise an
     *    infinite loop could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from
     *    the list while a handler is executing, so the list could
     *    change structure during the call.
     * 4. Because we only fetch the current time before entering the loop,
     *    the only way a new timer will even be considered runnable is if
     *	  its expiration time is within the same millisecond as the
     *	  current time.  This is fairly likely on Windows, since it has
     *	  a course granularity clock.  Since timers are placed
     *	  on the queue in time order with the most recently created
     *    handler appearing after earlier ones with the same expiration
     *	  time, we don't have to worry about newer generation timers
     *	  appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}
	    
	if ((timerHandlerPtr->time.sec > time.sec)
		|| ((timerHandlerPtr->time.sec == time.sec)
			&& (timerHandlerPtr->time.usec > time.usec))) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - (int)timerHandlerPtr->token) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it,
	 * to avoid potential reentrancy problems.
	 */

	(*nextPtrPtr) = timerHandlerPtr->nextPtr;
	(*timerHandlerPtr->proc)(timerHandlerPtr->clientData);
	ckfree((char *) timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
}
```

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1252–1295. Full-source SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`; snippet SHA-256 `84dfd0e1ec089a7b4be43581d176442092c7806109e71ddbfe1a2bf534fce0d9`; retained evidence `e98d4bc726037374525cd`.

```text
Tcl_UpdateObjCmd(clientData, interp, objc, objv)
    ClientData clientData;	/* Not used. */
    Tcl_Interp *interp;		/* Current interpreter. */
    int objc;			/* Number of arguments. */
    Tcl_Obj *CONST objv[];	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static CONST char *updateOptions[] = {"idletasks", (char *) NULL};
    enum updateOptions {REGEXP_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptions) optionIndex) {
	    case REGEXP_IDLETASKS: {
		flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
		break;
	    }
	    default: {
		panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	    }
	}
    } else {
        Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }
    
    while (Tcl_DoOneEvent(flags) != 0) {
	/* Empty loop body */
    }

    /*
     * Must clear the interpreter's result because event handlers could
     * have executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 505–583. Full-source SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`; snippet SHA-256 `355dbea6b6e44f00264b51bae10c6f7f2d323221a01000e231f0721331e4672c`; retained evidence `eb83569215d92fa37da10`.

```text
TimerHandlerEventProc(
    Tcl_Event *evPtr,		/* Event to service. */
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	(*nextPtrPtr) = timerHandlerPtr->nextPtr;
	(*timerHandlerPtr->proc)(timerHandlerPtr->clientData);
	ckfree((char *) timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
}
```

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1404–1449. Full-source SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`; snippet SHA-256 `069ae6de945edcfce45705a5a28df7642a4510373638f76d3e68b8d907ec6920`; retained evidence `ebcd14ee056c41f642dc1`.

```text
Tcl_UpdateObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *CONST objv[])	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static CONST char *updateOptions[] = {"idletasks", NULL};
    enum updateOptions {REGEXP_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptions) optionIndex) {
	case REGEXP_IDLETASKS:
	    flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    Tcl_Panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_AppendResult(interp, "limit exceeded", NULL);
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 520–598. Full-source SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`; snippet SHA-256 `f70847309a2838e410a469aa647c3669e28153ea3496eb393d60e6bd403aaf44`; retained evidence `e5cad7b4101332ff96f7c`.

```text
TimerHandlerEventProc(
    Tcl_Event *evPtr,		/* Event to service. */
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	*nextPtrPtr = timerHandlerPtr->nextPtr;
	timerHandlerPtr->proc(timerHandlerPtr->clientData);
	ckfree(timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
}
```

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1483–1531. Full-source SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`; snippet SHA-256 `e3d8e596f696f178558290a5baea28a7d9f823c12515b64acb7a2671b994cf4a`; retained evidence `e1799f8f23cd143629768`.

```text
Tcl_UpdateObjCmd(
    ClientData clientData,	/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int optionIndex;
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS};

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch ((enum updateOptionsEnum) optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_WINDOW_EVENTS|TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    Tcl_Panic("Tcl_UpdateObjCmd: bad option index to UpdateOptions");
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 517–595. Full-source SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`; snippet SHA-256 `30979be71084db83492c85e45cf92d02bc0bb5612397976507205ed4d1332e71`; retained evidence `e4d5b46a62af04d177edb`.

```text
TimerHandlerEventProc(
    TCL_UNUSED(Tcl_Event *),
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    TimerHandler *timerHandlerPtr, **nextPtrPtr;
    Tcl_Time time;
    int currentTimerId;
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    tsdPtr->timerPending = 0;
    currentTimerId = tsdPtr->lastTimerId;
    Tcl_GetTime(&time);
    while (1) {
	nextPtrPtr = &tsdPtr->firstTimerHandlerPtr;
	timerHandlerPtr = tsdPtr->firstTimerHandlerPtr;
	if (timerHandlerPtr == NULL) {
	    break;
	}

	if (TCL_TIME_BEFORE(time, timerHandlerPtr->time)) {
	    break;
	}

	/*
	 * Bail out if the next timer is of a newer generation.
	 */

	if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
	    break;
	}

	/*
	 * Remove the handler from the queue before invoking it, to avoid
	 * potential reentrancy problems.
	 */

	*nextPtrPtr = timerHandlerPtr->nextPtr;
	timerHandlerPtr->proc(timerHandlerPtr->clientData);
	Tcl_Free(timerHandlerPtr);
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
}
```

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1953–2000. Full-source SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`; snippet SHA-256 `bd93821bbae5dbca7a8b7ba7747df9dcdfb13f8fa2ea3a4e93795d2318f39d2b`; retained evidence `e30bb9be41c1824d28e5d`.

```text
Tcl_UpdateObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS} optionIndex;

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 744–841. Full-source SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`; snippet SHA-256 `410e17893c54f43d64c225dfe1efe3950426cc8ee537190fcc4d878f68c0f30b`; retained evidence `eaf331dbfda81fff6c4db`.

```text
TimerHandlerEventProc(
    TCL_UNUSED(Tcl_Event *),
    int flags)			/* Flags that indicate what events to handle,
				 * such as TCL_FILE_EVENTS. */
{
    ThreadSpecificData *tsdPtr = InitTimer();

    /*
     * Do nothing if timers aren't enabled. This leaves the event on the
     * queue, so we will get to it as soon as ServiceEvents() is called with
     * timers enabled.
     */

    if (!(flags & TCL_TIMER_EVENTS)) {
	return 0;
    }

    /*
     * The code below is trickier than it may look, for the following reasons:
     *
     * 1. New handlers can get added to the list while the current one is
     *	  being processed. If new ones get added, we don't want to process
     *	  them during this pass through the list to avoid starving other event
     *	  sources. This is implemented using the token number in the handler:
     *	  new handlers will have a newer token than any of the ones currently
     *	  on the list.
     * 2. The handler can call Tcl_DoOneEvent, so we have to remove the
     *	  handler from the list before calling it. Otherwise an infinite loop
     *	  could result.
     * 3. Tcl_DeleteTimerHandler can be called to remove an element from the
     *	  list while a handler is executing, so the list could change
     *	  structure during the call.
     * 4. Because we only fetch the current time before entering the loop, the
     *	  only way a new timer will even be considered runnable is if its
     *	  expiration time is within the same millisecond as the current time.
     *	  This is fairly likely on Windows, since it has a course granularity
     *	  clock. Since timers are placed on the queue in time order with the
     *	  most recently created handler appearing after earlier ones with the
     *	  same expiration time, we don't have to worry about newer generation
     *	  timers appearing before later ones.
     */

    for (int timerHandlerIndex = timerHandlerMonotonic;
	    timerHandlerIndex <= timerHandlerWallclock; timerHandlerIndex++) {
	long long timeUS;
	TimerHandler *timerHandlerPtr, **nextPtrPtr;
	int currentTimerId;

	if (!tsdPtr->timerPendingQueue[timerHandlerIndex]) {
	    continue;
	}
	tsdPtr->timerPendingQueue[timerHandlerIndex] = 0;

	/*
	 * As the queues are independent, but the timer ids are used for both,
	 * its creation time should be compared per queue.
	 * Thus, use the queues latest id.
	 */

	currentTimerId = tsdPtr->lastTimerIdQueue[timerHandlerIndex];

	if (timerHandlerIndex == timerHandlerMonotonic) {
	    timeUS = Tcl_GetMonotonicTime();
	} else {
	    timeUS = Tcl_GetDayTime();
	}
	while (1) {
	    nextPtrPtr = &tsdPtr->firstTimerHandlerPtr[timerHandlerIndex];
	    timerHandlerPtr = tsdPtr->firstTimerHandlerPtr[timerHandlerIndex];
	    if (timerHandlerPtr == NULL) {
		break;
	    }

	    if (timeUS < timerHandlerPtr->time) {
		break;
	    }

	    /*
	     * Bail out if the next timer is of a newer generation.
	     */

	    if ((currentTimerId - PTR2INT(timerHandlerPtr->token)) < 0) {
		break;
	    }

	    /*
	     * Remove the handler from the queue before invoking it, to avoid
	     * potential reentrancy problems.
	     */

	    *nextPtrPtr = timerHandlerPtr->nextPtr;
	    timerHandlerPtr->proc(timerHandlerPtr->clientData);
	    Tcl_Free(timerHandlerPtr);
	}
    }
    TimerSetupProc(NULL, TCL_TIMER_EVENTS);
    return 1;
}
```

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1992–2039. Full-source SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`; snippet SHA-256 `e56e6a2cce1bfcc8b280959702c3586431d1974ba4eb6fa3e97ab1b480c8536e`; retained evidence `eaa5f919dc246958bc5d1`.

```text
Tcl_UpdateObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    int flags = 0;		/* Initialized to avoid compiler warning. */
    static const char *const updateOptions[] = {"idletasks", NULL};
    enum updateOptionsEnum {OPT_IDLETASKS} optionIndex;

    if (objc == 1) {
	flags = TCL_ALL_EVENTS|TCL_DONT_WAIT;
    } else if (objc == 2) {
	if (Tcl_GetIndexFromObj(interp, objv[1], updateOptions,
		"option", 0, &optionIndex) != TCL_OK) {
	    return TCL_ERROR;
	}
	switch (optionIndex) {
	case OPT_IDLETASKS:
	    flags = TCL_IDLE_EVENTS|TCL_DONT_WAIT;
	    break;
	default:
	    TCL_UNREACHABLE();
	}
    } else {
	Tcl_WrongNumArgs(interp, 1, objv, "?idletasks?");
	return TCL_ERROR;
    }

    while (Tcl_DoOneEvent(flags) != 0) {
	if (Tcl_Canceled(interp, TCL_LEAVE_ERR_MSG) == TCL_ERROR) {
	    return TCL_ERROR;
	}
	if (Tcl_LimitExceeded(interp)) {
	    Tcl_ResetResult(interp);
	    Tcl_SetObjResult(interp, Tcl_NewStringObj("limit exceeded", -1));
	    return TCL_ERROR;
	}
    }

    /*
     * Must clear the interpreter's result because event handlers could have
     * executed commands.
     */

    Tcl_ResetResult(interp);
    return TCL_OK;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_ProcessEvents`, lines 377–534. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `a3e723ecda063b80222488e831cb5303e8a9d586ecd1c3b0857c265d3dcd606d`; retained evidence `e4521e47b782c5165c471`.

```text
int Jim_ProcessEvents(Jim_Interp *interp, int flags)
{
    jim_wide sleep_us = -1;
    int processed = 0;
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");
    Jim_FileEvent *fe = eventLoop->fileEventHead;
    Jim_TimeEvent *te;
    jim_wide maxId;

    if ((flags & JIM_FILE_EVENTS) == 0 || fe == NULL) {
        /* No file events */
        if ((flags & JIM_TIME_EVENTS) == 0 || eventLoop->timeEventHead == NULL) {
            /* No time events */
            return -1;
        }
    }

    /* Note that we want call select() even if there are no
     * file events to process as long as we want to process time
     * events, in order to sleep until the next time event is ready
     * to fire. */

    if (flags & JIM_DONT_WAIT) {
        /* Wait no time */
        sleep_us = 0;
    }
    else if (flags & JIM_TIME_EVENTS) {
        /* The nearest timer is always at the head of the list */
        if (eventLoop->timeEventHead) {
            Jim_TimeEvent *shortest = eventLoop->timeEventHead;

            /* Calculate the time missing for the nearest
             * timer to fire. */
            sleep_us = shortest->when - Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
            if (sleep_us < 0) {
                sleep_us = 0;
            }
        }
        else {
            /* Wait forever */
            sleep_us = -1;
        }
    }

#ifdef HAVE_SELECT
    if (flags & JIM_FILE_EVENTS) {
        int retval;
        struct timeval tv, *tvp = NULL;
        fd_set rfds, wfds, efds;
        int maxfd = -1;

        FD_ZERO(&rfds);
        FD_ZERO(&wfds);
        FD_ZERO(&efds);

        /* Check file events */
        while (fe != NULL) {
            if (fe->mask & JIM_EVENT_READABLE)
                FD_SET(fe->fd, &rfds);
            if (fe->mask & JIM_EVENT_WRITABLE)
                FD_SET(fe->fd, &wfds);
            if (fe->mask & JIM_EVENT_EXCEPTION)
                FD_SET(fe->fd, &efds);
            if (maxfd < fe->fd)
                maxfd = fe->fd;
            fe = fe->next;
        }

        if (sleep_us >= 0) {
            tvp = &tv;
            tvp->tv_sec = sleep_us / 1000000;
            tvp->tv_usec = sleep_us % 1000000;
        }

        retval = select(maxfd + 1, &rfds, &wfds, &efds, tvp);

        if (retval < 0) {
            if (errno == EINVAL) {
                /* This can happen on mingw32 if a non-socket filehandle is passed */
                Jim_SetResultString(interp, "non-waitable filehandle", -1);
                return -2;
            }
        }
        else if (retval > 0) {
            fe = eventLoop->fileEventHead;
            while (fe != NULL) {
                int mask = 0;
                int fd = fe->fd;

                if ((fe->mask & JIM_EVENT_READABLE) && FD_ISSET(fd, &rfds))
                    mask |= JIM_EVENT_READABLE;
                if (fe->mask & JIM_EVENT_WRITABLE && FD_ISSET(fd, &wfds))
                    mask |= JIM_EVENT_WRITABLE;
                if (fe->mask & JIM_EVENT_EXCEPTION && FD_ISSET(fd, &efds))
                    mask |= JIM_EVENT_EXCEPTION;

                if (mask) {
                    int ret = fe->fileProc(interp, fe->clientData, mask);
                    if (ret != JIM_OK && ret != JIM_RETURN) {
                        /* Remove the element on handler error */
                        Jim_DeleteFileHandler(interp, fd, mask);
                        /* At this point fe is no longer valid - it will be assigned below */
                    }
                    processed++;
                    /* After an event is processed our file event list
                     * may no longer be the same, so what we do
                     * is to clear the bit for this file descriptor and
                     * restart again from the head. */
                    FD_CLR(fd, &rfds);
                    FD_CLR(fd, &wfds);
                    FD_CLR(fd, &efds);
                    fe = eventLoop->fileEventHead;
                }
                else {
                    fe = fe->next;
                }
            }
        }
    }
#else
    if (sleep_us > 0) {
        usleep(sleep_us);
    }
#endif

    /* Check time events */
    te = eventLoop->timeEventHead;
    maxId = eventLoop->timeEventNextId;
    while (te) {
        jim_wide id;

        if (te->id > maxId) {
            te = te->next;
            continue;
        }
        if (Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW) >= te->when) {
            id = te->id;
            /* Remove from the list before executing */
            Jim_RemoveTimeHandler(eventLoop, id);
            te->timeProc(interp, te->clientData);
            /* After an event is processed our time event list may
             * no longer be the same, so we restart from head.
             * Still we make sure to don't process events registered
             * by event handlers itself in order to don't loop forever
             * even in case an [after 0] that continuously register
             * itself. To do so we saved the max ID we want to handle. */
            Jim_FreeTimeHandler(interp, te);

            te = eventLoop->timeEventHead;
            processed++;
        }
        else {
            te = te->next;
        }
    }

    return processed;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELUpdateCommand`, lines 647–670. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `3035ae6d80cfc350574ae10e6b9497acb90b38918f0bc343ae3930dbdb2b1126`; retained evidence `e4521e47b782c5165c471`.

```text
static int JimELUpdateCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    Jim_EventLoop *eventLoop = Jim_CmdPrivData(interp);
    static const char * const options[] = {
        "idletasks", NULL
    };
    enum { UPDATE_IDLE, UPDATE_NONE };
    int option = UPDATE_NONE;
    int flags = JIM_TIME_EVENTS;

    if (argc == 1) {
        flags = JIM_ALL_EVENTS;
    }
    else if (argc > 2 || Jim_GetEnum(interp, argv[1], options, &option, NULL, JIM_ENUM_ABBREV) != JIM_OK) {
        return JIM_USAGE;
    }

    eventLoop->suppress_bgerror = 0;

    while (Jim_ProcessEvents(interp, flags | JIM_DONT_WAIT) > 0) {
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
