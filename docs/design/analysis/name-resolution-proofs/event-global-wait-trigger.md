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

## Historical references

- `e98d4bc726037374525cd` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ebcd14ee056c41f642dc1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e1799f8f23cd143629768` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e30bb9be41c1824d28e5d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaa5f919dc246958bc5d1` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclEvent.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e2f9702e19cdde19e5959` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e50602151a40b9938af06` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eb029205417a105e12c46` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ed35df3983d9d47f766da` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e812a02d6c53140c521b3` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e93d0bb942b35f0ac36fa` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ef368fd443939e7aad345` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee8856bb4b5e61800a239` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e8ccbc6059640d42a87b9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ec7eec673ea2335416d68` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3e97ed30e9df1cf4b664` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e41bbfa82065f9fc329c9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e2106861514c4bab46c8d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ea3ca61315f4a1c839087` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0b9ff68648da0cfd74cc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e8a9e8a29c21d575d9aae` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e60d24545805acf1811fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e8a9a046ab3a237d8908a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ed704e19420d173670a02` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3dfbf59c5ee7fbcb5034` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e03224d21c4c87f29eb1a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `efb5f9f11c7dae0ba804c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e504fa070c6f5d4e271ac` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e80b264d261c0f3db47b5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e625e4b0bfc4ac486c7d2` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee39a4b4ed85a5b2bce1d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e64f81976c8e23ebbd246` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ed2dfd77654b5b34db08b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e95be77c51fdbc8296e39` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e304496a389bea9526b3d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e05828d73d31496b4248e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee0c1a573aa3f7120a562` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e742f9220c2352cce39cf` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e669aedb6ddf4238ec209` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eb4e84e926d82eecc98cb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec744ca4997170110b8fe` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e02b39f8e6384033b7cc5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec17d8d7e67f43bd83a5c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0ecb2ad186007cc114e9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e61ab276cb86b9c798097` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e51687290d1684dd7311c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e63ca5d243e22c3901da6` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e7a44edadc05e4d445adc` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e0bf8f0bf1551700f91a4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e37f412abe3c571fa4a69` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ea188e980c5f37553ed0a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6cdd6b699bcaec9ae025` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e8c087d856d315425bf0b` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed38bf2850a70a63ce41f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ebdf9dacb6afa3a72889c` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e51d0eb576539f77bf18d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/ARRAY_SAME_WRITE/stderr). Unmodified original stderr for this isolated process.
- `ee432b6085ef1e6b0e769` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e686339a9cdf130452eca` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e846d6c87e23c044acd84` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/UNSET_WRITE/stderr). Unmodified original stderr for this isolated process.
- `e63555db492bc59dec4a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ef4b6ee6f77847137ef94` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e5db503c263f70680115e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e81916b6ba75efea37403` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eeb36599c3215f08f2c4b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ecfaa1a92b88447f70332` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e2afd4f8493d3bdbf489c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e311a698b39fb2c5b5fe3` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec1891a442aec7a35690a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1111eeb336fa0b449143` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e254c68c47a1dc1e89fd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e26e9aafcf150286ea68d` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e726e310ed0b1beebfd9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e1f12e4c0410c1f7d7597` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e4355469cdda835b3d35b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e795c27955d6dacf47479` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e29721bc0023735856a08` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `efde3f6e39dd4bdfe530e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e1e2926d68d8c8125b14b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ec73ecc5813510f923fbc` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e95e38a2a0d4872903841` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74851a9c97676b1fc01c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e98f7ba416a307b79935a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e895e0e1d8c9daf956ed7` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e89c3b9113e0598316010` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e8f5c054e4729d00aae43` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e9460c8f055914b282d89` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `eec087308347432c3a521` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e11bc541b868fcfd18412` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eaa1130656d3534bc8f78` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e48b2a3426c7892aa1c1f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e540253596f19c00be5cf` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e6455dc73505aed94a2fd` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/SAME_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4c41998c75a2ba9161ef` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e6b1d6f14c4020eb38bf8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ec60d7b0b0284a5e5f912` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/ARRAY_STAGGER/stderr). Unmodified original stderr for this isolated process.
- `e7c34dde0c53985052c96` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e9bc2f5655bf9f5432ff2` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e53f606c21c0035a4d8ab` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e4abaf9e2310a73659fd1` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e937f4c1fc354adc2b00e` (input): [rust/tcl-registry/tests/data/native_event_original/v110/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v110/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e86edb0e779ef261853b1` (input): [rust/tcl-registry/tests/data/native_event_original/v110/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e11ad7d3e6ca0017d7dac` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e82b1b87503ea6330233e` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ebe25f4185c511c53f673` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eee7511390183d6b0be48` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e599d187be8f51790be14` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.4.20/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e9e7da6b95aa88e96b795` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed0356bd1549efcda4874` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e37f5ff98bb89c68f3b64` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e5352c1d3f62c2eab3a7c` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1602a5f87f0a79a5c1d8` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e02424c25909c2b948766` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eceac53e6f780ab72a0e4` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.5.19/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e6abab5f2777db83bba88` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eaf62fc74493d7d23a633` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee0780541b2c87ecf2397` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e811d971d7bd79a2f0945` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed99090b5fb234e2679ad` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e565cfa65f9cc5c16b95a` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee5017a512beab740fb67` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/8.6.18/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e8477748147741b87b721` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e633c4e432fe514175496` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e0c3c92d577a0203ae502` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `eaa487ba939eedabcd8a6` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e634caf9e5dabb567ff60` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4c856809b0364a007f0c` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eac4021ca3115e6ee3677` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.0.4/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e2d7aeef03bb198a2c158` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3a8c4b38ee4bcbfc5f97` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ea8417d91100f662bd532` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `e8ac3540057c95c7ac803` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed49596e4d3c16fee8a93` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2529cf08b29d7b818c9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ee917d56592dd487dc0cd` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/9.1.0/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.
- `ef7c3f7c417488841dfdb` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e657400aab6cdbbf8dccb` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eaccb6bcd4005cd6e9fad` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/SAME_NESTED/stderr). Unmodified original stderr for this isolated process.
- `eb3ef7004c763d54513ba` (provider): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ed87f554161c81e00af40` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `effb5018ee1a838b8e55b` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ed21d651003ac44d5a6a5` (observation): [rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v110/capture/jim/ARRAY_NESTED/stderr). Unmodified original stderr for this isolated process.

