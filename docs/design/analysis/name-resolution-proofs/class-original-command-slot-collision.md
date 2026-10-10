# naming.class.original-command-slot-collision

Kind: `native-observation`

## Problem statement

A namespace-relative object creation can encounter an existing ordinary procedure. Reporting a similarly rendered object label cannot establish that the original destination command was free or replaced.

## Question

When original relative TclOO creation targets p occupied by an ordinary procedure in current a:, what caught error, subsequent procedure result and command query are observed?

## Conclusion

C8.6/C9.0/C9.1 retain ORIGINAL0 `1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0`: creation is caught with code1, the original ordinary procedure remains callable and the original relative object query is empty. The separate copy-labelled input stops at ORIGINAL1 while attempting to create ::source, which already names the stock source command; no destination copy collision is reached there. C8.4/C8.5/Jim reach ORIGINAL0/NOT_APPLICABLE in both sources. All12 host processes and six compiler commands exit0 with empty stderr. Independent pinned C source explains its own inspected original naming/publication code; no private table/header/token identity, general object replacement policy, caller frame or Native compilation/Normal result is issued by public equality or source replay. For this question the Runtime control consumes both unchanged collision originals across six columns (12 comparisons), retaining failed ::source setup separately. VM consumes the one creation-collision original across six columns; no copy entry is inferred. Shared linked controls total48 Runtime and24 VM comparisons across the creation/collision/copy records; these are assertion definitions, not an executed pass or another native provider observation.

A marked shared VM definition binds the one original create-over-command collision program as its independent 6-window public subset.

## Scope

Two unchanged ASCII LF sources execute separately under full original provider initialisation and counted C API evaluation. The creation collision is a reached caught guest error; the copy-labelled input retains an independent setup failure before copying. All original input/requests/drivers/actual commands/ELFs/whole streams/receipts and required provider/archive/header/build/source pins are retained. Independently reached copy inputs have their own original-copy-holder-and-collision record. BIG-IP and Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass.

