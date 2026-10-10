# naming.event.original-list-script-byte-boundaries

Kind: `native-observation`

## Problem statement

String joining can change an original callback List and its non-Unicode/native-zero variable-name objects.

## Question

When original List script objects contain kFF, k00tail or kC080tail, do after/update and an original-object getter reach VALUE on each captured provider?

## Conclusion

Every captured provider returns OK for scheduling, update and the subsequent getter, with VALUE for all three key variants. C first registration reports after#0; Jim reports after#1. Update results are empty on C and VALUE on Jim. The probe observes public getters after each call and does not measure retained headers, reference counts or custom conversion callbacks.

## Scope

18 original C/Jim object-vector scenarios, each with counted native string key, original List script, after zero, update and original same-key set getter. Literal raw 00, FF and encoded C080 remain distinct. No Document ingress, original allocation identity or general existing-object/cache grant. C84 Tcl_GetReturnOptions is absent; Jim C return-options API is not queried; corresponding rows remain explicitly not-tested. Raw receipts and prior attempts remain immutable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: tcl8.4.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: tcl8.5.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: tcl8.6.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: tcl9.0.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: tcl9.1.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#0'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b''; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Configured pinned source/header/library/Makefile and executed probe SHA associations are retained in provider receipts; reported patchlevel was queried in each isolated captured process.. Channel: Original counted Tcl/Jim String/List script and getter argv passed by Tcl_EvalObjv/Jim_EvalObjVector, with the intervening update run through the captured public source entry; public result getters after each call.. Dialect: jim.

