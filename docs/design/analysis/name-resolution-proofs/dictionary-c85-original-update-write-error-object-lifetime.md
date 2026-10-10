# naming.dictionary.c85-original-update-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

The defined lifetime of a dictionary result needs direct reached cleanup evidence. Public caller bytes alone cannot establish a safe storage-equivalence contract or identify the object ownership exercised by an exact compiled write-trace-error control.

## Question

At the exact original C8.5 dictionary-update write-error control, what live refcount, defined-scalar/same-cell checks and watched free-hook counter does the separately pinned instrumented current archive record?

## Conclusion

The separately pinned current C8.5.19 archive94313727 produces the original case9 public row. Its independent observer archive340cb367 records errors1, live refcount_before_release1, same_cell1 and defined_scalar1 immediately before the selected INST_DICT_UPDATE_END error-cleanup TclDecrRefCount, followed by final_free1 at the watched TclFreeObj entry. The watch is cleared at that entry without reading a freed header. Controls0/19 record errors0, unobserved sentinels-1 and final_free0. Baseline and observer case9 whole JSON stdout are byte-identical; all three observer case/result-code/result-hex fields exactly match their corresponding immutable C8.5 columns; this finite byte agreement does not establish safe object/storage identity or equivalence of another archive. All nine actual baseline/build/observer commands exit0. Build-object stderr retains654 bytes of compiler warnings; observer stderr retains the intentional99/102-byte counter rows. This is an observed live pre-release cell/header check plus a reached watched free hook in the independently instrumented current archive, not a private lifetime observation of the separately preserved archive99e0d524 capture. No freed-header dereference, allocation-reuse model, original fixture replacement, generic dict-update/dict-with lifetime, other provider, compiler/frame/admission or Rust pass follows. The current VM source comparator retains all329 captured outer process-code observations and compares326 complete public result windows plus three bounded captured C8.5/C8.6 update-error and C8.6 with-error diagnostic/read-status windows. Each bounded window is reported after its independently observed original update/with final free: even outer completion, diagnostics and caller read status are captured process observations, without defined storage, completion, read or effect guarantees. Complete other windows do not certify their lifetime. The separate C8.5/C8.6 update and C8.6 with VM host-storage controls check their own safe live headers and dictionary members under unchanged source, independently of native freed contents and private archived-executable identity. An executed software outcome belongs only to its exact source/image in the independent Rust validation ledger.

## Scope

The unchanged original C API probe and original counted case9 source run with Tcl_CreateInterp/Tcl_EvalObjEx and no Tcl_Init. The baseline uses current archive94313727. The scratch observer replaces only tclExecute.o/tclObj.o compiled with the original configured CC_SWITCHES; guest source and original public field reporting are unchanged. The added final probe line reports integer counters. Live pre-decrement checks retain refcount, original variable-pointer comparison and scalar-defined state; TclFreeObj records the watched header and clears its watch. Full original/instrumented sources and patches, actual archive/object/executable bytes, all commands/receipts and complete public stdout/warning/counter stderr are retained. Strict-pin225 input and its exact exec_command tool response retain a pre-compilation rejection. The tool response is not an independently captured launcher stdout/stderr/process receipt or native completion. The C8.5 pre-release counters and free hook remain the bounded original observation. Excluding the freed-value field does not turn remaining post-free fields into defined guarantees; the current comparator and separate own-host controls retain that distinction.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Current archive94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc; independently observed archive340cb36741b0067bd2f1d3f7aedab7bdb40f4ea386d49b572f919cb399d6e75b. Actual sources/headers/configured Makefile/object/executable and command pins retained. Runtime version query/compiler version unrecorded.. Channel: Original unchanged C API probe: Tcl_CreateInterp without Tcl_Init, original counted Tcl_EvalObjEx; independent live integer counters in separately built archive and final stderr only.. Dialect: Pinned original provider API/source purpose.

Original case9 observer: errors1/refcount_before_release1/same_cell1/defined_scalar1/final_free1. Controls0/19: errors0/refcount/same_cell/defined_scalar-1/final_free0. Baseline and observed original public row equality is independently checked and does not grant old-archive lifetime identity. All nine actual commands exit0; exact compiler warnings and expected observer stderr are preserved.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No exact original lifetime-counter/provider process is executed for this question.

## Exact evidence