The fixture selects an actual software core/compiler and compares public original bytes, without launching new original processes or observing private native token/header/cache identity. Constructed-holder reporting and collision results remain separate original questions. No passing software outcome, oo::copy availability, copy permission, activation/frame or compiler instruction admission is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Every unchanged source reaches ORIGINAL0/NOT_APPLICABLE through its actual capability branch. No supported TclOO/coroutine API or substitute Jim/C implementation is inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Every unchanged source reaches ORIGINAL0/NOT_APPLICABLE through its actual capability branch. No supported TclOO/coroutine API or substitute Jim/C implementation is inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Complete ORIGINAL code/value fields: create-over-ordinary-command-in-original-holder=0/1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0; copy-over-ordinary-command-in-original-holder=1/can't create object "::source": command already exists with that name. The ::source setup failure in copy-over-ordinary-command-in-original-holder occurs before copying and supplies no copy result.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Complete ORIGINAL code/value fields: create-over-ordinary-command-in-original-holder=0/1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0; copy-over-ordinary-command-in-original-holder=1/can't create object "::source": command already exists with that name. The ::source setup failure in copy-over-ordinary-command-in-original-holder occurs before copying and supplies no copy result.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Complete ORIGINAL code/value fields: create-over-ordinary-command-in-original-holder=0/1 {can't create object "p": command already exists with that name} ORIGINAL_PROC 0; copy-over-ordinary-command-in-original-holder=1/can't create object "::source": command already exists with that name. The ::source setup failure in copy-over-ordinary-command-in-original-holder occurs before copying and supplies no copy result.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Exact fresh full provider initialisation with independently required executable/archive/header/source/build and counted-driver ELF pins; compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Every unchanged source reaches ORIGINAL0/NOT_APPLICABLE through its actual capability branch. No supported TclOO/coroutine API or substitute Jim/C implementation is inferred.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_oo_original_collision249-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.receipt.json). SHA-256 `2111620426974f0d44e5dacfb7d8816b27128fe0c010cc0fd3612d6c10161ff5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `e0855a04c57975965593cc1c8d68a5948030e2f84f8e6de18c25dc2c9a81537e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `2a48e0363a3a20ef7e00ab50bd2ab36a03991a28767fc68ef4840c8031e21b76`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `84357df21f17f66cca09748c23fec56dcefd4bec74c9fd763c046c1b19bb5e4f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `2a48e0363a3a20ef7e00ab50bd2ab36a03991a28767fc68ef4840c8031e21b76`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.4.20/probe.elf). SHA-256 `05f4d331c8694eabdafe2850f6cec2ebd873ab8f28489542a3ac6b4130493c18`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.receipt.json). SHA-256 `c0be20140ae7fcdc68f7191ab781a9e7bbe7a7dc73acc889cd14f9e110c7ea5f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `de12cbb463122d7495bb12bba54931184a35375936b9e6cedc9fcc3658ddd2aa`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `9c4236fd91194e5aacb35e4ad87dad849a1e9a36006b31a9524891d2289e8c77`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `50d43f89f16360cfc57190e76ae8f126f73b079d7fef4d73bb2e71840443f4b0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `9c4236fd91194e5aacb35e4ad87dad849a1e9a36006b31a9524891d2289e8c77`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.5.19/probe.elf). SHA-256 `293413fad21f45573888dfe20752df635b7e933df56a8a42ba31f8b7dd641e4f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.receipt.json). SHA-256 `2ab5b647303de3f8080724302db02b4cd0c277b3011cf0f08b3169b8ffbb7eb7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `7e74f149a9464c9162f19d087327b1e7884aff4fc123d96b0eff7c39df4d6445`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `b1c4c0c7da40525bc6446528e2dc17395a90e222bef2a16ae12d1233bcd91b29`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `b5d2d6cf94776c8c0c0ef296c0d5ee08746707fd6d6ee05c6e6747cd6b3d2eb6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `a4a40d29a5390254fd42263c119241d990fc7d19dc94ecde928bcd6f4b897b33`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/8.6.18/probe.elf). SHA-256 `f0cd376df3769984586301045a4a46d89cf1e8d1a5dc0b9109ee67c4f9aa6f56`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.receipt.json). SHA-256 `9e65150550536ae7cc2da8d871a1315b07954fc2f9895d1e57f8c07b9956edba`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `803710af0a88c2c068ffdc9b716b2e195bb1a33aa3ebaf615af7cffe10ceb1ad`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `68cb1fdb98212589694f5996d5cd04bcdf8ed928ec83a1cd9ca74ff1ad2b6f7f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `111bec7379ac6f841ac229c758c28a37f5dcbc4663f6b9e1229dd33019752db4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `1457ffd0e34933970f9560da963c2cc464b265de1ef9c8cd561084d7db9b44b0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.0.4/probe.elf). SHA-256 `b9f2588b8239707391180b1e383f59ba3adda48bf9e18b7fae01da67ac6c43ab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.receipt.json). SHA-256 `e5f18dbd236ddf42be230be81f9ab590f9597694df950e85565eeb91bd27262e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `e69f48f8f2fabacd7239aa4652c85eeca5e1d5ebed4b4417d9e498eb4acebc62`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `72341ec11fb2457490397e2be3fceee96fe25afb95b231dd8c23434b83c83d18`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `2559d5e2f41e56a9f7c2fe5e7f61ec1be980cdbb482fa0a8409907ba8cc95db6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `3259301b396c5b7689abf8727a3d3591239ceaddf1cc01a440492da273eee4e8`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/9.1.0/probe.elf). SHA-256 `1bc81545f4ee1b52144049ec1e25ed3c5110fbc6b5269b120250cb4f655c8528`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-capture.py` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/capture.py](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/capture.py). SHA-256 `392a6fa0fa7313545ce269b3972973310bf5f8e553b1fe6a9a14c043d7ed6e4c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-copy-over-ordinary-command-in-original-holder.tcl` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/copy-over-ordinary-command-in-original-holder.tcl](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/copy-over-ordinary-command-in-original-holder.tcl). SHA-256 `e59c07652739aec7cbc96f8623be67482064f7d5f59fadbcbbddc641dfd92b98`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-create-over-ordinary-command-in-original-holder.tcl` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/create-over-ordinary-command-in-original-holder.tcl](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/create-over-ordinary-command-in-original-holder.tcl). SHA-256 `8eca6e09d9b0f8fc841854a15d8157896483eba11174b24bd0b7cf9d8bbf98e0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.receipt.json). SHA-256 `f1a8c739e2112d528e6d7f2a69a9f2fcda5a2c1f50c51dee75ed642b34c5d6a4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-copy-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `a79d732666f3bd6f743d338422a4092d7767758fd00832abcecfaa8aeb94c6f3`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-copy-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-copy-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/copy-over-ordinary-command-in-original-holder/stdout). SHA-256 `3663965ccba45ea5fa85db6981518703f10eafe8b9ba8667332dd58fa31b87fe`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-create-over-ordinary-command-in-original-holder-receipt.json` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/receipt.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/receipt.json). SHA-256 `a7cc289b154804d3eddfac7ccc20748ce6a027f0872cd080498dba1be035b91a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-create-over-ordinary-command-in-original-holder-stderr` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stderr](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-create-over-ordinary-command-in-original-holder-stdout` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stdout](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/create-over-ordinary-command-in-original-holder/stdout). SHA-256 `3663965ccba45ea5fa85db6981518703f10eafe8b9ba8667332dd58fa31b87fe`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_oo_original_collision249/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/jim/probe.elf). SHA-256 `6f1e064d4a3d7451d30405a106407a8b0d689c176299621a934d5a96015b3a16`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-original-request.json` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/original-request.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/original-request.json). SHA-256 `e7d5cb55a08f85f824422db9510fcbcdc3e4ae03d8d3a5fae8951e6d94cb9251`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-probe.c` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/probe.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/probe.c). SHA-256 `5f3ce148bc84c374eec674275763ec575eb3466f21fd860f7009845549f96759`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request.json` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/request.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request.json). SHA-256 `e7d5cb55a08f85f824422db9510fcbcdc3e4ae03d8d3a5fae8951e6d94cb9251`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-copy-over-ordinary-command-in-original-holder.tcl` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/copy-over-ordinary-command-in-original-holder.tcl](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/copy-over-ordinary-command-in-original-holder.tcl). SHA-256 `e59c07652739aec7cbc96f8623be67482064f7d5f59fadbcbbddc641dfd92b98`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-create-over-ordinary-command-in-original-holder.tcl` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/create-over-ordinary-command-in-original-holder.tcl](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/create-over-ordinary-command-in-original-holder.tcl). SHA-256 `8eca6e09d9b0f8fc841854a15d8157896483eba11174b24bd0b7cf9d8bbf98e0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-request.json` (input): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/request.json](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/request.json). SHA-256 `e7d5cb55a08f85f824422db9510fcbcdc3e4ae03d8d3a5fae8951e6d94cb9251`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclNamesp.c). SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclOO.c). SHA-256 `e749370dcaeab6d214b811a246536c1cf06f92edf2c51c8f1dbd8f4a6f57b3ef`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclNamesp.c). SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclOO.c). SHA-256 `2e26684094bbe01c988c20386de1a3bf53646f47ce84ad50a807414455d1d3a7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclNamesp.c). SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclOO.c). SHA-256 `f2abb0b2ca6fa8cf621f158f209a48ee20ec2a08073d2f2c4af541ddf0babbbd`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-root-launch.log` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/root-launch.log](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/root-launch.log). SHA-256 `6562c6321ddf4e420e86c664f06f791c8eb26532a76e05b03ffd991a59137f6a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-8.6.18-tclBasic.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-8.6.18-tclNamesp.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclNamesp.c). SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-8.6.18-tclOO.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/8.6.18/tclOO.c). SHA-256 `e749370dcaeab6d214b811a246536c1cf06f92edf2c51c8f1dbd8f4a6f57b3ef`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.0.4-tclBasic.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.0.4-tclNamesp.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclNamesp.c). SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.0.4-tclOO.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.0.4/tclOO.c). SHA-256 `2e26684094bbe01c988c20386de1a3bf53646f47ce84ad50a807414455d1d3a7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.1.0-tclBasic.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.1.0-tclNamesp.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclNamesp.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclNamesp.c). SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_oo_original_collision249-source-anchors-9.1.0-tclOO.c` (observation): [rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclOO.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/source-anchors/9.1.0/tclOO.c). SHA-256 `f2abb0b2ca6fa8cf621f158f209a48ee20ec2a08073d2f2c4af541ddf0babbbd`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 1792–1841. Full-source SHA-256 `e749370dcaeab6d214b811a246536c1cf06f92edf2c51c8f1dbd8f4a6f57b3ef`; snippet SHA-256 `38e9990bd80e609574f57d8ad05e781c5223cd3b40094ca05550b0f74bad4b8d`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy,
	*inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    Tcl_SetErrorCode(interp, "TCL", "OO", "OVERWRITE_OBJECT", (char *)NULL);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2354–2403. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `c4e4c78ffd40ee691dc9386aed2c62e1fe4ffe5283bac06026aeb8fb9df8b1c4`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    ClientData clientData,	/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * [***] This is wrong.  See Tcl Bug a16752c252.
	 * However, this buggy behavior is kept under particular circumstances
	 * to accommodate deployed binaries of the "tclcompiler" program
	 * <http://sourceforge.net/projects/tclpro/> that crash if the bug is
	 * fixed.