v106/ORIGINAL_LIST_RAW_FF: ORIGINAL_LIST_RAW_FF_SCHEDULE: code 0, bytes b'after#1'; ORIGINAL_LIST_RAW_FF_UPDATE: code 0, bytes b'VALUE'; ORIGINAL_LIST_RAW_FF_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_RAW_ZERO: ORIGINAL_LIST_RAW_ZERO_SCHEDULE: code 0, bytes b'after#1'; ORIGINAL_LIST_RAW_ZERO_UPDATE: code 0, bytes b'VALUE'; ORIGINAL_LIST_RAW_ZERO_VALUE: code 0, bytes b'VALUE' v106/ORIGINAL_LIST_ENCODED_ZERO: ORIGINAL_LIST_ENCODED_ZERO_SCHEDULE: code 0, bytes b'after#1'; ORIGINAL_LIST_ENCODED_ZERO_UPDATE: code 0, bytes b'VALUE'; ORIGINAL_LIST_ENCODED_ZERO_VALUE: code 0, bytes b'VALUE'

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
- `e79bf69ca48ec6caceb65` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `3e98bddede37dcce230cfd3c3bb0a4cdb681fb569ff6a0ac422caac8e762f0f5`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6e4bee24f4085d76ab48` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `25aaf0be091b16c95797a315608bfd85ece54dc0554cde0b4d15755d2a02b9a3`. Unmodified original stdout, including partial rows when externally timed out.
- `e64552ef1d080117fbb7e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). SHA-256 `8c439e4cd180405936d6a9b19888948e0dadc160038a9c6137f6a79f7f32dcfc`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). SHA-256 `d12f655970e0f356451ae31671821682c330c17a638c017c0532ae65423090b5`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). SHA-256 `8974ae4ce744ecced90120b0ad8853655b1bf92636249f4adbe4fdabb00d0c46`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). SHA-256 `74227d0d71958f68cf2ca608258972345dd7a13354a53fbae309aeb6b0716226`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). SHA-256 `6240f33086da1411a3448bd030a6b13395cf03e9bbe35edbc36b04def0c82bee`. Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeb64365e9c0d3551fd62` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `e460ceedc364fd77a4d9fd5c1db4d4577f04cb3bd3a30d63778ce555b7d224fb`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e45405b28097e80643a6a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `097d1ff94457b354f622d9d5acc764e2483bc5bbe958fb5ba4298e7e4a097179`. Unmodified original stdout, including partial rows when externally timed out.
- `eb3cc61c4d51835b9c143` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea420302d507ceec16c93` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `74af906fd0a5f4beca2cfce8dbfdb693ca6a2786954d9e2be41d7d4db25858a9`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8b4d2d762292bfb5484d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `a7af6793e5b6ce793b6707b61c39cad58c8902d5aa65a06481546c6a70fb3975`. Unmodified original stdout, including partial rows when externally timed out.
- `e0060aa2b68158517d329` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e11d7b853d4da016873b8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `573f30d01887c92924f07d5e6aadcac253c151cb6048f3fb3666d49817d51329`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8f8148a1291bf26f8ed8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `ed01c3cd52caaabe72582c0a747b3403cb0467e3f4e7e420ff1403b0e8c1fc06`. Unmodified original stdout, including partial rows when externally timed out.
- `e2f74eb28377fa7fa0af1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). SHA-256 `32125bf7d06e3a00238be44df707b992b5f31c567845b24ccdadfa4c02a6a398`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e12738a5b66fe94a092df` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `3d35deec9780228e07fde4b64a96155082cece78e471e007882a8a005c9a117a`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e29fdc622e1b8cb54607f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `162ac209c5313ada2f8fb9cd805fd6c75a8012df258e2640051451768c476866`. Unmodified original stdout, including partial rows when externally timed out.
- `e3f8923d1e613191318cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e1f63e1692baeae4759f8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `c68ba82413433d763960a0e9a9e38b12b0efc9c81c3dd1d42e3e507bf6990368`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e123f596d6d790adeb7c8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `45a20733f0969812ccff60bc2c239a6ca7383782cfaf69ad9fbcd67fa2810819`. Unmodified original stdout, including partial rows when externally timed out.
- `e786733d1a19d35ffdb07` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef46e2acef0d638d49e67` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `1a1d554e868e0a7632374ef25c4cf159073dbc6bc6cb13235399b7945825a4bd`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e33fa7bcde62217f1b391` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `12ae9d5a7c2e5b57ab6aef52c62978262e7dc6f160dc32e43c2c1eb393851b22`. Unmodified original stdout, including partial rows when externally timed out.
- `e7dd8a3a111d8f187aad9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). SHA-256 `7b6ca42197560dc859c9288d04ad5e316782272515eb36f5063904c679c370f5`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed1efc37ab1aed0603210` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `0f1c5671374e25dbcef4e17f138fb0a95ca75554a409c5c573333814f8df98db`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edab8083a4f53acf22edb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `f357d537d33d5e2651b334e4c074f92332a6b4061bd94b047797fcd25d566498`. Unmodified original stdout, including partial rows when externally timed out.
- `e9552da1ec430a002a536` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e8fae6cf981e1cbabb745` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `9ca85eda43058aa264223ba2a3f58ded9585aa802919df5718b0bd51c1f677bb`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea8dad2d420e17df3f36c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `b33850798197ca7133076a20cdc21e27732a5b7022d5ad6308a3f410077277e8`. Unmodified original stdout, including partial rows when externally timed out.
- `e1fa16abcb9dbf30486f6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e96c7fc8ba682a27c89f3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `246316d40c3fe07fbef32b734898161f6bdaa215f9d0c0984e76da6ed5b67a3b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef39e02f55116d78e3c6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `bd9f6f1d6fa648f35741542ec9f79da7b294aaf978c82a1ea7f8ea0bef1c3edf`. Unmodified original stdout, including partial rows when externally timed out.
- `eed940d9b2ebe338c2da9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). SHA-256 `f9c507ef96754fb7b3a57959c06573bb9f85eca0158e0a51388aa059f23d4ae3`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ec944911d67b83d44152e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `63bbdb1d5db144b475ac809078e00405452e38259ef114302225ae93f97a995c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e97a0e09009d4b15c847c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `f5517e3916159685271724d5d5ac3e7926474ae109df20091a62a3c70ea166f7`. Unmodified original stdout, including partial rows when externally timed out.
- `e9d500c1741a5095da042` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea4bc09b9dcb82042459c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `5ccb9bae3ffe474c0d55b1a82e20bc6a2de532dff968c5cd9c3d527a408fd631`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed8559e8d54ed74205fa8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `cb41ec443ae1a3956666a70bc741739accfebbb38d5d38a06406e097916f947b`. Unmodified original stdout, including partial rows when externally timed out.
- `e216dec22d87422af40e7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e4d1af6e6d028f005fec4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `a89be6f556847912d0fd92eade651dd276cc89af3d6fa9849429305ba6985613`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e74306b2b66d25373dcd2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `4fb625758fd4a4c5cddbf9ae0cd421ea001b39c54d61c1803bfe31f5524268b2`. Unmodified original stdout, including partial rows when externally timed out.
- `e53c4c7781f6c08d7c5ac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). SHA-256 `8ab6d94b8a62881d8550847e65a37c78565fd8e3fb34da7ac999f61f19b7f613`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e06cadefe216ef1c4f7ab` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `f1c58351e715397a6ded0aff31326a65494ab9b9f532bfdb7925fdc8cb397e72`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed863eaf17e3b7371eabc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `47b4742daaaeda7bf8945e1bf1efffb3da157f0d366bf1d0276a364becda4446`. Unmodified original stdout, including partial rows when externally timed out.
- `e9548f448b042c6a5363a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ef1082ebe3ce4ab0e8e8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `1160796c829e0e8cc17474715dd00a9059a8b12e0addf52586b33d1b9db6e794`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e638514e86c25042df7d6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `dd52b24303f9fff60a2d6decb15308a4a84e5dc672a425269cc10cd9b152fb53`. Unmodified original stdout, including partial rows when externally timed out.
- `e27e3d3e771179e617fa2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `e83ee1877b45faa60ebd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/receipt.json). SHA-256 `60130b7bf55eb16abf11d6778c397f8e8b354ac6331087b98d72ef1c4a0b054b`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e32a6c1307fdebcd88e3b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/stdout.tsv). SHA-256 `0ea28d8f949648e4ad03d4691e7431351857ab8d74c73fdb78a21cc80bcac639`. Unmodified original stdout, including partial rows when externally timed out.
- `edd1ea97035d3f877b545` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_FF/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). SHA-256 `62acb18227dbc5c9f1d53f9adafe6017177b93e17bdb0dd25931bee8187932d2`. Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e35bea67022bd253b14ea` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/receipt.json). SHA-256 `9193eac4022c85ac1a8aa9d4c95e12539956aa2ca0246aa940dedd02c232681c`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea8c528036e944ec0b151` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/stdout.tsv). SHA-256 `cbf77820ba0eeb8c58c11be3e7469c51435cd438ae466c8a42f74b8d6afe3e38`. Unmodified original stdout, including partial rows when externally timed out.
- `e33c642e09c3d79d0a117` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_RAW_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.
- `ea6710b90a9d4a45b6dff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/receipt.json). SHA-256 `a2f03ec27c2910e6c3299d5d45aa5433be693b78f6ec9022f1c4e53b24271aae`. Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e232664a62ee2253f4055` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/stdout.tsv). SHA-256 `0d8cbce36d0624c64970789d7b363da3ed513e24308e7df0dd3c3f6d082c87be`. Unmodified original stdout, including partial rows when externally timed out.
- `efd97dbfa9705d953d8c4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ORIGINAL_LIST_ENCODED_ZERO/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified original stderr for this isolated process.

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
- [rust/tcl-vm/src/cmd_event/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_list_event_scripts_retain_counted_name_objects` (linked): Original List callbacks with three counted name variants across six profiles (18 controls). The assertion covers result and cell access, not headers, refcounts or arbitrary conversion effects. No execution claim.
- [runtime/rust/src/cmd_event/native_original_tests.rs](../../../../runtime/rust/src/cmd_event/native_original_tests.rs), `cmd_event::native_original_tests::original_list_event_scripts_retain_counted_name_objects` (linked): Original List callbacks with three counted name variants across six profiles (18 controls). The assertion covers result and cell access, not headers, refcounts or arbitrary conversion effects. No execution claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