- `native_dict_write_error_lifetime226-Makefile.observer` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/Makefile.observer). SHA-256 `917e4d33de8b5d6198a6bf7870d725954eee23254995e6618bcc1f7917e1d3a4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/archive-copy.json). SHA-256 `b595dd236d4ae7659f5c8bee13ce44efb64da93de3d579949b85d0a94918a1e8`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.receipt.json). SHA-256 `29c9d652646ced292f3914db627cbb00c51a73607ba72187367240ba6c4069aa`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stdout). SHA-256 `eaff8f339fcc47a1eee86e30a42a01ec2c8309bec38b8600ae85bf93515ca8bf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.receipt.json). SHA-256 `0b2877dd797e7cb8b2bedfe925abfd42003bb75fdc3d627e90f836ce937c2e6b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-baseline-current-probe` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-probe](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-probe). SHA-256 `61f9356e3a1c007ec47e2d88e621689166c49c53e8a2b211129286a63c5cb40b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-archive-replacement.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.receipt.json). SHA-256 `f1cda6b79445a6c9d061e0456351b07f5101545203f7d8d172c40ab5d83a03d9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-archive-replacement.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-archive-replacement.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-archive-replacement.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-execute.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.receipt.json). SHA-256 `199b9c78f57f0206840a8f83a43a5089e921afa95cc2d4cc93327bff3336c9e6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-execute.stdout). SHA-256 `c7cdc8ba66bedbf07b6a37a60d20c133afa63bf3195dfaa3fa3c094ec24c3e0a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.receipt.json). SHA-256 `34a49be1151e03aff7accb95d6e818b3483e0de2e2971d4a57e91996c9030668`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.stderr). SHA-256 `28f0dae250ea4f153084e7eae7129fd3750223b11aaf8836cfca63aa17f0fef0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/build-object.stdout). SHA-256 `186ec4735fcfa05dcc33765979242f72a073bf6bcad8a1351070b8461980e539`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-capture-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/capture-summary.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/capture-summary.json). SHA-256 `060b168aa8a2d79ee4e2e63d8012a55e36989ee9ca4391984d5f855b10022eb9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-launcher225-launcher-tool-response.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-launcher225/launcher-tool-response.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-launcher225/launcher-tool-response.json). SHA-256 `e4ef4da29d6fef460afdef4f96bc089eb1b5200a32d9ecd68170b7b141d9488c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-capture.py` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/capture.py). SHA-256 `99f06412d6a44c4482af54a34932490008dcf1957a721c2e3858efcea8c3a8ff`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-instrumentation.patch` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumentation.patch](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumentation.patch). SHA-256 `04467fb3fa9ce8272dec02e6f917ff1b94f9d4430ef6dacb02f0786b2e166d1b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-instrumented-probe.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-probe.c). SHA-256 `2fc93c8d36f35ec1d1179babc229c8980bd77ff4a2b3b488e9f49c8f4ff86070`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-instrumented-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-source/tclExecute.c). SHA-256 `22f01f3793bed82e911214cb9aaae7dde37952371c00f1969ff9a46023a8782e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-instrumented-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/instrumented-source/tclObj.c). SHA-256 `aacbdc95101446dbb8e6f880003f2af2ce232402bde953825f5f274794712db7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-probe.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-probe.c). SHA-256 `e95451728352a915dfbc033719e60aabcab9ae4f53566088b583bbd4f12c6f3d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-provider.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-provider.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-provider.json). SHA-256 `187bdbec01f287f656fe948b49f38cfe7641d69f0771b9cc15e2c8a5fb040992`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source.tcl). SHA-256 `092c3aa2025b580b926d4b065e10d1d83d86e5620e20a17da9d58ee2c6f67b33`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c). SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c). SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclVar.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclVar.c). SHA-256 `24f29b0694b5e755f99ab98a0486a63807de9d368b81df65e71ebacb95817768`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-probe-instrumentation.patch` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/probe-instrumentation.patch](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/probe-instrumentation.patch). SHA-256 `fd1a8662384690502a8610dc2a0d5f3b4b0272da08abb32bd8a065fae05a72c2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-failed-request225-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/request.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/request.json). SHA-256 `05adf103f7d2c48a06451f8a89badacc2b91250a847c3481114843bad675ebf4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-libtcl8.5-observed.a` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/libtcl8.5-observed.a](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/libtcl8.5-observed.a). SHA-256 `340cb36741b0067bd2f1d3f7aedab7bdb40f4ea386d49b572f919cb399d6e75b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.receipt.json). SHA-256 `7af0912e528f67e48bb7241d9ed6611163621ef9ad0d9b620ab932b5636fb989`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stderr). SHA-256 `1e91f1304dbdce3cb96c254d1f0a362a2e22a9c4060cdaa4a100ad6d716548e9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stdout). SHA-256 `9250aa92b34ff0ff1b87358bcfa4ac93aba4d0f3b4f88d08f6b6dec6a6077fc6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.receipt.json). SHA-256 `a8dc9fa33db2d539f7d42c03ec8bc3acf8d453b1a1dfe1d868b1828e2243e1d7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stderr). SHA-256 `1e91f1304dbdce3cb96c254d1f0a362a2e22a9c4060cdaa4a100ad6d716548e9`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stdout). SHA-256 `fe97708fc91e69ab88bd15086d57de883d0f2c6a660b7494d91224b66a3b2e7a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.receipt.json). SHA-256 `4260e29b28564770308ec06d50310f839abc984445cd8bd6edd824373f5c0071`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stderr). SHA-256 `9fd0e374738a9c464df576e466204d07b133e4eb8452fe3e8131a99e2c40bc49`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stdout). SHA-256 `eaff8f339fcc47a1eee86e30a42a01ec2c8309bec38b8600ae85bf93515ca8bf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.receipt.json). SHA-256 `0492344115aea1234a49a58929e184462f90c7bd76d8cc6a159b78ee396b7df8`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-observed-probe` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-probe). SHA-256 `6b0d04ae1d1b62d4c9a85f23b626442f2a4af182fb5eb437cad21a1a42b55c2a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-original-columns.tsv` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/original-columns.tsv](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/original-columns.tsv). SHA-256 `a0033268917b943996f7de5b5775af158e0963728c909d69709e37a03852dcaf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-capture.py` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/capture.py). SHA-256 `0fef1acbe3a745e88c00646e86720c9cf3eb167a04b8ed8fd96d3c94af8e1bab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-instrumentation.patch` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumentation.patch](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumentation.patch). SHA-256 `04467fb3fa9ce8272dec02e6f917ff1b94f9d4430ef6dacb02f0786b2e166d1b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-instrumented-probe.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-probe.c). SHA-256 `2fc93c8d36f35ec1d1179babc229c8980bd77ff4a2b3b488e9f49c8f4ff86070`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-instrumented-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-source/tclExecute.c). SHA-256 `22f01f3793bed82e911214cb9aaae7dde37952371c00f1969ff9a46023a8782e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-instrumented-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/instrumented-source/tclObj.c). SHA-256 `aacbdc95101446dbb8e6f880003f2af2ce232402bde953825f5f274794712db7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-probe.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-probe.c). SHA-256 `e95451728352a915dfbc033719e60aabcab9ae4f53566088b583bbd4f12c6f3d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-provider.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-provider.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-provider.json). SHA-256 `187bdbec01f287f656fe948b49f38cfe7641d69f0771b9cc15e2c8a5fb040992`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl). SHA-256 `092c3aa2025b580b926d4b065e10d1d83d86e5620e20a17da9d58ee2c6f67b33`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclExecute.c). SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclObj.c). SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-original-source-tclVar.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source/tclVar.c). SHA-256 `24f29b0694b5e755f99ab98a0486a63807de9d368b81df65e71ebacb95817768`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-probe-instrumentation.patch` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/probe-instrumentation.patch](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/probe-instrumentation.patch). SHA-256 `fd1a8662384690502a8610dc2a0d5f3b4b0272da08abb32bd8a065fae05a72c2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-request-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/request.json](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/request/request.json). SHA-256 `19e5819807e7b83a606b54a3908e119bc11859416ff71c2e5fc00c8290948569`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-tclExecute.o` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/tclExecute.o](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/tclExecute.o). SHA-256 `1c0ada3d0cb895fbc6b5ae0ca7288ef84eaf81308a551f116db6115a070f80f4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_dict_write_error_lifetime226-tclObj.o` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/tclObj.o). SHA-256 `9eecb77c5b9bb33b6ddd030985d5dfd29b3230d3e4d3f2f71d32151bab1ddc0c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END`, lines 7253–7270. Full-source SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`; snippet SHA-256 `7350bc5bb95be7be250ecc7e7ec9fd07b1b8698b6397598450797cd096d8457c`; retained evidence `native_dict_write_error_lifetime226-request-original-source-tclExecute.c`.