```

tcl8.6 8.6.18, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2141–2190. Full-source SHA-256 `b4b3095f69b2192f13bf355d70a0efeb9aa78b5e6ceccf6b17536cc5e9be1e24`; snippet SHA-256 `8690834281fbb37e17992d2a85a0d5af1189cfffc37978ddab82a4ff4fb03993`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr) /* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 2134–2183. Full-source SHA-256 `2e26684094bbe01c988c20386de1a3bf53646f47ce84ad50a807414455d1d3a7`; snippet SHA-256 `78c65abdc83974adc1f48d401e9982479f11ceaab9e6bf135825255b92d01f26`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy;
    Namespace *inNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    OO_ERROR(interp, OVERWRITE_OBJECT);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2946–2995. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `60d68076d13c75854313ed89ca4f290c4157a83e49c536af476491879c3994f9`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    int deleted = 0, isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.
	 */

```

tcl9.0 9.0.4, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2301–2350. Full-source SHA-256 `73e79904d662d12564dd7d35f922fd896566411524a6e14c6454325f2ce17b8d`; snippet SHA-256 `8690834281fbb37e17992d2a85a0d5af1189cfffc37978ddab82a4ff4fb03993`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr) /* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `TclNewObjectInstanceCommon`, lines 2115–2164. Full-source SHA-256 `f2abb0b2ca6fa8cf621f158f209a48ee20ec2a08073d2f2c4af541ddf0babbbd`; snippet SHA-256 `78c65abdc83974adc1f48d401e9982479f11ceaab9e6bf135825255b92d01f26`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclOO.c`.

```text
TclNewObjectInstanceCommon(
    Tcl_Interp *interp,
    Class *classPtr,
    const char *nameStr,
    const char *nsNameStr)
{
    Tcl_HashEntry *hPtr;
    Foundation *fPtr = GetFoundation(interp);
    Object *oPtr;
    const char *simpleName = NULL;
    Namespace *nsPtr = NULL, *dummy;
    Namespace *inNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

    if (nameStr) {
	TclGetNamespaceForQualName(interp, nameStr, inNsPtr,
		TCL_CREATE_NS_IF_UNKNOWN, &nsPtr, &dummy, &dummy, &simpleName);

	/*
	 * Disallow creation of an object over an existing command.
	 */

	hPtr = Tcl_FindHashEntry(&nsPtr->cmdTable, simpleName);
	if (hPtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "can't create object \"%s\": command already exists with"
		    " that name", nameStr));
	    OO_ERROR(interp, OVERWRITE_OBJECT);
	    return NULL;
	}
    }

    /*
     * Create the object.
     */

    oPtr = AllocObject(interp, simpleName, nsPtr, nsNameStr);
    if (oPtr == NULL) {
	return NULL;
    }
    oPtr->selfCls = classPtr;
    AddRef(classPtr->thisPtr);
    TclOOAddToInstances(oPtr, classPtr);

    /*
     * Check to see if we're really creating a class. If so, allocate the
     * class structure as well.
     */

    if (TclOOIsReachable(fPtr->classCls, classPtr)) {
	/*

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2902–2951. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `17063e4965d879b920a21894c1c16df941c322675901d6909ab29184f57b92a0`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
TclCreateObjCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,	/* Name of command, without any namespace
				 * components. */
    Tcl_Namespace *namesp,	/* The namespace to create the command in */
    Tcl_ObjCmdProc2 *proc,	/* Object-based function to associate with
				 * name. */
    void *clientData,		/* Arbitrary value to pass to object
				 * function. */
    Tcl_CmdDeleteProc *deleteProc)
				/* If not NULL, gives a function to call when
				 * this command is deleted. */
{
    bool deleted = false;
    int isNew = 0;
    Command *cmdPtr;
    ImportRef *oldRefPtr = NULL;
    ImportedCmdData *dataPtr;
    Tcl_HashEntry *hPtr;
    Namespace *nsPtr = (Namespace *) namesp;

    /*
     * If the command name we seek to create already exists, we need to delete
     * that first. That can be tricky in the presence of traces. Loop until we
     * no longer find an existing command in the way, or until we've deleted
     * one command and that didn't finish the job.
     */

    while (1) {
	hPtr = Tcl_CreateHashEntry(&nsPtr->cmdTable, cmdName, &isNew);

	if (isNew || deleted) {
	    /*
	     * isNew - No conflict with existing command.
	     * deleted - We've already deleted a conflicting command
	     */
	    break;
	}

	/*
	 * An existing command conflicts. Try to delete it...
	 */

	cmdPtr = (Command *)Tcl_GetHashValue(hPtr);

	/*
	 * Command already exists; delete it. Be careful to preserve any
	 * existing import links so we can restore them down below. That way,
	 * you can redefine a command and its import status will remain
	 * intact.

```

tcl9.1 9.1.0, revision `Pinned independent original provider source`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclNamesp.c`, function `TclGetNamespaceForQualName`, lines 2298–2347. Full-source SHA-256 `fce8ca8ee9a21f2e87f7a1917e7a2e4109f9fadf72fd0e585673d1450409b815`; snippet SHA-256 `d7db985acab61d1526075a352098e1c187ca7f042e8af80f3389ffe04c823a5b`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclNamesp.c`.

```text
TclGetNamespaceForQualName(
    Tcl_Interp *interp,		/* Interpreter in which to find the namespace
				 * containing qualName. */
    const char *qualName,	/* A namespace-qualified name of an command,
				 * variable, or namespace. */
    Namespace *cxtNsPtr,	/* The namespace in which to start the search
				 * for qualName's namespace. If NULL start
				 * from the current namespace. Ignored if
				 * TCL_GLOBAL_ONLY is set. */
    int flags,			/* Flags controlling the search: an OR'd
				 * combination of TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS, and
				 * TCL_CREATE_NS_IF_UNKNOWN. */
    Namespace **nsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from *cxtNsPtr or, if
				 * TCL_GLOBAL_ONLY is set, if qualName is
				 * found in the global :: namespace. NULL is
				 * stored otherwise. */
    Namespace **altNsPtrPtr,	/* Address where function stores a pointer to
				 * containing namespace if qualName is found
				 * starting from the global :: namespace.
				 * NULL is stored if qualName isn't found
				 * starting from :: or if the TCL_GLOBAL_ONLY,
				 * TCL_NAMESPACE_ONLY, TCL_FIND_ONLY_NS,
				 * TCL_CREATE_NS_IF_UNKNOWN flag is set. */
    Namespace **actualCxtPtrPtr,/* Address where function stores a pointer to
				 * the actual namespace from which the search
				 * started. This is either cxtNsPtr, the ::
				 * namespace if TCL_GLOBAL_ONLY was specified,
				 * or the current namespace if cxtNsPtr was
				 * NULL. */
    const char **simpleNamePtr)	/* Address where function stores the simple
				 * name at end of the qualName, or NULL if
				 * qualName is "::" or the flag
				 * TCL_FIND_ONLY_NS was specified. */
{
    Interp *iPtr = (Interp *) interp;
    Namespace *nsPtr = cxtNsPtr, *lastNsPtr = NULL, *lastAltNsPtr = NULL;
    Namespace *altNsPtr;
    Namespace *globalNsPtr = iPtr->globalNsPtr;
    const char *start, *end;
    const char *nsName;
    Tcl_HashEntry *entryPtr;
    Tcl_DString buffer;
    int len;

    /*
     * Determine the context namespace nsPtr in which to start the primary
     * search. If the qualName name starts with a "::" or TCL_GLOBAL_ONLY was

```


## Consumer bindings

- [runtime/rust/src/cmd_oo/native_original_holder_tests.rs](../../../../runtime/rust/src/cmd_oo/native_original_holder_tests.rs), `cmd_oo::native_original_holder_tests::original_oo_publication_preserves_constructed_holder_and_collision_results` (linked): Compare the creation-collision original across six full columns; ordinary procedure preservation remains the measured value and no VM copy API or private slot identity is donated. Selected Jim full-init conformance ingress explicitly installs the shared source distribution; bootstrap is distinct from core-only and metadata purposes, with unchanged original rows and no executed pass claim.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs), `original_result`: Decode the exact original OO completion/result fields, preserving holder-creation and collision fixture groups independently of reporting labels.
- [rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_original_holder_tests.rs), `cmd_oo::native_original_holder_tests::original_oo_publication_preserves_constructed_holder_and_collision_results` (linked): The shared VM body compares24 public completion/result windows across four unchanged ASCII originals and six selected core/compiler fixtures. This question binds only the one original create-over-command collision program, 6 of those windows; equal reporting names cannot prove holder equality and no oo::copy entry is inferred.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
