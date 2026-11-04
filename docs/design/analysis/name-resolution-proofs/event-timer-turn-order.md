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

## Historical references

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e16abf8d36a4cd96fdedd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ecc7b88f2ba86b6a6d207` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ef13492b7b90ba099b904` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e9af9e52ab69d7fd45adf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e15656cbf5f96a08b2f70` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1033406b91429b3b25ee` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `e0ba66e3f4e29bdad9c89` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e432232a3f41aaf661a13` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e614fe17b96901e8b9c30` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e157746e11ad028d93ed0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef3da3504427efc398e9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e939d9025bc2fa798fbe9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `e4fc29bae82aaee45c43e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e7b86a4a15b410f2cb453` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0edb9dfab9a8548c9b4a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eb1faa95c6d0feeafa947` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea6ef2ce51d0357844c20` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e6bbc71a34a539cff8266` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `efe8a70752fac9d2f45b1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9f5e5c9f29dd872c0fbb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e7bdc99d73f78c5e33923` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea51409fa4a36ec27cbb9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ead87b2028d3c70312c8e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e119f894db4cdfe63c378` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `ebd2d2e6ec4cb04eff758` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e33dce9744f64c4a14234` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee0614cc6c827a86921e7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef938f1ce1764b6e5ca6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e36b8767c1a84886e124d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9b760316ac0ffd76a77a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `e405ee8549dc8c3f94ae5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e99c265c5eb4697ef73a7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea1dc0a200cb7824f4763` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_IDLE/stderr). Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea57e7fab8a8e6bfff262` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e30f35d4091b85c3985f9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e128f6324cbd19a3aa42d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/DELAY_NO_EVENTS/stderr). Unmodified original stderr for this isolated process.
- `eafef857662e635df0046` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e83ffbcd7667167566326` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea745d33ee0f23a2ae296` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e0f4ade737b6fa4540810` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef861641ef2e7e5fce7cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee2def82b0d1c58115709` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.
- `e1b56c0fc185e889741f7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e64285d3703241e67e276` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e8cf32fd9829794001e31` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e7747ef839deab5013c55` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2a62d48715c44a14b2da` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e55ab4851c32ea7c840eb` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.
- `e09e98de05b86472833b6` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e37ce5f23df39410a235e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee04b418bee6135e8d624` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e8d9efddd9335e237ece4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e56b5b4f35a4489239046` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e540f8b59a0b03a1b4448` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.
- `e8c8f0d70afa56228b3a3` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea0272a7551bac69d0a97` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed27ad988f74aaaded0c7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ec7d73937c1fb9ece2ef9` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb12444f96efc1dcd160b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e960c4f9354c6e35b3ae8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.
- `ec43f65a980f16d062796` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8c45f1690171312814fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea147dbcac9d0f33e46db` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e7a06ea6585321890a732` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea1a8fb8f4e3a1a712b2b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `edaf242a10c2278197cfe` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.
- `ed8535c87807ff404fe3e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb65a60180ea7caadcbc9` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0ce4fc941a14062c9c54` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_CANCEL/stderr). Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e9dd8cbefc412c3e219f1` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e207fc2c5c3d4b5901696` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9e21e2de9c7189b0a8f4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/BATCH_NEW/stderr). Unmodified original stderr for this isolated process.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 467–549. Retained reference `e7ca34f2d6fd9344d4610`.

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1252–1295. Retained reference `e98d4bc726037374525cd`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 505–583. Retained reference `eb83569215d92fa37da10`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1404–1449. Retained reference `ebcd14ee056c41f642dc1`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 520–598. Retained reference `e5cad7b4101332ff96f7c`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1483–1531. Retained reference `e1799f8f23cd143629768`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 517–595. Retained reference `e4d5b46a62af04d177edb`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1953–2000. Retained reference `e30bb9be41c1824d28e5d`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `TimerHandlerEventProc`, lines 744–841. Retained reference `eaf331dbfda81fff6c4db`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_UpdateObjCmd`, lines 1992–2039. Retained reference `eaa5f919dc246958bc5d1`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_ProcessEvents`, lines 377–534. Retained reference `e4521e47b782c5165c471`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELUpdateCommand`, lines 647–670. Retained reference `e4521e47b782c5165c471`.

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
