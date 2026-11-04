# naming.event.original-option-query-boundaries

Kind: `native-observation`

## Problem statement

Reparsing rendered after/update selectors loses the provider-specific abbreviation, identifier, query and result boundaries.

## Question

What are the exact captured after cancellation/info, abbreviation and update selector outcomes on the six selected providers?

## Conclusion

The retained cancel controls leave done at 0; script-info returns set done 1. Zero-timer kind is timer on C and idle on Jim. C accepts after in, rejects ambiguous after i with its composed diagnostic, and accepts update i; Jim requires exact after selector matches and accepts abbreviated update. Missing-event code/result/options remain exact captured rows, with C84/Jim options API unavailable/unqueried explicit.

## Scope

Eight exact source controls per provider in v106. Current selected table/identifier/query boundaries, not generic numeric conversion, index-cache authority, all raw binary selector cases or event-lifetime proof. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.4.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or a number' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.5.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or an integer' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl8.6.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or an integer' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.0.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or an integer' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: tcl9.1.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'timer' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 0, bytes b'' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, info, or an integer' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Fresh-process counted ASCII source passed to Tcl_EvalEx/Jim_EvalObj; public result/return-options getter after evaluation; external deadline outside guest.. Dialect: jim.

v106/CANCEL_COUNTED: CANCEL_COUNTED: code 0, bytes b'0' v106/CANCEL_CONCAT: CANCEL_CONCAT: code 0, bytes b'0' v106/INFO_SCRIPT: INFO_SCRIPT: code 0, bytes b'set done 1' v106/INFO_KIND: INFO_KIND: code 0, bytes b'idle' v106/INFO_MISSING: INFO_MISSING: code 1, bytes b'event "after#99999999" doesn\'t exist' v106/AFTER_ABBREVIATION: AFTER_ABBREVIATION: code 1, bytes b'bad argument "in": must be cancel, idle, or info' v106/AFTER_AMBIGUOUS: AFTER_AMBIGUOUS: code 1, bytes b'bad argument "i": must be cancel, idle, or info' v106/UPDATE_ABBREVIATION: UPDATE_ABBREVIATION: code 0, bytes b''

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
- `eb9325cec7267e7cb1aa5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee0568126e2ef3d7568a9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee07621ae3c069bc5e215` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e3282e9e4740a3e1c47b8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec31ed7f9ece72f4d2169` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e245242f2daca6f339fd4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `e364ec755633abdf8c91d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9d99095ebaed79fceb0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1c735108d4b2a99ca64e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `ea6c93db0f327ef7a0ffd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e96f320295a482e1872d7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e61f8c925be915cc44f69` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `e2e5d444adaa1d7067f76` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e08632751803efc1b280f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9b4a662324d6f1253bc9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `e1e6638f470add0c7cb4b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef024b89be2ed8f47dc2c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e93d59e869ca6f874f045` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `ee8a828b769c03da287cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4f14052e1c21207121b6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e7716ec43a132ae5fef95` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `e5ba522396da7953dfc0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e64618ca5c9b2b9113535` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e7fa613ea1a5496a4a10c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `ec29bce19148d582b231a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e734b6b53636e780159f3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1509ed92b962dc9df3c8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e23a39088f7832156cde1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e93c8462a3ab075a7ceac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea886c9cb64feb544ac59` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `ed338267db2dd1ba293cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb14e5e5fdc36118b2462` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec0789124957b4eff9b5e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e98a2e2be12fa548602f6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6cf1cd5937d3c1980c5e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9d09956aabd7b326a3ab` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `e9777c033d45b70351133` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e244eac912600eb24eddf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0b2801005092b86c9de2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `e60e221fe7af73019e2ff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e302ccdb7f6a6525c5d63` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e2dad65a3928ea13ca7ae` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e41aeb771c77f34c60f24` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eafd03548ae81efb09178` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ebfe9879e86ae80e28be8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `e3aa4330e2f29a131ba32` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9971549d5933891c33d0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eedd70aa16cfca05ddefd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e8ad39838e522ed58d99e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeafa0ba2ac9735c07b3a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee25fc930099fea94ba8a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef576c40843dd872074e8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eabdbe1ea2e3168c96fc7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3ce27f78681cc5f7c04f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `ec387ddceba56930b08e8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea5d06eb9ac84b4a38fce` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee02e4ae770aa7ae7dbc7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `ed46c402ea84fb215a0d1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef9743fe503d4c869a958` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e66c7cffd06f675cd5e75` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `eb3d4747e4fdf0039a5a1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5c6250736d1d64127f8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea25c71e030c485bc6360` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `e6749aa2a4f931d2c33c5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e040137b0145ad258ddaf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eccc19d78360280e89282` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e1f8f5936b5349af7892a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edaab5b61c91df0a53cfc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eec66389a3eba5e981b63` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `e0e7c3a6bd23468f7c9a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3675d6cd33bd37940740` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e6cbe8573cf877d20de3c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e5b4603e03625040953d7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4fd257ab4d6b57d4f8a1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec6483935f5a4e303d693` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e6a4a6da8bd13ccc98465` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e579bda8551b4557e5b10` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e2cec3171d57c0129e6e0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `edd74eeadf459c0533b98` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e61bf269e929bab478189` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e2817ef614ca472ed5d85` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e1c3efe03c2150d71a368` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eefe02444a39ba0b0e3e5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e789f93f8a18a4e973ebe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `e069926499e06b68f3913` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee123cb3871b9f0292149` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed05149daad2a65f71087` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `e8ab87bd04ecedff6aaab` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeae1a41f795c0e440a6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee774bb7505710c2d19eb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `efcd4f95fbbf55c6b64c0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e88a3f93f16b54f3c0c40` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ecf8adb8d924fe17ffe0d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `ea740ed33f275a86d6af6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea7fd8d56f215132ae1b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e43a790738a9547728892` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e65396ddcdc30c4adea9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2efc604c633b54647082` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed0e224d5afb203a5e0ad` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e239003266a12dd486e49` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e900d1c1c980aab4b1fe6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee1702a28d1b886676a46` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `e31ac280e6db7eb77e31e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb83ab51c30a2a102443e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eec28553b446ad017579e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `ec7c1471a6d65b2d4f127` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee472415f95f53a5d33b4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e2bacdd1ea647def235e9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `e49549faf0f2b544baebb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3f23d87ae93dcf475c40` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e990fe7d8d13522aff586` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `e8f189dff109cefa08b8b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e0b02d69d705550e98a3b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eee095fd06b05bc75e014` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e89fbe34aa9ce04782ab8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef69a32dfe0a4cf7b7226` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eef85686fdecdc202789a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `e91d75cf46a5d812347d5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edf68ea8b38fbfcbcff3b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e23c5ca15a7b0da2e5bf4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e5531f64a1448337efc31` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3ace066a58fb0b6264f9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee5a147998c350de86b7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stderr). Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e630a33934a2c5744b36d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e227ec1f4dda0ae0dfca4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `edfdd8b8c81b4683732cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stderr). Unmodified original stderr for this isolated process.
- `e1438b2a8ae26cd495d87` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e345915890cb47fea0b9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ebf6a23b2694d9f58268e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e54b09aec9647dafffb09` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebf47181fa3c510b766b1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e6fc481b481dd9f982879` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stderr). Unmodified original stderr for this isolated process.
- `eb593c4ce3a75c5d02908` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e94fadfa0dbc1307b2504` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0959243c24a47df7ac7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stderr). Unmodified original stderr for this isolated process.
- `edc3810c0705e0ed6cc0f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e410c8b6d027a928739c9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1039434050aeb6a35224` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.
- `e4c2ec657d15c3d8e6868` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec3961a0be867998d1460` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eb5544d39b66e0da5a5cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stderr). Unmodified original stderr for this isolated process.
- `e50f3fdab3ee7747ca388` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1b25a142c5fa425ce36a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e222fe54c231c8faa8488` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stderr). Unmodified original stderr for this isolated process.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 735–933. Retained reference `e7ca34f2d6fd9344d4610`.

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `GetAfterEvent`, lines 955–982. Retained reference `e7ca34f2d6fd9344d4610`.

```text
GetAfterEvent(assocPtr, commandPtr)
    AfterAssocData *assocPtr;	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr;
{
    char *cmdString;		/* Textual identifier for after event, such
				 * as "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = Tcl_GetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 766–969. Retained reference `eb83569215d92fa37da10`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `GetAfterEvent`, lines 1065–1092. Retained reference `eb83569215d92fa37da10`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    char *cmdString;		/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 781–992. Retained reference `e5cad7b4101332ff96f7c`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `GetAfterEvent`, lines 1119–1146. Retained reference `e5cad7b4101332ff96f7c`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 776–981. Retained reference `e4d5b46a62af04d177edb`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `GetAfterEvent`, lines 1098–1125. Retained reference `e4d5b46a62af04d177edb`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = (int)strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `Tcl_AfterObjCmd`, lines 1018–1171. Retained reference `eaf331dbfda81fff6c4db`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `GetAfterEvent`, lines 1484–1511. Retained reference `eaf331dbfda81fff6c4db`.

```text
GetAfterEvent(
    AfterAssocData *assocPtr,	/* Points to "after"-related information for
				 * this interpreter. */
    Tcl_Obj *commandPtr)
{
    const char *cmdString;	/* Textual identifier for after event, such as
				 * "after#6". */
    AfterInfo *afterPtr;
    int id;
    char *end;

    cmdString = TclGetString(commandPtr);
    if (strncmp(cmdString, "after#", 6) != 0) {
	return NULL;
    }
    cmdString += 6;
    id = (int)strtoul(cmdString, &end, 10);
    if ((end == cmdString) || (*end != 0)) {
	return NULL;
    }
    for (afterPtr = assocPtr->firstAfterPtr; afterPtr != NULL;
	    afterPtr = afterPtr->nextPtr) {
	if (afterPtr->id == id) {
	    return afterPtr;
	}
    }
    return NULL;
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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELAfterCommand`, lines 686–793. Retained reference `e4521e47b782c5165c471`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimParseAfterId`, lines 276–286. Retained reference `e4521e47b782c5165c471`.

```text
static jim_wide JimParseAfterId(Jim_Obj *idObj)
{
    const char *tok = Jim_String(idObj);
    jim_wide id;

    if (strncmp(tok, "after#", 6) == 0 && Jim_StringToWide(tok + 6, &id, 10) == JIM_OK) {
        /* Got an event by id */
        return id;
    }
    return -1;
}
```

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_DeleteTimeHandler`, lines 339–359. Retained reference `e4521e47b782c5165c471`.

```text
jim_wide Jim_DeleteTimeHandler(Jim_Interp *interp, jim_wide id)
{
    Jim_TimeEvent *te;
    Jim_EventLoop *eventLoop = Jim_GetAssocData(interp, "eventloop");

    if (id > eventLoop->timeEventNextId) {
        return -2;              /* wrong event ID */
    }

    te = Jim_RemoveTimeHandler(eventLoop, id);
    if (te) {
        jim_wide remain;

        remain = te->when - Jim_GetTimeUsec(CLOCK_MONOTONIC_RAW);
        remain = (remain < 0) ? 0 : remain;

        Jim_FreeTimeHandler(interp, te);
        return remain;
    }
    return -1;                  /* NO event with the specified ID found */
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
