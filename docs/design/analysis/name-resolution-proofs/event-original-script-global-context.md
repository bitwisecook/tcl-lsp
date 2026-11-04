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

## Historical references

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e82bde6ea86354ca82c9b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eab0ebdce23d2b8fb2e3d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9a8002705e1a2b64d2a8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef4b434db4305c93fa04e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `edc1e83aca47cc9392f36` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e640d06939660d2393b74` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `ee1b7f7617169073897af` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e79205d2ae998d5a38673` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3e06ba4dd754f84e7e64` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `e29e3f7504be1e6186b5f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee08b9a64f471521ea7d6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3e64357e5d9edc9603e2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e4da0dad00b144f2d84b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb458bd4525034a789ada` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e03c642460973a4a71bf1` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4cddff13a14363486cd6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e083744aa190aa6b11a05` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e328169da8d6e71512e9a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `e00089d0781440c179788` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ede77607a6e3e5b6fe12f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e51802ab802d88a1b81f0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `eee602952d4b918098df6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e65d8386702d49432e609` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3d293e66456f98648886` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e3bf64c9cc0e463891bf7` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e462e52fe279b3c913d8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed1577e1a62eafcdb0bdb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eed23a20a0104b1261f17` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e791339df6453253e3d7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e20f480d24b4dc4267ff5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `e9c2d2284dd4d892c5646` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e31eb883b262af10420a2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e976aff5ad43f905d94bf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `e25f775b9b31952683c47` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e00ad21bce1bde9752240` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee9fd72d3cf3f25042c64` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `ead267513fde61d3a4089` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e78e165002788f2fae147` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed484834653d0e4f0ace8` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e18bc58805088eae322ff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef22b225d0907df5e8f26` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e893b9b51f2461fb33262` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `e88a09d8915be94a2c28b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5473775471ca1fcdc422` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eb4c690475a43a7f4e9ec` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `e6eda56329e5344e77920` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebe44981b65f5c67066ac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea3d77b738c2eb78ea1be` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e40b200d1cb7f0a117120` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e89934a3483a811078b8c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3a128d3c675c0b71d777` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ebbacadbe86729017ee83` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e138c17fa020177b0236f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec87f97a725ac3962e667` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `e1ccb3a915087d24f788d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e346125b0898fb3210e6b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ede16036b2674c4976026` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `e16c0a7f6b9fcc92dbd25` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e207806f45de34a350562` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0e6ef567ac12a2a70bfe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.
- `e34c3b6b3352510dcef42` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec0aa600a32f71a8aba32` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1a94d3ed5a18d3a8a3a0` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_NAMESPACE/stderr). Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ea73d42750afc50e5d1fa` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e5da3ced5266ce42e8fff` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3c5e025f16621d82decf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/GLOBAL_FROM_PROC/stderr). Unmodified original stderr for this isolated process.
- `e6fbc574f1a76a96d31c3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e43776fb771dfd241990d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e012199e98b3a08793a66` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/CONCAT_TRIM/stderr). Unmodified original stderr for this isolated process.
- `e5295b320aea9635c0347` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1ee05298aff9dce615c2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e7ac7eda60b44ddcb74dd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PURE_LIST_SCRIPT/stderr). Unmodified original stderr for this isolated process.

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclTimer.c`, function `AfterProc`, lines 1005–1052. Retained reference `e7ca34f2d6fd9344d4610`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclTimer.c`, function `AfterProc`, lines 1114–1158. Retained reference `eb83569215d92fa37da10`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclTimer.c`, function `AfterProc`, lines 1168–1212. Retained reference `e5cad7b4101332ff96f7c`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclTimer.c`, function `AfterProc`, lines 1147–1191. Retained reference `e4d5b46a62af04d177edb`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclTimer.c`, function `AfterProc`, lines 1533–1577. Retained reference `eaf331dbfda81fff6c4db`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `Jim_EvalObjBackground`, lines 112–148. Retained reference `e4521e47b782c5165c471`.

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