```text
	    Tcl_IncrRefCount(dictPtr);
	    TclDecrRefCount(varPtr->value.objPtr);
	    varPtr->value.objPtr = dictPtr;
	} else {
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVar(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		result = TCL_ERROR;
		goto checkForCatch;
	    }
	}
	NEXT_INST_F(9, 1, 0);
    }

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclVar.c`, function `TclPtrSetVar`, lines 1927–1955. Full-source SHA-256 `24f29b0694b5e755f99ab98a0486a63807de9d368b81df65e71ebacb95817768`; snippet SHA-256 `bd8b18bd16f917974085de910540ba4f21e60408946f74042da2ab838d78cce1`; retained evidence `native_dict_write_error_lifetime226-request-original-source-tclVar.c`.

```text
	    }
	}
    } else if (newValuePtr != oldValuePtr) {
	/*
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/original-source/tclObj.c`, function `TclFreeObj`, lines 1432–1459. Full-source SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`; snippet SHA-256 `3c598527abe361cef89e2fd069d356056cfbc00f9d8d97522763f3985dcbe67f`; retained evidence `native_dict_write_error_lifetime226-request-original-source-tclObj.c`.

```text
}
#else /* TCL_MEM_DEBUG */

void
TclFreeObj(
    register Tcl_Obj *objPtr)	/* The object to be freed. */
{
    /* Invalidate the string rep first so we can use the bytes value
     * for our pointer chain, and signal an obj deletion (as opposed
     * to shimmering) with 'length == -1' */

    TclInvalidateStringRep(objPtr);
    objPtr->length = -1;

    if (!objPtr->typePtr || !objPtr->typePtr->freeIntRepProc) {
	/*
	 * objPtr can be freed safely, as it will not attempt to free any
	 * other objects: it will not cause recursive calls to this function.
	 */

	TCL_DTRACE_OBJ_FREE(objPtr);
	TclFreeObjStorage(objPtr);
	TclIncrObjsFreed();
    } else {
	/*
	 * This macro declares a variable, so must come here...
	 */


```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/instrumented-source/tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END observer`, lines 7267–7283. Full-source SHA-256 `22f01f3793bed82e911214cb9aaae7dde37952371c00f1969ff9a46023a8782e`; snippet SHA-256 `af9459c6d75d768e5f82af44639763d8cb1330a2832924f1efc7daa66658501f`; retained evidence `native_dict_write_error_lifetime226-request-instrumented-source-tclExecute.c`.

```text
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		result = TCL_ERROR;
		goto checkForCatch;
	    }
	}
	NEXT_INST_F(9, 1, 0);

```

tcl8.5 8.5.19, revision `Exact original/current source and separately instrumented scratch build; source-control revision unrecorded.`, `/workspace/.proofs/native-dict-write-error-lifetime226-request/instrumented-source/tclObj.c`, function `TclFreeObj observer`, lines 1440–1455. Full-source SHA-256 `aacbdc95101446dbb8e6f880003f2af2ce232402bde953825f5f274794712db7`; snippet SHA-256 `0e89197a06e615117d77c34b9addc754fc332694e2363b7ff7dab38a56af4a60`; retained evidence `native_dict_write_error_lifetime226-request-instrumented-source-tclObj.c`.

```text
TclFreeObj(
    register Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }

    /* Invalidate the string rep first so we can use the bytes value
     * for our pointer chain, and signal an obj deletion (as opposed
     * to shimmering) with 'length == -1' */

    TclInvalidateStringRep(objPtr);
    objPtr->length = -1;

    if (!objPtr->typePtr || !objPtr->typePtr->freeIntRepProc) {

```


## Consumer bindings

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain329 unchanged captured outer process-code references and326 complete results plus three bounded post-free observations: C8.5/C8.6 update-error and C8.6 with-error. Every later outer/result/diagnostic/caller-read field remains captured output without defined storage/completion/read/effect or another archived executable private lifetime; the original seven signal controls remain separate.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c85_dictionary_write_error_retains_defined_host_storage` (linked): Read unchanged baseline/observer bytes and reached counters, then separately assert the VM own safe live header/member policy under original source. Backend host storage is not native freed-content or private archived-header identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
