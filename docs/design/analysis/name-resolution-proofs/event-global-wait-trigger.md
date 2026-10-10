# naming.event.global-wait-trigger

Kind: `native-observation`

## Problem statement

Polling rendered caller variables cannot implement the selected global wait trigger.

## Question

Do these global scalar/array waits stop on an unchanged write, an unset, or only a changed value after the current timer turn?

## Conclusion

C wakes on a global write/unset trace, whereas Jim compares retained global values after processing events. Deterministic v110 nested registration yields SAME/OLD on all five C providers and SAME NEXT/NEW on Jim. Same-turn v106 writes both run before the wait returns; unsetting yields existence 0 on all six. The v107 staggered rows are timing-conditioned observations, not deadline-independent laws.

## Scope

Seven finite source controls across v106/v107/v110, fresh interpreter per case. Global subject lookup/trigger and actual returned values only. No unlimited notifier availability, signal/body options, trace-free ordinary read, or physical slot/lifetime claim. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'OLD' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'OLD'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/SAME_WRITE: SAME_WRITE: code 0, bytes b'SAME NEXT' v106/ARRAY_SAME_WRITE: ARRAY_SAME_WRITE: code 0, bytes b'NEW' v106/UNSET_WRITE: UNSET_WRITE: code 0, bytes b'0' v107/SAME_STAGGER: SAME_STAGGER: code 0, bytes b'SAME NEXT' v107/ARRAY_STAGGER: ARRAY_STAGGER: code 0, bytes b'NEW' v110/SAME_NESTED: SAME_NESTED: code 0, bytes b'SAME NEXT' v110/ARRAY_NESTED: ARRAY_NESTED: code 0, bytes b'NEW'

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance build attached.. Channel: not exercised. Dialect: bigip.

No BIG-IP provider was executed or inspected for this Event question.

## Exact evidence

- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`. Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e2f9702e19cdde19e5959` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/receipt.json). SHA-256 `2b14388098cfd871ab50d26acf3a6b6c631d0de2930f7b45204298ca7b26d344`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e50602151a40b9938af06` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stdout.tsv). SHA-256 `96796bb3ed015a46edd8ad53f1c16187a90251c554c9d1200e46a1f2003cc6d6`. Unmodified original stdout, including partial rows when externally timed out.
- `eb029205417a105e12c46` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ed35df3983d9d47f766da` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/receipt.json). SHA-256 `a65f39797f9bed9384da3f8bb29fd306e0d6ea1eb6cea095bdd4ce8524536f4f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e812a02d6c53140c521b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `58fa18c03cc3b03cb1bef60bb2a1bbbdaf29f7f8d5557610aabf63cf62348f02`. Unmodified original stdout, including partial rows when externally timed out.
- `e93d0bb942b35f0ac36fa` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef368fd443939e7aad345` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/receipt.json). SHA-256 `4b47a9f93e7efd21c2d5de5f4de2eeccef38ace1276f8d1eec72fc0743d319e5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee8856bb4b5e61800a239` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stdout.tsv). SHA-256 `74d6fb715d2796e91ef932f895046da9f722c68a39f41bc0c90e217dcce29811`. Unmodified original stdout, including partial rows when externally timed out.
- `e8ccbc6059640d42a87b9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec7eec673ea2335416d68` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/receipt.json). SHA-256 `e8156868dbf4dd80168275ef267ee1064eaad5a522628f0f67df30a7be1d1be8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3e97ed30e9df1cf4b664` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stdout.tsv). SHA-256 `18c9718b9f9605084a282d246ffbca94929d4a487aac2f97127d8d0577efc3e8`. Unmodified original stdout, including partial rows when externally timed out.
- `e41bbfa82065f9fc329c9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e2106861514c4bab46c8d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/receipt.json). SHA-256 `142a6a0865683058c87f3e08bb901430acae59879e8b2db178eaa9fd37ba7ed8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea3ca61315f4a1c839087` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `dc9dbf7143eb540569a9182fde689eaed2625cc00208aaaffe340545b5cc45b0`. Unmodified original stdout, including partial rows when externally timed out.
- `e0b9ff68648da0cfd74cc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8a9e8a29c21d575d9aae` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/receipt.json). SHA-256 `30a5b1b15c3f59f454bf8d12293d1cfa718f480d52eb2baf88c17aaaab9708df`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e60d24545805acf1811fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stdout.tsv). SHA-256 `87a83f1d3095dcf9f2d2a4f82c607b8513c813d33ecd24bdd94bd47a0b01416d`. Unmodified original stdout, including partial rows when externally timed out.
- `e8a9a046ab3a237d8908a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ed704e19420d173670a02` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/receipt.json). SHA-256 `cce1e020e40aed7fc1a6b8fedd9728ac833af941fb3fd1fdd08847c190cf2111`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3dfbf59c5ee7fbcb5034` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stdout.tsv). SHA-256 `b73bba3bed1161d2c777d6db1d55292037eb128980a4dffa5d57dc4aa9b3da65`. Unmodified original stdout, including partial rows when externally timed out.
- `e03224d21c4c87f29eb1a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `efb5f9f11c7dae0ba804c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/receipt.json). SHA-256 `575bb69b8b16c862e9799f18d7390e024195aa6c36ba0d280beebbde21d85a96`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e504fa070c6f5d4e271ac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `ba6656db652e25cf77b7bd659368f64dc765b7ee63b7df73470b0d6deb30acb1`. Unmodified original stdout, including partial rows when externally timed out.
- `e80b264d261c0f3db47b5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e625e4b0bfc4ac486c7d2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/receipt.json). SHA-256 `24093217ad5216dc9285f8db7a12ef4b62be23dcc439289eff5f2251a47c660c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee39a4b4ed85a5b2bce1d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stdout.tsv). SHA-256 `734861a1d1f7a0ac75a8b7ae802536a1b4017051de9b499cf848f67f8b1826cd`. Unmodified original stdout, including partial rows when externally timed out.
- `e64f81976c8e23ebbd246` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ed2dfd77654b5b34db08b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/receipt.json). SHA-256 `77a17595196a1d9a03188a89205cab9036a336f65ef3b7bfb46d6732c701bb40`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e95be77c51fdbc8296e39` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stdout.tsv). SHA-256 `9cdf52c2ea48e3c6d01a1f7852b48e4a76d47713950c69e17167559a5930dde7`. Unmodified original stdout, including partial rows when externally timed out.
- `e304496a389bea9526b3d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e05828d73d31496b4248e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/receipt.json). SHA-256 `600945146d1c47e1a5381606b1690f95c3aa1e036f821ac9cd421d553fd7a4d2`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee0c1a573aa3f7120a562` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `c972d9b5ea1d81b9dc604318542b8f426dd62d314a6e0a58ac7630e551ed5fd4`. Unmodified original stdout, including partial rows when externally timed out.
- `e742f9220c2352cce39cf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e669aedb6ddf4238ec209` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/receipt.json). SHA-256 `5322a960adf3322553d3697e9c57affe38726175f7e99ecb784a3544d7751417`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb4e84e926d82eecc98cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stdout.tsv). SHA-256 `ddced4f8750752faa1ff8e6e030e65d4da6dff02a81864efb0d5bf0bf874b0b4`. Unmodified original stdout, including partial rows when externally timed out.
- `ec744ca4997170110b8fe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e02b39f8e6384033b7cc5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/receipt.json). SHA-256 `7b80de8577ed3c5310e53459f8306bb693887839a271a3924968fb0fda006304`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec17d8d7e67f43bd83a5c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stdout.tsv). SHA-256 `1785571ed5f9e9b3e94c704d5e1220fb0f170fbc6c40b5ee8f65bb01655f9b1b`. Unmodified original stdout, including partial rows when externally timed out.
- `e0ecb2ad186007cc114e9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e61ab276cb86b9c798097` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/receipt.json). SHA-256 `ce12ea93338f7f0c8975a5cd7299e4ac9a28adc86a8c3cd51049fef7e932eafd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e51687290d1684dd7311c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `c575c25cd618f3fe08f4490c70150a5e22445a0385c5419b55ad92620295d117`. Unmodified original stdout, including partial rows when externally timed out.
- `e63ca5d243e22c3901da6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e7a44edadc05e4d445adc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/receipt.json). SHA-256 `82222591add029ee29690485dcf7c534d42a150b38a949962837bb2eebad7bc5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e0bf8f0bf1551700f91a4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stdout.tsv). SHA-256 `5910622a7a1a8e3d4a60cc807f047d74a01a26c938e1c7ed6ea45d9cd11f6dce`. Unmodified original stdout, including partial rows when externally timed out.
- `e37f412abe3c571fa4a69` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea188e980c5f37553ed0a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/receipt.json). SHA-256 `d7a38acdcb46ce403620e591658b29d6fc8ab187076fa5f82a08bafcf5e4befd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6cdd6b699bcaec9ae025` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stdout.tsv). SHA-256 `ff667ac57f6bf5c8b692ae51036d2471e704a086eb89db61bffac5cd0ae6180d`. Unmodified original stdout, including partial rows when externally timed out.
- `e8c087d856d315425bf0b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed38bf2850a70a63ce41f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/receipt.json). SHA-256 `101329ee2acbcfa8b0920726c14a52c079df57e315dc0edc25ed6cd62128a51b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebdf9dacb6afa3a72889c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stdout.tsv). SHA-256 `6cac8916074bd8fb76c28c979c645967051f46d262b71d43882adcb45c4adb6a`. Unmodified original stdout, including partial rows when externally timed out.
- `e51d0eb576539f77bf18d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ee432b6085ef1e6b0e769` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/receipt.json). SHA-256 `f611fd1cd12f9a0387046707965b936d6669028fa1d9ad586be5344eefa8bc86`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e686339a9cdf130452eca` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stdout.tsv). SHA-256 `5c1ba0800d70896f2651d3176284980e5808035dcb3d28dfab8ac3e64396aa2c`. Unmodified original stdout, including partial rows when externally timed out.
- `e846d6c87e23c044acd84` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e63555db492bc59dec4a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/receipt.json). SHA-256 `9f1ed4ff21e27447e99a8868dcef109530223d9f7c5d406b3513f56d778f6078`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef4b6ee6f77847137ef94` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stdout.tsv). SHA-256 `61e8025957526727a1a64ba2e7b16b9d19a34a2a14273b81ee3b52cd072a8807`. Unmodified original stdout, including partial rows when externally timed out.
- `e5db503c263f70680115e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). SHA-256 `08d5e613d23c9319712feb1d713b91a0f3e59e0418f527147a09d31df2dfc28f`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). SHA-256 `c52f03c8c20bab56f0a2e905d6c8c42d33235a2ede36b83649ffa5d35256b152`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). SHA-256 `f16fe8da41ee99ee01a2a1098d1b18d1a47decbdfaaabd13052ce18dd2e5508b`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). SHA-256 `e063fe10f9f6de3c8ab5fc1034cb42009c078044819104d2d1a635ff215ddbf2`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). SHA-256 `8625f645ffd0d2c22375052395dcb11ea7ae60a1b6d6d49fefca8bbcc4c927fb`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e81916b6ba75efea37403` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/receipt.json). SHA-256 `4914538b7867312c419bcc54a8dc59248abac2e768ac92a4d8a75f450031a92e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeb36599c3215f08f2c4b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stdout.tsv). SHA-256 `cd72274566a1956d35e2e9b3f2b713fa52a96a44a9906fc1b65b88948213e18b`. Unmodified original stdout, including partial rows when externally timed out.
- `ecfaa1a92b88447f70332` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e2afd4f8493d3bdbf489c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/receipt.json). SHA-256 `94b041c93d1058da312534cad7c213e78dcc710b1b0f30732f655e2496bc4d2e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e311a698b39fb2c5b5fe3` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stdout.tsv). SHA-256 `9c8b4224c3ef2818858943854254e39d27e3b67c87d3320fc343871340f2f48b`. Unmodified original stdout, including partial rows when externally timed out.
- `ec1891a442aec7a35690a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). SHA-256 `49987b659897bfa0668cb0dfdf7417e6e3db4da8d6ac96d51435332e57bef843`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1111eeb336fa0b449143` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/receipt.json). SHA-256 `e408acb2aed7188dc3b33954f679596f5dd569325061ddc0720e879f691fb86c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e254c68c47a1dc1e89fd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stdout.tsv). SHA-256 `0f18f1dc4ada2d5d0afda52550dc7ddc6418e9badffd747ba0f37de78946122b`. Unmodified original stdout, including partial rows when externally timed out.
- `e26e9aafcf150286ea68d` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e726e310ed0b1beebfd9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/receipt.json). SHA-256 `bee917ee8864e3f20f4ec6d1581b79734da87534a1f9d08a291ceb6b742101e3`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1f12e4c0410c1f7d7597` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stdout.tsv). SHA-256 `413e5a89f0e7e25f9e32adcc51cbd8c7c132db81bae30f1ebe01ceb4634e8282`. Unmodified original stdout, including partial rows when externally timed out.
- `e4355469cdda835b3d35b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). SHA-256 `288681099d036401a5bcbdd258b2738c8d0b9bbd4fc157a0ab71de83e8d15015`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e795c27955d6dacf47479` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/receipt.json). SHA-256 `4021fa254ed4d400399c375b048b3bea71174a0ca140042baed46f0d2d82a91c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e29721bc0023735856a08` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stdout.tsv). SHA-256 `c4cde396820f71cc35181343297a069a0321fca9fede7c800199f84fc0cd8bcd`. Unmodified original stdout, including partial rows when externally timed out.
- `efde3f6e39dd4bdfe530e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1e2926d68d8c8125b14b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/receipt.json). SHA-256 `169c8315be8da9f7f868e6c0a29a6f43430092b1b3db2b068420fdaba56139f3`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec73ecc5813510f923fbc` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stdout.tsv). SHA-256 `0f7d7f189ea19b97250c6c195f6f210a8257b37af266341f21263b8506e85e6a`. Unmodified original stdout, including partial rows when externally timed out.
- `e95e38a2a0d4872903841` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). SHA-256 `b97df66ab92c5c474dcd2e84c6032e5580d67fc617a4bde0f0647848fbcec5db`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74851a9c97676b1fc01c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/receipt.json). SHA-256 `ef7e8afe2e728a8263e2e224b16b94f7fb4de7de316e9d6034345faaa210df25`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e98f7ba416a307b79935a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stdout.tsv). SHA-256 `34c1868f2daa6b64a5a98afca3a6152fa320f2c89f1899892805506124b1ce76`. Unmodified original stdout, including partial rows when externally timed out.
- `e895e0e1d8c9daf956ed7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e89c3b9113e0598316010` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/receipt.json). SHA-256 `325519141eec654d5df94717a7ea7c48e44a4fb5c8f461e738c971267393c090`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8f5c054e4729d00aae43` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stdout.tsv). SHA-256 `93f71aeb5b176eb03d88d24d1929213d44d7a473142369e52d9f329424c20e9f`. Unmodified original stdout, including partial rows when externally timed out.
- `e9460c8f055914b282d89` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). SHA-256 `605fc7740871917aabfb184678f9e0a2d77ecfc43c4e6b5c38a3b5a087916093`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eec087308347432c3a521` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/receipt.json). SHA-256 `fdb787f437b7e39269cd76b268dbe3bc6d5ceaff51bfcef449f7f7f2ff679ae1`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e11bc541b868fcfd18412` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stdout.tsv). SHA-256 `7199912c4b52c7100c0c4e96c58583910dfd6afb8cc45ea592400ecc2944e5a3`. Unmodified original stdout, including partial rows when externally timed out.
- `eaa1130656d3534bc8f78` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e48b2a3426c7892aa1c1f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/receipt.json). SHA-256 `5195004a0df316de1d47f414e481f248834a2164c0597a9aff307179f2a19944`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e540253596f19c00be5cf` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stdout.tsv). SHA-256 `adab6bfbd2a4183c2a66aba0db968b9bef80342f889231035bd4cfdd6dd99bdb`. Unmodified original stdout, including partial rows when externally timed out.
- `e6455dc73505aed94a2fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). SHA-256 `56acfee2e4d15207ae30905aa25c737b89c678e5873e652697cfe4a2373c87b9`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4c41998c75a2ba9161ef` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/receipt.json). SHA-256 `7c258ed12ac9f0451f79ade1953b51d5f9956d6845b1a11d0f792fcd6f3d8530`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6b1d6f14c4020eb38bf8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stdout.tsv). SHA-256 `95c7d211b69a68da2d17f39e1bddd824044577a294f27b4be554a4b0fcdd71c5`. Unmodified original stdout, including partial rows when externally timed out.
- `ec60d7b0b0284a5e5f912` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e7c34dde0c53985052c96` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/receipt.json). SHA-256 `6daee43804e9a6a2dbe85d89ac69a941423c8d72982a68e3e65ce1176cf584fa`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9bc2f5655bf9f5432ff2` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stdout.tsv). SHA-256 `df37e77acf9e0ac6af19dab49cd52dc586c6762e15c700afcfef05acc139b059`. Unmodified original stdout, including partial rows when externally timed out.
- `e53f606c21c0035a4d8ab` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e4abaf9e2310a73659fd1` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/receipt.json). SHA-256 `db047887452e85c8527c53be858d35758e9792d3f102e09743c9f00e4c2fdcdc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e937f4c1fc354adc2b00e` (input): [rust/tcl-registry/tests/data/native_event_original/v110/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v110/probe.c). SHA-256 `c0b99dcec4d0610a262c1d589f9eba13ab04469ff88ca3c8d858c2b52e99f4ac`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e86edb0e779ef261853b1` (input): [rust/tcl-registry/tests/data/native_event_original/v110/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/inputs.json). SHA-256 `7320e8b67f9aa9a4e9f71a2c1378ddc9a4639eaa858d9d89c88980be4ba70c70`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e11ad7d3e6ca0017d7dac` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture.py). SHA-256 `c48def08a8e2a7a4b67bfa5f1bf64c1ea5ce7343c0fd0a45061260880b8b6765`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e82b1b87503ea6330233e` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/queue.json). SHA-256 `23645c331af7af4dca8b69299db9d90ec89a7facdab537e4e65ee5d7e9a791b4`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ebe25f4185c511c53f673` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/receipt.json). SHA-256 `91e0851aafa63dd6b6de5347dddd1dc7f79179e01e33b82e0254fca105802449`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eee7511390183d6b0be48` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stdout.tsv). SHA-256 `ca923032dc8964499487e473f7315b943ba6bc1cfc28fd7e040142b926b9b4de`. Unmodified original stdout, including partial rows when externally timed out.
- `e599d187be8f51790be14` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e9e7da6b95aa88e96b795` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/receipt.json). SHA-256 `b9174fd5cb49de520ce13c6770cb02790abae9616b2973aa0eaa7882dd002716`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed0356bd1549efcda4874` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stdout.tsv). SHA-256 `f2f3ba80ca225edf163469cfc2a0b31322471e69b5b2b4f1665ad6c446aeb0d1`. Unmodified original stdout, including partial rows when externally timed out.
- `e37f5ff98bb89c68f3b64` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5352c1d3f62c2eab3a7c` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/receipt.json). SHA-256 `17b418faca7b25da29bda38e58c89be9b24952566ae311a69a8928d09315fe7c`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1602a5f87f0a79a5c1d8` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/receipt.json). SHA-256 `2ec137e3f15a360d4fe3753b9d453d0e9cb7be9f67e37ce5b49acbe9354966aa`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e02424c25909c2b948766` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stdout.tsv). SHA-256 `10799a9e59f785d29d09c755b43886013fbaa044991a72d1ffb2ecdaba38a209`. Unmodified original stdout, including partial rows when externally timed out.
- `eceac53e6f780ab72a0e4` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e6abab5f2777db83bba88` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/receipt.json). SHA-256 `cf140bd212062cb02919652e4430234d83af0831d081ea77324bc5ceb3f97f60`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eaf62fc74493d7d23a633` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stdout.tsv). SHA-256 `7dcd325f1b63d28cbf9761e1c62b519316ccf84d81e5e408b0833ca930f058f3`. Unmodified original stdout, including partial rows when externally timed out.
- `ee0780541b2c87ecf2397` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e811d971d7bd79a2f0945` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/receipt.json). SHA-256 `68412a12396fc923f5f610b939a0e2a04bb893a4fbe2e50c41710a8c9393f2bd`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed99090b5fb234e2679ad` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/receipt.json). SHA-256 `17d09b67add4795d5fae3999a9e699742b238cd0d1f3cfeb5a1ba84156de2b12`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e565cfa65f9cc5c16b95a` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stdout.tsv). SHA-256 `e544779833d8093ad569b247d35a2e79a8f76fd1ed711d19ecaff6ed1c6368cb`. Unmodified original stdout, including partial rows when externally timed out.
- `ee5017a512beab740fb67` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8477748147741b87b721` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/receipt.json). SHA-256 `c1badcb38906c790ca23f0d2b6197aaeb06a605271a4931901ed25f50b025765`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e633c4e432fe514175496` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stdout.tsv). SHA-256 `3bc601f2f2d69230dea99d1f5e1fcf26dc4c16cba8905d2367b92fe9c38efb3c`. Unmodified original stdout, including partial rows when externally timed out.
- `e0c3c92d577a0203ae502` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eaa487ba939eedabcd8a6` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/receipt.json). SHA-256 `314bfa22bd8a4233bdb4bbf8d1b88d1b01f82ea0f3ffd86e8b852462f16324f1`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e634caf9e5dabb567ff60` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/receipt.json). SHA-256 `911fd7d68746d7887284ef80e8b9925b83e73c7b566af880ce9e1787fa60dd67`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4c856809b0364a007f0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stdout.tsv). SHA-256 `cda2221f0fa8dd31d4dbdf3fdc41e1ce53d0853d015740cb9bc48d8ca338cd9d`. Unmodified original stdout, including partial rows when externally timed out.
- `eac4021ca3115e6ee3677` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e2d7aeef03bb198a2c158` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/receipt.json). SHA-256 `ffc00b303bfc7ce17ce0242fac3dcacd846c2f51bdaa058763347a9878de7138`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3a8c4b38ee4bcbfc5f97` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stdout.tsv). SHA-256 `fe12b347c760215dcff68a9739674128f88100efb657a04c3046503849954304`. Unmodified original stdout, including partial rows when externally timed out.
- `ea8417d91100f662bd532` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8ac3540057c95c7ac803` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/receipt.json). SHA-256 `83e4444c4e9da6848029f260a4c3baa5104705d8b9b1537fa89f7e2f70e31e5b`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed49596e4d3c16fee8a93` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/receipt.json). SHA-256 `20430e86afb2b14469b6d6852c3986e4c1a3bdd9b4a7b7e31e5b9b6bc2154c19`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2529cf08b29d7b818c9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stdout.tsv). SHA-256 `8b917f68ae73fe6105c77d1a1c7d0ddf6af210d44292b599275563fbfb722766`. Unmodified original stdout, including partial rows when externally timed out.
- `ee917d56592dd487dc0cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef7c3f7c417488841dfdb` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/receipt.json). SHA-256 `e743fc1874877a603ce3325005d7b6ae94c95470c3d2ff85d52c74b2680b9064`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e657400aab6cdbbf8dccb` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stdout.tsv). SHA-256 `5c9474fce3f69857b31b75c8b4e07546daea6512e65a1f565a55d1c174f9131d`. Unmodified original stdout, including partial rows when externally timed out.
- `eaccb6bcd4005cd6e9fad` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb3ef7004c763d54513ba` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/receipt.json). SHA-256 `6b9c2549243fd7ee5b9f773ca9fb176d1c3e84bf792506176d2e31ae7b6d89ed`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed87f554161c81e00af40` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/receipt.json). SHA-256 `3bd1d64c72e556c97a0a78c64c52a8ee8c7e500ba271af4718d7e03242c852b8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `effb5018ee1a838b8e55b` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stdout.tsv). SHA-256 `c405c8f13e4dfa69a6dde8d90c53a3f0ead61351c79cfbdd56f79db2bae6a9c9`. Unmodified original stdout, including partial rows when externally timed out.
- `ed21d651003ac44d5a6a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.

## Source inspection

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `VwaitVarProc`, lines 1220–1231. Full-source SHA-256 `020cc8b9d0b020c2b1dac904f08b03c3e59889dd2ff493677f04696524d40673`; snippet SHA-256 `1e477af22d9a7a65f948075c243f4598545e58d84d5137a82caeec9cb6298f51`; retained evidence `e98d4bc726037374525cd`.

```text
VwaitVarProc(clientData, interp, name1, name2, flags)
    ClientData clientData;	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp;		/* Interpreter containing variable. */
    CONST char *name1;		/* Name of variable. */
    CONST char *name2;		/* Second part of variable name. */
    int flags;			/* Information about what happened. */
{
    int *donePtr = (int *) clientData;

    *donePtr = 1;
    return (char *) NULL;
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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `VwaitVarProc`, lines 1372–1383. Full-source SHA-256 `3d05eb479c767520e0b44459e91a0b4930cdb47139c4bfdb86e0beba6d04fb90`; snippet SHA-256 `687baa4208c9fbc9efc2f0d96a16ff69c3da3ffd9fb4559b518e3ddf1cdc8b27`; retained evidence `ebcd14ee056c41f642dc1`.

```text
VwaitVarProc(
    ClientData clientData,	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    CONST char *name1,		/* Name of variable. */
    CONST char *name2,		/* Second part of variable name. */
    int flags)			/* Information about what happened. */
{
    int *donePtr = (int *) clientData;

    *donePtr = 1;
    return NULL;
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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `VwaitVarProc`, lines 1450–1463. Full-source SHA-256 `81c0e0b655ecd46bd981750c9d710ec11dd1368b7f3f156886e0965c81788dd6`; snippet SHA-256 `f5f1223460517e696316e52932f2fdcf32d4f2bd681f237383668b1f30c79ed6`; retained evidence `e1799f8f23cd143629768`.

```text
VwaitVarProc(
    ClientData clientData,	/* Pointer to integer to set to 1. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    int flags)			/* Information about what happened. */
{
    int *donePtr = (int *)clientData;

    *donePtr = 1;
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `VwaitVarProc`, lines 1916–1933. Full-source SHA-256 `e6b69ac0fc6c9c335af90623398d06eb2d8ef7d2df5574327a52f622a37e31e4`; snippet SHA-256 `f2ef5df5e2c890abff496f7d99c3a5d765e3dac9d88bb570b3ded969a4c9f08f`; retained evidence `e30bb9be41c1824d28e5d`.

```text
VwaitVarProc(
    void *clientData,		/* Pointer to vwait info record. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    TCL_UNUSED(int) /*flags*/)	/* Information about what happened. */
{
    VwaitItem *itemPtr = (VwaitItem *) clientData;

    if (itemPtr->donePtr != NULL) {
	itemPtr->sequence = itemPtr->donePtr[0];
	itemPtr->donePtr[0] += 1;
	itemPtr->donePtr = NULL;
    }
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `VwaitVarProc`, lines 1955–1972. Full-source SHA-256 `9451a2c540dcd5ed61675c15f833f5ad677b551b61deb210ee6837216f477a54`; snippet SHA-256 `f2ef5df5e2c890abff496f7d99c3a5d765e3dac9d88bb570b3ded969a4c9f08f`; retained evidence `eaa5f919dc246958bc5d1`.

```text
VwaitVarProc(
    void *clientData,		/* Pointer to vwait info record. */
    Tcl_Interp *interp,		/* Interpreter containing variable. */
    const char *name1,		/* Name of variable. */
    const char *name2,		/* Second part of variable name. */
    TCL_UNUSED(int) /*flags*/)	/* Information about what happened. */
{
    VwaitItem *itemPtr = (VwaitItem *) clientData;

    if (itemPtr->donePtr != NULL) {
	itemPtr->sequence = itemPtr->donePtr[0];
	itemPtr->donePtr[0] += 1;
	itemPtr->donePtr = NULL;
    }
    Tcl_UntraceVar2(interp, name1, name2, TCL_TRACE_WRITES|TCL_TRACE_UNSETS,
	    VwaitVarProc, clientData);
    return NULL;
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

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `cmd_vwait`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/cmd_event.rs](../../../../runtime/rust/src/cmd_event.rs), `vwait_cmd`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_event_scripts_global_waits_and_timer_turns_match_native_controls` (linked): Finite code/result comparison for 21 retained source controls across six independently selected profiles (126 comparisons per port). No execution result is asserted by this binding.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
