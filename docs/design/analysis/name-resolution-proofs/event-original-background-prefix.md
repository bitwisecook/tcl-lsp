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

## Historical references

- `e7ca34f2d6fd9344d4610` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.4.20/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eb83569215d92fa37da10` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ef8bb2ebdccb8192ef78d` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e5cad7b4101332ff96f7c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ea9bf8d7d5c098c478df8` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4d5b46a62af04d177edb` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `ea3de3c863d05700ccfc3` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `eaf331dbfda81fff6c4db` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclTimer.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e791ac2cd6ff96da72529` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e4521e47b782c5165c471` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/jim/jim-eventloop.c). Complete retained pinned translation unit; the exact LF function window is separately recorded.
- `e123724388f8662dc5479` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4d552cae616020850bad` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e1013c02b786ae4787640` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e09479906ff5e8421f6f8` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e74e2ebf3504b3ca2ded9` (input): [rust/tcl-registry/tests/data/native_event_original/v106/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v106/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e12af5b939567f6623a39` (input): [rust/tcl-registry/tests/data/native_event_original/v106/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ef618b8f2a212bdd89ab6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `efc1919f2cad632a99862` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e86127283c231113b4bd5` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e4c30cdcc3daab9ce078f` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e08d3ad19e836d289fe8a` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e8713f35d778bb238f37a` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e1ea51c8f7a716eb60841` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e86d502dd6b79bc0ae21e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e656388044f19358644ca` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `efbf876bad68e15c877bf` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e97f5cda7f0779ed6ba12` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e2a808efc8b870155c485` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e036464c5dfee454cff5d` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `eea7018d6967845a3ab1c` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ebf8040312905f78db7bb` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `eac1303463e81a05ff9bd` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e2e121b714f2a48d1f22e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `ef20c4d901c733bab9ef6` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e4cc10151aeb0a283b3c4` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e3286a5ae533600d53a4e` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e470e3e985c3b2e7220a9` (observation): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `ec18a9b2f362f4345c580` (provider): [rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v106/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e6ed8c66cd0858b1fe2a4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ee38af258aca8ad271b7a` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e27c55a0804cd093d531c` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e83750ae8f3064b887dbc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.4.20/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ef83b5f3e2f393e153833` (input): [rust/tcl-registry/tests/data/native_event_original/v107/probe.c](../../../../rust/tcl-registry/tests/data/native_event_original/v107/probe.c). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `e696fc855fcac987b7333` (input): [rust/tcl-registry/tests/data/native_event_original/v107/inputs.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/inputs.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ec0cee9a10d970f6c2dcc` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture.py](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture.py). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `eeeaa10710fc163da2d56` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/queue.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/queue.json). Exact original input program or pinned harness for this capture variant. Original absolute paths/commands remain recorded; no new execution.
- `ed33acf513a9307d61b6e` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e585c9206a7c664b9dbb4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e581229313b566bac24c8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e5bd6d84fcc277fde5523` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.5.19/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `ee0f227382f1ee56c78f0` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `ed95b63d15133bdb11612` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `eafca7a79840a7053ff9f` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e65298f60f97813f2bea6` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/8.6.18/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e69e4d234d255693865f4` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e949b294fef43a8891fc5` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e3eccada9da550cd9faf8` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `eb8281dc03697ed866d24` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.0.4/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e833642caa9092df8aa07` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e7be4aa3551c56b251b05` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `e00166f233b13cba954af` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `e27005c71aa6473f5c3c2` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/9.1.0/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.
- `e11603269e92a396c6e93` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/receipt.json). Exact isolated-case process status, reported version, public protocol rows and outside timeout bound.
- `e440393829c37fa4b823b` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stdout.tsv). Unmodified original stdout, including partial rows when externally timed out.
- `ef7480ddb114e209a4341` (observation): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stderr](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/GLOBAL_PREFIX_BGERROR/stderr). Unmodified original stderr for this isolated process.
- `ef84dde9b34a3c2f2313d` (provider): [rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_event_original/v107/capture/jim/receipt.json). Exact configured source/header/library/Makefile/probe/runner/executable associations and compile command; associated fresh case receipts.

## Source inspection

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

tcl8.5 8.5.19, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInterp.c`, function `SlaveBgerror`, lines 2097–2116. Retained reference `ef8bb2ebdccb8192ef78d`.

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

tcl8.6 8.6.18, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInterp.c`, function `ChildBgerror`, lines 2268–2289. Retained reference `ea9bf8d7d5c098c478df8`.

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

tcl9.0 9.0.4, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInterp.c`, function `ChildBgerror`, lines 2342–2363. Retained reference `ea3de3c863d05700ccfc3`.

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

tcl9.1 9.1.0, revision `retained configured source tree`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInterp.c`, function `ChildBgerror`, lines 2727–2748. Retained reference `e791ac2cd6ff96da72529`.

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


## Consumer bindings

- [rust/tcl-registry/src/native_event.rs](../../../../rust/tcl-registry/src/native_event.rs), `NativeEventProtocol`: Pure independently selected event purpose; no original-object or callback-free grant.
- [rust/tcl-cmd-core/src/event.rs](../../../../rust/tcl-cmd-core/src/event.rs), `EventQueue`: Live original script-handle queue and current service-turn topology.
- [rust/tcl-vm/src/cmd_event.rs](../../../../rust/tcl-vm/src/cmd_event.rs), `report_bg_error`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `process_bg_errors`: Port-owned genuine original object/current frame consumer; does not infer independent Normal or Native preparation.

A named test is a coverage binding, not a claim that it executed.

## Replay

This record replays no interpreter. Original compile/run commands, input lengths, deadline status and source/header/library/executable hashes are retained in immutable receipts. Captured output directories and original absolute provisioning paths must not be overwritten. Source inspection can be reproduced from the attached full-file and exact LF/snippet hashes. No Rust execution claim.
