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
- `eb9325cec7267e7cb1aa5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/receipt.json). SHA-256 `9f23b20f02985e1a0543a47e1061563f59aa3bd4d16fd2cedd40a865b182f9e4`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee0568126e2ef3d7568a9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stdout.tsv). SHA-256 `9eca97f6890fb8d165cd789597832804c034150ffa1d32f94e4210b81baa4542`. Unmodified original stdout, including partial rows when externally timed out.
- `ee07621ae3c069bc5e215` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e3282e9e4740a3e1c47b8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/receipt.json). SHA-256 `a8ff6117dede02a134cc1a222fc73460da120718b9a066dee8120b57c6d4c886`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec31ed7f9ece72f4d2169` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stdout.tsv). SHA-256 `4e8b3f7d3997aa55c38a426e0e0ab475edfd02053082ff8493bea0b6df672cb5`. Unmodified original stdout, including partial rows when externally timed out.
- `e245242f2daca6f339fd4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e364ec755633abdf8c91d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/receipt.json). SHA-256 `3dd0827dc95b2cbfcd4b4f32fb18ea8ec0607ae6df69de6397db7675846fe395`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9d99095ebaed79fceb0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stdout.tsv). SHA-256 `f8ab03c8c2e9e5d53fa2e72ade291da9c2f782d9af66a1c04619a4e3ebdaf911`. Unmodified original stdout, including partial rows when externally timed out.
- `e1c735108d4b2a99ca64e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea6c93db0f327ef7a0ffd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/receipt.json). SHA-256 `adc06f2e8eb652e94f02708dee06dd836d61b24244542ac53a9c9da92dcd700a`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e96f320295a482e1872d7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stdout.tsv). SHA-256 `22ac78c74205394d5a818f84bd59e76a72b6b1a5d6bd80976270e86b3dee192f`. Unmodified original stdout, including partial rows when externally timed out.
- `e61f8c925be915cc44f69` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e2e5d444adaa1d7067f76` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/receipt.json). SHA-256 `6254b80cea0ea2df39fca0d10bea88112d41155fa45e27e4ec9e9ee64ba0d0bf`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e08632751803efc1b280f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stdout.tsv). SHA-256 `225ce9eec8d3a3b6f6487f052624c78eeeb4f0cf3d3e2cff47bca7e9b4b13fe6`. Unmodified original stdout, including partial rows when externally timed out.
- `e9b4a662324d6f1253bc9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1e6638f470add0c7cb4b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/receipt.json). SHA-256 `b6a189efb71e9409bcf820abbf401605a6f2e16db8c4734719e1c0912e9f09c3`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef024b89be2ed8f47dc2c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `02432d99a19b97c467c77046e084053e9a9d891222ede25130fb9aafa30b0048`. Unmodified original stdout, including partial rows when externally timed out.
- `e93d59e869ca6f874f045` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ee8a828b769c03da287cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/receipt.json). SHA-256 `1ecb86461a5af4eec789970f8fbacd779a6563a02cb1686b8f44e9fb4b783ce5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4f14052e1c21207121b6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `38963ad2b5cdedc3d5b71d3b18c3ee61c5c2da0bb29fc897e91b82ec70663430`. Unmodified original stdout, including partial rows when externally timed out.
- `e7716ec43a132ae5fef95` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5ba522396da7953dfc0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/receipt.json). SHA-256 `a20e5bcfebc277da31cd2fe2234368b7e8032abb0349b9333df17ce3f35d48a8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e64618ca5c9b2b9113535` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `9d74f252a163fbd30a443a2bef03cc695c0bdfb7a9e7b1430fdd70f35aa99480`. Unmodified original stdout, including partial rows when externally timed out.
- `e7fa613ea1a5496a4a10c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec29bce19148d582b231a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/receipt.json). SHA-256 `25d4819147c7e4b9e269ba06e0237fd23e1deead45fa79107834543e2f7bf87b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e734b6b53636e780159f3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stdout.tsv). SHA-256 `60d550cb98cc08a1b525609e3839541aec518834a3d5100249dd27706bc54997`. Unmodified original stdout, including partial rows when externally timed out.
- `e1509ed92b962dc9df3c8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e23a39088f7832156cde1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/receipt.json). SHA-256 `981e071e7221d7f0dbe7c39c8ce864f87a662bb854c1e4f95ebde694c44411ed`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e93c8462a3ab075a7ceac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stdout.tsv). SHA-256 `e97e970fa594a804061c9c6c732855a35b5fede60da8ae59cea350f112a9b191`. Unmodified original stdout, including partial rows when externally timed out.
- `ea886c9cb64feb544ac59` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ed338267db2dd1ba293cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/receipt.json). SHA-256 `31183891822b5f79260d66e7e552a1abdab75fb5c790a4a02a57fd76951be105`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb14e5e5fdc36118b2462` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stdout.tsv). SHA-256 `0eac1b0108e4f7bc98ce166ed1a251d76c48cf427271ee90f41cbb82b6538afe`. Unmodified original stdout, including partial rows when externally timed out.
- `ec0789124957b4eff9b5e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e98a2e2be12fa548602f6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/receipt.json). SHA-256 `8e0766e00ae880c14006bdde569212108f46df3a36ea565ea2f45def34ed4a7e`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6cf1cd5937d3c1980c5e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stdout.tsv). SHA-256 `a25b8188c237a7190d4313b2f448379f7e91ca2748b2d1a1adc2639bfb73d5f7`. Unmodified original stdout, including partial rows when externally timed out.
- `e9d09956aabd7b326a3ab` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e9777c033d45b70351133` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/receipt.json). SHA-256 `df69f2edc021643fa296b04ce36044f7f625a66adeed7c68af0656af213f8f05`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e244eac912600eb24eddf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stdout.tsv). SHA-256 `660c3f2144a4ba432a2db9787e973e673d4e314634c24fa7cf9addec5c2eaa46`. Unmodified original stdout, including partial rows when externally timed out.
- `e0b2801005092b86c9de2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e60e221fe7af73019e2ff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/receipt.json). SHA-256 `bdf7e1373cbfcf68ed760afaf1cb99cf6dc00f6876769d14d9c0b2b49be0df94`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e302ccdb7f6a6525c5d63` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `6d1bf667669923002119b06c975c05a465e935cd59e49d4ca420f04574ddb054`. Unmodified original stdout, including partial rows when externally timed out.
- `e2dad65a3928ea13ca7ae` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e41aeb771c77f34c60f24` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/receipt.json). SHA-256 `0aecdded61083e5a63a8093e3bb76b4b79aed920671d11f1ef146a66df3b25ff`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eafd03548ae81efb09178` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `b6fd9d2299880711480cf034fdd1f1ffe9ac97d04f4d561ed78fc86142db034c`. Unmodified original stdout, including partial rows when externally timed out.
- `ebfe9879e86ae80e28be8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e3aa4330e2f29a131ba32` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/receipt.json). SHA-256 `04fb707a4ff34d5968f1a292c3ac2d3acdb34aa26c75051de1a800150265bb71`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9971549d5933891c33d0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `6fd8c6f503098c7ba5642b65e13be434b44afd9e8ff649db5ecde0a5055f099b`. Unmodified original stdout, including partial rows when externally timed out.
- `eedd70aa16cfca05ddefd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8ad39838e522ed58d99e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/receipt.json). SHA-256 `cf102534b5773c8326422a4f99a09f8017e4515e0256725c5f4cea430713ff66`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeafa0ba2ac9735c07b3a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stdout.tsv). SHA-256 `eb08ed3534c249b0725a04ceb6b0fea2beb6f83b73b890062d06c0aa1f59d9da`. Unmodified original stdout, including partial rows when externally timed out.
- `ee25fc930099fea94ba8a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef576c40843dd872074e8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/receipt.json). SHA-256 `600074528a87617d91a6c8f3c946b6f4b21bffafa45a40390f3c5abbb5d57b6c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eabdbe1ea2e3168c96fc7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stdout.tsv). SHA-256 `891760ab88281debb00d245032aae03fa84c8db7c01f851d989139948917b506`. Unmodified original stdout, including partial rows when externally timed out.
- `e3ce27f78681cc5f7c04f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec387ddceba56930b08e8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/receipt.json). SHA-256 `503735ecd94f6929531ab1d01a062c7baf51b41111d9ad42261204e9aadce4d7`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea5d06eb9ac84b4a38fce` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stdout.tsv). SHA-256 `303b9007e3beaef9a73edaa691fe0c748c7ba877416535deb02bd8f93be4c885`. Unmodified original stdout, including partial rows when externally timed out.
- `ee02e4ae770aa7ae7dbc7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ed46c402ea84fb215a0d1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/receipt.json). SHA-256 `c1f3ab6aa5fa9a46f50d006e103abfd90afd72a388e904bdb25b536ce13856a2`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef9743fe503d4c869a958` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stdout.tsv). SHA-256 `5a502f520be69be2804170573ddfcefa144a6db4a852d3bb4ba4954338cf7ea7`. Unmodified original stdout, including partial rows when externally timed out.
- `e66c7cffd06f675cd5e75` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb3d4747e4fdf0039a5a1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/receipt.json). SHA-256 `566418001cf43eeab11b221e6bac3bdaa93374e306e4e75f8d2428c28de2ad04`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5c6250736d1d64127f8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stdout.tsv). SHA-256 `04161c90202e8f15e849b730c08bb6b0d136ca747c31eb53f4a2f4a0d2831d23`. Unmodified original stdout, including partial rows when externally timed out.
- `ea25c71e030c485bc6360` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e6749aa2a4f931d2c33c5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/receipt.json). SHA-256 `272e8aa1a67cb5d2c756b3c44328c91a3191b839594f37f64bd33368155e0bff`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e040137b0145ad258ddaf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `9baa71075f31dd80f87b6046932d0a26366b555312d0a6c858e4910530246695`. Unmodified original stdout, including partial rows when externally timed out.
- `eccc19d78360280e89282` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1f8f5936b5349af7892a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/receipt.json). SHA-256 `b4efd01efd9e79ced1de9786452017bf5c4372c1a13c4d983116d54e2f79777a`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edaab5b61c91df0a53cfc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `505952b5c46dc0bdca387bd349e057e5ffb66d43813737f73f258e220232b5eb`. Unmodified original stdout, including partial rows when externally timed out.
- `eec66389a3eba5e981b63` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e0e7c3a6bd23468f7c9a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/receipt.json). SHA-256 `868787c8f2ea9260e77f1bc7be990035caeee72b80062d7676e0426b8254fa4a`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3675d6cd33bd37940740` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `9b9b5f4f0a1d93da071677670861335a81162bf9772ec5c5774981fdd2089c1a`. Unmodified original stdout, including partial rows when externally timed out.
- `e6cbe8573cf877d20de3c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5b4603e03625040953d7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/receipt.json). SHA-256 `32448cb1c1d906a31eb596521e9c38f8c81c7861406c478232620582830580c1`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4fd257ab4d6b57d4f8a1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stdout.tsv). SHA-256 `c646ba6037fc5f797b1eff2770bdc32e8798b0b8e2a5228335dda1378dd3b88f`. Unmodified original stdout, including partial rows when externally timed out.
- `ec6483935f5a4e303d693` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e6a4a6da8bd13ccc98465` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/receipt.json). SHA-256 `1550b7e89b55577da31982dbae97cce8a64a4a07219a70bc7ad7b28bf1b749fd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e579bda8551b4557e5b10` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stdout.tsv). SHA-256 `ded0cc0525e9553a93eb23af04810b3f89c5ef27625188ac288f71e1532a23ad`. Unmodified original stdout, including partial rows when externally timed out.
- `e2cec3171d57c0129e6e0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `edd74eeadf459c0533b98` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/receipt.json). SHA-256 `468b1a3373f0a3ca1ea2fcc4325ec08b4540dca6fb033e76ffd80d8f8c365258`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e61bf269e929bab478189` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stdout.tsv). SHA-256 `6c026dbd3cdb55e98b5e903b3aae393a69ffbd218f51607217d1afe740c214ae`. Unmodified original stdout, including partial rows when externally timed out.
- `e2817ef614ca472ed5d85` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1c3efe03c2150d71a368` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/receipt.json). SHA-256 `029ff0428c4cc433da2a1f76a79470ce3194745089649b6ebcfbf91154ffbde6`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eefe02444a39ba0b0e3e5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stdout.tsv). SHA-256 `31d49e37ad031b6af9e71fc5c683f044be7295cc62a730cdac93ca877d302d8f`. Unmodified original stdout, including partial rows when externally timed out.
- `e789f93f8a18a4e973ebe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e069926499e06b68f3913` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/receipt.json). SHA-256 `d3e3a9029a8d9d1a24e8d89fc2004bbb9e3d3e32620064c2c7adb616652413e0`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee123cb3871b9f0292149` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stdout.tsv). SHA-256 `e50a834628b2ad7f8559e542c6d61edf0bd21d644390993a7404f7bb2ce10ad6`. Unmodified original stdout, including partial rows when externally timed out.
- `ed05149daad2a65f71087` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8ab87bd04ecedff6aaab` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/receipt.json). SHA-256 `27a69099c86317dc76e19806bb5be7d9c32630703b80250aebb48baf8c9de9da`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeae1a41f795c0e440a6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `8c3373c36be63d9b9234642469028fc81fc63e191c615a111b50eb3c08554952`. Unmodified original stdout, including partial rows when externally timed out.
- `ee774bb7505710c2d19eb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efcd4f95fbbf55c6b64c0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/receipt.json). SHA-256 `3ea93d140160ef03df844e8293f4e4aad4267a15b5dd18d356f2dbeafad658aa`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e88a3f93f16b54f3c0c40` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `affa43ba47f5b7a90d1fc2739bc4b86a0a9cae19617b77d9d647a694d5e4552c`. Unmodified original stdout, including partial rows when externally timed out.
- `ecf8adb8d924fe17ffe0d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea740ed33f275a86d6af6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/receipt.json). SHA-256 `3458f38b15b296e8d19ed1e5973ffe1571b0926c865ea374b643044dab5a8d07`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea7fd8d56f215132ae1b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `05fd52c039da64c904f99c54d79dc0f9035f5b31b191f6c3e4cc4cb1c41afb83`. Unmodified original stdout, including partial rows when externally timed out.
- `e43a790738a9547728892` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e65396ddcdc30c4adea9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/receipt.json). SHA-256 `064c1a8baa41bc18d0900d951d3b31833fddf69fb920a0bffb2838dc464e3169`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2efc604c633b54647082` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stdout.tsv). SHA-256 `673cf6267b6380fe1915f1c228779268d87ec05e896436388f1a7a143864be8b`. Unmodified original stdout, including partial rows when externally timed out.
- `ed0e224d5afb203a5e0ad` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e239003266a12dd486e49` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/receipt.json). SHA-256 `6eb5ec08315e36c5059e40c5e44c579984c3d8cd055bbb9f44fa4371e5dc96bf`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e900d1c1c980aab4b1fe6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stdout.tsv). SHA-256 `df2659bde9f8cf90014aa7aeb6a36b4b70550c5ca1456702f61bfc8ec1970016`. Unmodified original stdout, including partial rows when externally timed out.
- `ee1702a28d1b886676a46` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e31ac280e6db7eb77e31e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/receipt.json). SHA-256 `167f7f45245b8ad1340aad02bb0bd482bf3819442832dc46496c0a8a0d4d4ac1`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb83ab51c30a2a102443e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stdout.tsv). SHA-256 `c11351dda1aa20c0de8e2debace890e3c7ac8fe7d2500732046be0b108b453e0`. Unmodified original stdout, including partial rows when externally timed out.
- `eec28553b446ad017579e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec7c1471a6d65b2d4f127` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/receipt.json). SHA-256 `7141d2a1462b73ee49c589bbabaa038aaa8583ac02a7da9eede06c271c988b8c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee472415f95f53a5d33b4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stdout.tsv). SHA-256 `1b3fc0f211336786de8020bac2282da250b842426afc541e61ced371ba222c0a`. Unmodified original stdout, including partial rows when externally timed out.
- `e2bacdd1ea647def235e9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e49549faf0f2b544baebb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/receipt.json). SHA-256 `c3069fff745225c26e021cbdfe4114ab406a9de508beba35bdffd41988341505`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3f23d87ae93dcf475c40` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stdout.tsv). SHA-256 `3b7cb9e7ed1f86f33b75c841f287ea4c05be45a89ed1cc4cc5293e86aa50eb2c`. Unmodified original stdout, including partial rows when externally timed out.
- `e990fe7d8d13522aff586` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8f189dff109cefa08b8b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/receipt.json). SHA-256 `fe514f7176ef41369b0ba8277a56d0bdb3df712f6d4aac82dd1b064851db8bb8`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e0b02d69d705550e98a3b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `4aa30538182b41d2f382212552fe7081e7573a9a991b169082b30bab51d082fe`. Unmodified original stdout, including partial rows when externally timed out.
- `eee095fd06b05bc75e014` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e89fbe34aa9ce04782ab8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/receipt.json). SHA-256 `8015664a6ded65360aed3801cdfdcf0afce98a3f476650fa1c1472d88676b55d`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef69a32dfe0a4cf7b7226` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `a6efa2ce6fce8e0cb2ff830c72da08ef6a60b128d19c771364219ce72e5ac342`. Unmodified original stdout, including partial rows when externally timed out.
- `eef85686fdecdc202789a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e91d75cf46a5d812347d5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/receipt.json). SHA-256 `8d97663f9e32313c9d370d50bf7453c4c52754e0c788f070597c0638426f5935`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edf68ea8b38fbfcbcff3b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `b71c9723b6d3ee23ea312a44996b5fd25b1a1b4fb828fe0e98d32800d3978e06`. Unmodified original stdout, including partial rows when externally timed out.
- `e23c5ca15a7b0da2e5bf4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e5531f64a1448337efc31` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/receipt.json). SHA-256 `7523e39568bcad3d3d57a9705fb8dcd396d4f219980d7c149bc55245da7c299f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3ace066a58fb0b6264f9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stdout.tsv). SHA-256 `6d7f976d1b45f1fa9f53412460a85f4b6346a1404eef5793a876ce13ad6baea5`. Unmodified original stdout, including partial rows when externally timed out.
- `ee5a147998c350de86b7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_COUNTED/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e630a33934a2c5744b36d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/receipt.json). SHA-256 `14c598f443bdc56c736495d2f949c9c180e4f03b4a6441cf2d95bf716036a0bb`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e227ec1f4dda0ae0dfca4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stdout.tsv). SHA-256 `632652d6c94608272b3ab0e01a2da5b204a17355ecf9aba6b7b80cbb3b67ce58`. Unmodified original stdout, including partial rows when externally timed out.
- `edfdd8b8c81b4683732cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CANCEL_CONCAT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1438b2a8ae26cd495d87` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/receipt.json). SHA-256 `6a00be91565d78ce17df8f550b66bf5d1653b6602481892cc91d2aeb9eac4bc0`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e345915890cb47fea0b9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stdout.tsv). SHA-256 `fc0bb4e31fcf381c0dbda8af2554e867a35bf06609e166efd6d3028ec75c466f`. Unmodified original stdout, including partial rows when externally timed out.
- `ebf6a23b2694d9f58268e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_SCRIPT/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e54b09aec9647dafffb09` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/receipt.json). SHA-256 `f83f4818b9dcdc661d873e90ba3c18ff3dd679a14e529dbf416ebb68a3bf1949`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebf47181fa3c510b766b1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stdout.tsv). SHA-256 `0738c51cf014246743bd148df59bd30b810358b17e57ae44684fba2ea5dc96fa`. Unmodified original stdout, including partial rows when externally timed out.
- `e6fc481b481dd9f982879` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_KIND/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eb593c4ce3a75c5d02908` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/receipt.json). SHA-256 `f780255b56fd6f4b9cb28d1913023ba8405c6d347a987d6cd6fec5a77cddc76d`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e94fadfa0dbc1307b2504` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stdout.tsv). SHA-256 `dc64a1ed8c0f5f563f59762f1f896f81b6884b7ef24d2e8eff9cd3c83b856389`. Unmodified original stdout, including partial rows when externally timed out.
- `e0959243c24a47df7ac7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/INFO_MISSING/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `edc3810c0705e0ed6cc0f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/receipt.json). SHA-256 `d8fe901cc38d180557d5abe003edef64adba8b1fea90f3653e0ef3efaac0d653`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e410c8b6d027a928739c9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stdout.tsv). SHA-256 `e45487dd89c2165eba0c745f242dda423989a310f614b20685f673a0e82772e4`. Unmodified original stdout, including partial rows when externally timed out.
- `e1039434050aeb6a35224` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e4c2ec657d15c3d8e6868` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/receipt.json). SHA-256 `ba32de4f7a04718fec02ac781e28134c75e32a0da756878460ff79d54a10381f`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec3961a0be867998d1460` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stdout.tsv). SHA-256 `73a3f276960528380ffaa426837c4c05db0e55c1320093c0250056ab48d12ead`. Unmodified original stdout, including partial rows when externally timed out.
- `eb5544d39b66e0da5a5cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/AFTER_AMBIGUOUS/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e50f3fdab3ee7747ca388` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/receipt.json). SHA-256 `56bccebbd962d7610f0964f6b05e078209de5b8abf12e6abe034c5201ecae1e0`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1b25a142c5fa425ce36a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stdout.tsv). SHA-256 `4783c490dab932551d7d2ea26ee78f3a7c6fc651928c457f890d9606e2bb6fe8`. Unmodified original stdout, including partial rows when externally timed out.
- `e222fe54c231c8faa8488` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UPDATE_ABBREVIATION/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `GetAfterEvent`, lines 955–982. Full-source SHA-256 `3a54a00f6b2037ce2afc039054dad9c2a7b5c80c77c0258c696a3c9f4ca15cfa`; snippet SHA-256 `e1bee03ceddfaa8fe17a4d72881f1d9159d1c9e5bace9a534d9dea826ece4710`; retained evidence `e7ca34f2d6fd9344d4610`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `GetAfterEvent`, lines 1065–1092. Full-source SHA-256 `9c6e3aa2bad086fe1e94dadca7e99c22b5997b55b07065e6d9256a7a692255ff`; snippet SHA-256 `9c4faf972703f3c39efd728c849b1cadd3b86a7fad521151776042d90591f027`; retained evidence `eb83569215d92fa37da10`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `GetAfterEvent`, lines 1119–1146. Full-source SHA-256 `648da0aebdcd2adde777ab81d12e4722c7ca8396ff5034e71c4e3f19e25ca7d5`; snippet SHA-256 `4630c11baad387a6273ae2994613a69d0996b5f1ee5ba3816b0745bd334ff6c9`; retained evidence `e5cad7b4101332ff96f7c`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `GetAfterEvent`, lines 1098–1125. Full-source SHA-256 `9be43ec6af72e0ba19422b277c6badb07fec3e604fbaeef8ee6147d3bc21284e`; snippet SHA-256 `77a0dffb17991ef92c1eef35f07a0b40b244b04d02b6d035532221c32fc3e65a`; retained evidence `e4d5b46a62af04d177edb`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `GetAfterEvent`, lines 1484–1511. Full-source SHA-256 `580f163fb0bbd01a5a5ffbd9992cf7f4d2dec932355edb7fe6383b0352ab581b`; snippet SHA-256 `77a0dffb17991ef92c1eef35f07a0b40b244b04d02b6d035532221c32fc3e65a`; retained evidence `eaf331dbfda81fff6c4db`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimParseAfterId`, lines 276–286. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `c6172bc9c6746fa68b96b625fae160cba63a6760b507e87aa1ee71daf518d6f6`; retained evidence `e4521e47b782c5165c471`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_DeleteTimeHandler`, lines 339–359. Full-source SHA-256 `b9536f7021919026f8ee2cfcdb35a59305db07a23e60daa50d5da966d52a1a12`; snippet SHA-256 `87635142d26f7203943226df8058e914d5328205eea7410cad1a639fc4736763`; retained evidence `e4521e47b782c5165c471`.

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