## Source inspection

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1176–1216. Retained reference `e98d4bc726037374525cd`.

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

tcl8.4 8.4.20, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEvent.c`, function `VwaitVarProc`, lines 1220–1231. Retained reference `e98d4bc726037374525cd`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1321–1368. Retained reference `ebcd14ee056c41f642dc1`.

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEvent.c`, function `VwaitVarProc`, lines 1372–1383. Retained reference `ebcd14ee056c41f642dc1`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1387–1447. Retained reference `e1799f8f23cd143629768`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEvent.c`, function `VwaitVarProc`, lines 1450–1463. Retained reference `e1799f8f23cd143629768`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1498–1867. Retained reference `e30bb9be41c1824d28e5d`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEvent.c`, function `VwaitVarProc`, lines 1916–1933. Retained reference `e30bb9be41c1824d28e5d`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `Tcl_VwaitObjCmd`, lines 1536–1906. Retained reference `eaa5f919dc246958bc5d1`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEvent.c`, function `VwaitVarProc`, lines 1955–1972. Retained reference `eaa5f919dc246958bc5d1`.

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

jim 0.84-9-g5bac7c9, revision `5bac7c9`, `/workspace/.proofs/native-providers/jimtcl/jim-eventloop.c`, function `JimELVwaitCommand`, lines 565–645. Retained reference `e4521e47b782c5165c471`.

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
