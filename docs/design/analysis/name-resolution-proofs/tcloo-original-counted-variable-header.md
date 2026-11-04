# naming.tcloo.original-counted-variable-header

Kind: `native-observation`

## Problem statement

Equal printed variable names cannot establish preservation of the original counted object. A direct declaration/query control must retain its original allocation, public query and explicit same-object comparison separately from byte equality.

## Question

Does direct TclOO variable declaration with the original counted a-NUL-z object return that same original object through info class variables, and what counted bytes does the direct query retain?

## Conclusion

C8.6.18/C9.0.4/C9.1.0 record DECLARE0 and QUERY0/list-parse 0/count 1/same-object 1 with counted bytes61007a. The same-object Boolean directly compares the sole public query list element pointer with the original allocated declaration object; it is not inferred from equal text. C8.4.20/C8.5.19/Jim record the original stock-TclOO-unavailable NOT_APPLICABLE branch. All six external processes and six actual compiler builds exit 0; stderr is empty. This exact local pointer comparison and byte getter observe only this original declaration/query object within one fresh interpreter. No cross-interpreter object equality, header fields/type/cache equivalence, source-word materialisation, physical variable store/read, class lifetime/method dispatch or compiler/frame/admission follows. BIG-IP and Rust assertions are not executed by this proof.

## Scope

The original ASCII LF C driver creates a public Tcl_NewStringObj from three original bytes61 00 7a, keeps declaration/query argv references, calls Tcl_EvalObjv(flags0) separately for oo::define C variable and info class variables, calls Tcl_ListObjGetElements on the direct public result and compares its sole member pointer with the original. Counted result bytes use Tcl_GetStringFromObj. C providers use full Tcl_Init; Jim core/static initialisation records its measured unavailable branch. No textual Tcl source constructor substitutes for the original counted object; no addresses or private fields are emitted.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual driver 9d9c655fa6ee258d48e57a6ead517192a3f11367005f296bb7be4a906e6ea051 independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact original stock-TclOO-unavailable NOT_APPLICABLE branch. No original direct declaration/query or object-comparison Boolean is observed; no absence of arbitrary extension API is inferred. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual driver fc8c89480951efc7243dbb626a81f607c62054490557ad698d85014095979b97 independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact original stock-TclOO-unavailable NOT_APPLICABLE branch. No original direct declaration/query or object-comparison Boolean is observed; no absence of arbitrary extension API is inferred. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual driver 90c44dde5d2ee3a3f177e29f49283dda7133d4932d9fcd296eed4f30131d4471 independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact public DECLARE code 0; direct QUERY code 0, list parser code 0, element count 1, direct sole-member/original pointer-comparison Boolean 1, counted getter bytes61 00 7a. The explicit Boolean observes this original object within the same interpreter only; equal bytes alone are not its authority. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual driver f7b64083c8c2498a074e2e49032708ac389f8c4d5f0407e0556180af34bd400d independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact public DECLARE code 0; direct QUERY code 0, list parser code 0, element count 1, direct sole-member/original pointer-comparison Boolean 1, counted getter bytes61 00 7a. The explicit Boolean observes this original object within the same interpreter only; equal bytes alone are not its authority. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual driver 9bba62867e7015c1486d6dde70040db6824eaa8ce62c903432b81039012b2947 independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact public DECLARE code 0; direct QUERY code 0, list parser code 0, element count 1, direct sole-member/original pointer-comparison Boolean 1, counted getter bytes61 00 7a. The explicit Boolean observes this original object within the same interpreter only; equal bytes alone are not its authority. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual driver b0526041746516226eb7e90bec55e751da6c9eb870fcb35875e575aef4e74daa independently verified with complete compiler/link command and all required source/library/header/build pins; compiler version unrecorded.. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Dialect: Pinned original provider API/source purpose.

Exact original stock-TclOO-unavailable NOT_APPLICABLE branch. No original direct declaration/query or object-comparison Boolean is observed; no absence of arbitrary extension API is inferred. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Historical references

- `native_oo_variable_header300-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-REQUEST.md` (input): [rust/tcl-registry/tests/data/native_oo_variable_header300/REQUEST.md](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/REQUEST.md). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-capture.py` (input): [rust/tcl-registry/tests/data/native_oo_variable_header300/capture.py](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/capture.py). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/stderr](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_variable_header300/jim/stdout](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-probe.c` (input): [rust/tcl-registry/tests/data/native_oo_variable_header300/probe.c](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/probe.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_variable_header300-request.json` (input): [rust/tcl-registry/tests/data/native_oo_variable_header300/request.json](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/request.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::variable_declaration_introspection_retains_original_counted_object` (linked): Actual NativeCore C8.6/C9.0/C9.1 Runtime declaration/query control compares the retained original counted object directly, independently of byte equality or arbitrary Logical repinning. This binding claims no executed Rust outcome.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::unknown_method_preserves_original_selector_for_returned_slot_data` (linked): A separate Rust dispatcher control keeps the actual original counted selector header through an overridable unknown method. Native300 directly observed only the declaration/query boundary; no extra native intermediate observer is claimed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.


## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 2 | 0 | 0 | 0 |
| `isolated` | 2 | 0 | 0 | 0 |

[Exact original body hashes and process outcomes](rust-validation/frozen349/question-body-outcomes.json) join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.
