# naming.list.arithseries-original-multiple-index-conversion

Kind: `native-observation`

## Problem statement

A finite arithmetic sequence query needs its exact selected index spelling, public result and observed representation type. Result equality alone cannot establish representation retention or a huge-allocation/host-limit contract.

## Question

For the original nine-element lseq value, what caught result and reported representation type follow single-index, index-list path, separate-index and out-of-range path calls?

## Conclusion

C9.0.4/C9.1.0 return original completion code 0 with the single-index payload `0 0 arithseries`. Both valid index-list path and separate-index controls return `0 0 list`; the out-of-range index-list path returns `0 {} arithseries`. The first payload field is the original caught call code, the second its public result and the third the selected type word reported for the retained input sequence after the call. C8.4.20/C8.5.19/C8.6.18/Jim report the original UNAVAILABLE branch for each control. All 24 fresh external source processes and six driver compilations exit 0 with empty stderr. These finite reported types and values grant no object-address/refcount/private-member identity, compiler/opcode choice, general laziness, huge allocation or arbitrary out-of-range behaviour. Independently pinned C9 TclLindexFlat source inspections distinguish its single-index abstract special case from its generic multiple-index conversion loop; those text facts are a separate evidence channel. BIG-IP and Rust assertions are not tested.

## Scope

Four exact original ASCII LF source controls run independently in fresh full Tcl_Init or Jim core/static initialisation under the unchanged counted Native202/210/222 C API driver, with no added outer source envelope. Each available source creates lseq9, catches its exact original lindex spelling, reads ::tcl::unsupported::representation and emits only the extracted public type word with caught result/code. Addresses and complete representation descriptions are not emitted. Complete original source/requests, driver, launch script, actual commands/receipts/stdout/stderr/ELFs and provider source/header/archive/build pins are preserved. Two full pinned tclListObj.c copies and exact TclLindexFlat excerpts are inspected independently; source text does not identify private state during the executed call.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: each control returns UNAVAILABLE under its original lseq guard; no lindex/type observer branch is reached. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: each control returns UNAVAILABLE under its original lseq guard; no lindex/type observer branch is reached. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: each control returns UNAVAILABLE under its original lseq guard; no lindex/type observer branch is reached. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: single-index=0 0 arithseries; index-list-path=0 0 list; separate-indices=0 0 list; out-of-range-path=0 {} arithseries. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: single-index=0 0 arithseries; index-list-path=0 0 list; separate-indices=0 0 list; out-of-range-path=0 {} arithseries. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual original full provider initialisation; required source/header/archive/build and captured driver ELF pins retained. Compiler version unrecorded. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Dialect: Pinned original provider API/source purpose.

Exact four original payloads: each control returns UNAVAILABLE under its original lseq guard; no lindex/type observer branch is reached. All driver ORIGINAL completion codes are 0, and every process exits 0 with empty stderr. Public result and selected reported input type remain distinct from private object or huge-materialisation claims.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Pinned original provider API/source purpose.

No appliance execution answers this exact question.

## Exact evidence

- `native_arithseries_multiple_index231-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.receipt.json). SHA-256 `915a463d3cfefb6525fcf609ba2821f4cdd8f034df4060c0f24a60a12d908e8d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/receipt.json). SHA-256 `2edd2470964fb8b72c1e8b147c29a5f3402aba08b41d4392da94c947f36f09f4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stdout). SHA-256 `10ce921ac299424f9a8ab3b057f7882433bd92c5b1881af19103985b21b465ce`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/receipt.json). SHA-256 `9cfc604af0cfc31d0930d40b5def59ecfc1eaf1b742179899c3aa7c76744601d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stdout). SHA-256 `10ce921ac299424f9a8ab3b057f7882433bd92c5b1881af19103985b21b465ce`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/probe.elf). SHA-256 `05f4d331c8694eabdafe2850f6cec2ebd873ab8f28489542a3ac6b4130493c18`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/receipt.json). SHA-256 `3562a6aecb45792571699b51a5a2e29a76a9eef0ca6de420eb7766170771e58a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stdout). SHA-256 `10ce921ac299424f9a8ab3b057f7882433bd92c5b1881af19103985b21b465ce`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/receipt.json). SHA-256 `4354a48b240b54ed804c49dfe309bb7794829aa46fa62bf09b5794c9c86bd768`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stdout). SHA-256 `10ce921ac299424f9a8ab3b057f7882433bd92c5b1881af19103985b21b465ce`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.receipt.json). SHA-256 `d2fda4d72e010c2761f674f256a9d9a02f9489319ad0c5c9ce1f47b9fa3adc86`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/receipt.json). SHA-256 `81ba14dad3f11c3d6d24cba6037e1189e071c0f778310e1927ebfb1a2dcae39d`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stdout). SHA-256 `eff5ad699e17a4b4fe53c910645d9beaa7ec783b9ec283cf43f4894cbb4f9a51`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/receipt.json). SHA-256 `df9eafe6d57f191f202618e688e9b8a6346a977bffdc79c9d0f65367dd7e65d0`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stdout). SHA-256 `eff5ad699e17a4b4fe53c910645d9beaa7ec783b9ec283cf43f4894cbb4f9a51`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/probe.elf). SHA-256 `293413fad21f45573888dfe20752df635b7e933df56a8a42ba31f8b7dd641e4f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/receipt.json). SHA-256 `4fbe3527343d1be74ee29364f57900d999ed395f769d9ef4857ae48687d069b4`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stdout). SHA-256 `eff5ad699e17a4b4fe53c910645d9beaa7ec783b9ec283cf43f4894cbb4f9a51`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/receipt.json). SHA-256 `b991fb94e4b311b6426c98d46a8ee79e415e4b51c1bff404a7af198d8caa9242`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stdout). SHA-256 `eff5ad699e17a4b4fe53c910645d9beaa7ec783b9ec283cf43f4894cbb4f9a51`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.receipt.json). SHA-256 `0d587216c864dc49188c0517e893691a49166351a86a9da05607b91b6dfaa450`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/receipt.json). SHA-256 `cb820c6a7bf8cd03c326d21ad287d7bfcdd31e7e9ad58525863f4e1e215c250a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stdout). SHA-256 `f401dbcc240981548a002d3e2139bd86704d53c835a236810681a871ab2caecf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/receipt.json). SHA-256 `0f608a48c146810cf68375056c837381738a913fa925bfc9ee9541f9dd226894`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stdout). SHA-256 `f401dbcc240981548a002d3e2139bd86704d53c835a236810681a871ab2caecf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/probe.elf). SHA-256 `f0cd376df3769984586301045a4a46d89cf1e8d1a5dc0b9109ee67c4f9aa6f56`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/receipt.json). SHA-256 `7a59d27d02f6dda90cbe4e20544a8520ac7fa2605cd5548d79b31cba9aa4efab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stdout). SHA-256 `f401dbcc240981548a002d3e2139bd86704d53c835a236810681a871ab2caecf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/receipt.json). SHA-256 `1942dc60ac1ce6fc080cc34f412b28e271ffd724e92a7e0e62e9e93d2b4ba08e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stdout). SHA-256 `f401dbcc240981548a002d3e2139bd86704d53c835a236810681a871ab2caecf`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.receipt.json). SHA-256 `9a4dae56c4d49cf0db02f22bea0fbbaf67fcad7d90a35bad5f00f580270315bb`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/receipt.json). SHA-256 `790192907ad2e71f972310fe4895f52168864e14f319cc66f798bca4336cb3b8`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stdout). SHA-256 `2105aa7507583ee19096bfd4788f8a2c8943de805a8d5585ce892aaf95c3e03c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/receipt.json). SHA-256 `face0db4d32f499c600a885305629fcc4bbffe90755faf17c296acc5e6aedc18`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stdout). SHA-256 `12077b02177f4134f96ce6f68cc652222fef8dfbe4a42feaaa4d965b91893c56`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/probe.elf). SHA-256 `b9f2588b8239707391180b1e383f59ba3adda48bf9e18b7fae01da67ac6c43ab`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/receipt.json). SHA-256 `8ecb99418216974a0f653c8be327533967deca4489be60f3ed19f3a8ba3eb11c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stdout). SHA-256 `2105aa7507583ee19096bfd4788f8a2c8943de805a8d5585ce892aaf95c3e03c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/receipt.json). SHA-256 `574858469344c7eafc75fbc0eda522c1a7c57bf7db570581b40dbb7e4d4e9d20`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stdout). SHA-256 `dc6c44ca252c8d7bd8b8188e58e006816186067343e967f63e904e2824d8ad74`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.receipt.json). SHA-256 `2e87e30c44a2276d5246aa282513b4f94cec797e48306ee3b12c3071f22d97df`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/receipt.json). SHA-256 `1d168b6673b5dd3a52b1e7e93bfea3c322e6e3324d77cb805d6ede8bd1f196b6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stdout). SHA-256 `de8e6b4eba79ecfad8411d8d4cba6f8d3b8e2de4bdc7d28c0a9034ed8f7efe5e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/receipt.json). SHA-256 `82079b1107bb0e09f72fcbe84c55add5358692b5250dabfc8b72e3fa873e732f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stdout). SHA-256 `f795fedcbb3d4439a934d2fcf096561d47d47d4c33c3821ccf3759da80c66674`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/probe.elf). SHA-256 `1bc81545f4ee1b52144049ec1e25ed3c5110fbc6b5269b120250cb4f655c8528`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/receipt.json). SHA-256 `0a6e372e83bd19955150af97ebb5e91e012674282816a4681c326762846eed95`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stdout). SHA-256 `de8e6b4eba79ecfad8411d8d4cba6f8d3b8e2de4bdc7d28c0a9034ed8f7efe5e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/receipt.json). SHA-256 `b3be945c682fe4b73ea55674f47c9271e38536e22798457b0a47e94f72689d69`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stdout). SHA-256 `ed4a6a0e0b211ade5fea879c4ceabfb0eb64b2511439c370b0b0a091cc7b1bfe`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-capture.py` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/capture.py](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/capture.py). SHA-256 `5ca36afbb4bb19b6bfbe4ec6393fa0bfc891d29ab4831615395ff582963b3519`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-index-list-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/index-list-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/index-list-path.tcl). SHA-256 `acd330f56a3be7e81d372c2d17aa28e0da2c8c089e0ef337e13add93312e6c97`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.receipt.json). SHA-256 `225fec483bcbd2636323e275c241b69d7a7d430c0332f38562f1e534acd4c2f5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/receipt.json). SHA-256 `8a0cf68c8ec18b9bcd0c57f23aa4e7b5f00a5296a0e47d48e908341ed96cfd1f`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stdout). SHA-256 `16c6194adc18a7104286e524a2a7e7edf1af25c67528e187dbcbeab5eb01a26a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/receipt.json). SHA-256 `1837f0186c383dbc4652bd6432637a30960ca7123190247df0b5fc735aa6b781`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stdout). SHA-256 `16c6194adc18a7104286e524a2a7e7edf1af25c67528e187dbcbeab5eb01a26a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/probe.elf). SHA-256 `6f1e064d4a3d7451d30405a106407a8b0d689c176299621a934d5a96015b3a16`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/receipt.json). SHA-256 `edce9f640f1351b4bd082eaa2e5b6fb3f26c6a115f346aa71ea7c8f1758f6685`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stdout). SHA-256 `16c6194adc18a7104286e524a2a7e7edf1af25c67528e187dbcbeab5eb01a26a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/receipt.json). SHA-256 `58b73182be1faab3c164bb1926690239ead58f8ff955a75d276aeae8f4e840c6`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stdout). SHA-256 `16c6194adc18a7104286e524a2a7e7edf1af25c67528e187dbcbeab5eb01a26a`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-out-of-range-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/out-of-range-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/out-of-range-path.tcl). SHA-256 `05d62c6ba1b7c8b7a90f68f8ae674171fbb400561fff749dbf4a0f1b33259f7c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-probe.c` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/probe.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/probe.c). SHA-256 `5f3ce148bc84c374eec674275763ec575eb3466f21fd860f7009845549f96759`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request.json` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request.json). SHA-256 `d62b978c3830b5a69cb020e6bbf3028f66be53a783e2c9fe3318a0679be23284`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-index-list-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/index-list-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/index-list-path.tcl). SHA-256 `acd330f56a3be7e81d372c2d17aa28e0da2c8c089e0ef337e13add93312e6c97`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-out-of-range-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/out-of-range-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/out-of-range-path.tcl). SHA-256 `05d62c6ba1b7c8b7a90f68f8ae674171fbb400561fff749dbf4a0f1b33259f7c`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-request.json` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/request.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/request.json). SHA-256 `d62b978c3830b5a69cb020e6bbf3028f66be53a783e2c9fe3318a0679be23284`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-separate-indices.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/separate-indices.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/separate-indices.tcl). SHA-256 `23d9068a5688d86d720d196b3383cb5cf74b4b5c5a89b8d871b9120aba00332e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-single-index.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/single-index.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/single-index.tcl). SHA-256 `50c789189eb5f9704a8ffd9180d79a3463ed0a889b0c73c7871c97cd503055c7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.0.4-TclLindexFlat.txt` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-TclLindexFlat.txt](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-TclLindexFlat.txt). SHA-256 `b3230076f4026fdddf339b9f3386651b4e594fc8755547ec9268e60ecdb79ec5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.0.4-tclListObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-tclListObj.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-tclListObj.c). SHA-256 `15d49e9df1a0e799eb199ab5ceb58e77e0b3587d7200156ddf82344102069d95`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.1.0-TclLindexFlat.txt` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-TclLindexFlat.txt](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-TclLindexFlat.txt). SHA-256 `b3230076f4026fdddf339b9f3386651b4e594fc8755547ec9268e60ecdb79ec5`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.1.0-tclListObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-tclListObj.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-tclListObj.c). SHA-256 `6711651457a8f8813f37c71e5e8958aca9d2e83a6fc7c7e281ae3f7ada10e81b`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-separate-indices.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/separate-indices.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/separate-indices.tcl). SHA-256 `23d9068a5688d86d720d196b3383cb5cf74b4b5c5a89b8d871b9120aba00332e`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-single-index.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/single-index.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/single-index.tcl). SHA-256 `50c789189eb5f9704a8ffd9180d79a3463ed0a889b0c73c7871c97cd503055c7`. Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl9.0 9.0.4, revision `Exact pinned original C release source/excerpt; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclListObj.c`, function `TclLindexFlat`, lines 2729–2833. Full-source SHA-256 `15d49e9df1a0e799eb199ab5ceb58e77e0b3587d7200156ddf82344102069d95`; snippet SHA-256 `b3230076f4026fdddf339b9f3386651b4e594fc8755547ec9268e60ecdb79ec5`; retained evidence `native_arithseries_multiple_index231-request-tcl9.0.4-TclLindexFlat.txt`.

```text
TclLindexFlat(
    Tcl_Interp *interp,		/* Tcl interpreter. */
    Tcl_Obj *listObj,		/* Tcl object representing the list. */
    Tcl_Size indexCount,	/* Count of indices. */
    Tcl_Obj *const indexArray[])/* Array of pointers to Tcl objects that
				 * represent the indices in the list. */
{
    int status;
    Tcl_Size i;

    /* Handle AbstractList as special case */
    if (indexCount == 1 && TclObjTypeHasProc(listObj,indexProc)) {
	Tcl_Size listLen = TclObjTypeLength(listObj);
	Tcl_Size index;
	Tcl_Obj *elemObj = listObj; /* for lindex without indices return list */
	for (i=0 ; i<indexCount && listObj ; i++) {
	    if (TclGetIntForIndexM(interp, indexArray[i], /*endValue*/ listLen-1,
		    &index) != TCL_OK) {
		return NULL;
	    }
	    if (i==0) {
		if (TclObjTypeIndex(interp, listObj, index, &elemObj) != TCL_OK) {
		    return NULL;
		}
	    } else if (index > 0) {
		// TODO: support nested lists
		Tcl_Obj *e2Obj = TclLindexFlat(interp, elemObj, 1, &indexArray[i]);
		Tcl_DecrRefCount(elemObj);
		elemObj = e2Obj;
	    }
	}
	if (elemObj == NULL) {
	    /*
	     * TclObjTypeIndex returns TCL_OK with NULL in elemObj if
	     * index was out of bounds.
	     */
	    TclNewObj(elemObj);
	}
	Tcl_IncrRefCount(elemObj);
	return elemObj;
    }

    Tcl_IncrRefCount(listObj);

    for (i=0 ; i<indexCount && listObj ; i++) {
	Tcl_Size index, listLen = 0;
	Tcl_Obj **elemPtrs = NULL;

	status = Tcl_ListObjLength(interp, listObj, &listLen);
	if (status != TCL_OK) {
	    Tcl_DecrRefCount(listObj);
	    return NULL;
	}

	if (TclGetIntForIndexM(interp, indexArray[i], /*endValue*/ listLen-1,
		&index) == TCL_OK) {
	    if (index < 0 || index >= listLen) {
		/*
		 * Index is out of range. Break out of loop with empty result.
		 * First check remaining indices for validity
		 */

		while (++i < indexCount) {
		    if (TclGetIntForIndexM(interp, indexArray[i],
			    TCL_SIZE_MAX - 1, &index) != TCL_OK) {
			Tcl_DecrRefCount(listObj);
			return NULL;
		    }
		}
		Tcl_DecrRefCount(listObj);
		TclNewObj(listObj);
		Tcl_IncrRefCount(listObj);
	    } else {
		Tcl_Obj *itemObj;
		/* TODO - this will cause shimmering of inner abstract lists! */
		/*
		 * Must set the internal rep again because it may have been
		 * changed by TclGetIntForIndexM. See test lindex-8.4.
		 */
		if (!TclHasInternalRep(listObj, &tclListType)) {
		    status = SetListFromAny(interp, listObj);
		    if (status != TCL_OK) {
			/* The list is not a list at all => error. */
			Tcl_DecrRefCount(listObj);
			return NULL;
		    }
		}

		ListObjGetElements(listObj, listLen, elemPtrs);
		/* increment this reference count first before decrementing
		 * just in case they are the same Tcl_Obj
		 */
		itemObj = elemPtrs[index];
		Tcl_IncrRefCount(itemObj);
		Tcl_DecrRefCount(listObj);
		/* Extract the pointer to the appropriate element. */
		listObj = itemObj;
	    }
	} else {
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	}
    }
    return listObj;
}

```

tcl9.1 9.1.0, revision `Exact pinned original C release source/excerpt; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclListObj.c`, function `TclLindexFlat`, lines 2730–2834. Full-source SHA-256 `6711651457a8f8813f37c71e5e8958aca9d2e83a6fc7c7e281ae3f7ada10e81b`; snippet SHA-256 `b3230076f4026fdddf339b9f3386651b4e594fc8755547ec9268e60ecdb79ec5`; retained evidence `native_arithseries_multiple_index231-request-tcl9.1.0-TclLindexFlat.txt`.

```text
TclLindexFlat(
    Tcl_Interp *interp,		/* Tcl interpreter. */
    Tcl_Obj *listObj,		/* Tcl object representing the list. */
    Tcl_Size indexCount,	/* Count of indices. */
    Tcl_Obj *const indexArray[])/* Array of pointers to Tcl objects that
				 * represent the indices in the list. */
{
    int status;
    Tcl_Size i;

    /* Handle AbstractList as special case */
    if (indexCount == 1 && TclObjTypeHasProc(listObj,indexProc)) {
	Tcl_Size listLen = TclObjTypeLength(listObj);
	Tcl_Size index;
	Tcl_Obj *elemObj = listObj; /* for lindex without indices return list */
	for (i=0 ; i<indexCount && listObj ; i++) {
	    if (TclGetIntForIndexM(interp, indexArray[i], /*endValue*/ listLen-1,
		    &index) != TCL_OK) {
		return NULL;
	    }
	    if (i==0) {
		if (TclObjTypeIndex(interp, listObj, index, &elemObj) != TCL_OK) {
		    return NULL;
		}
	    } else if (index > 0) {
		// TODO: support nested lists
		Tcl_Obj *e2Obj = TclLindexFlat(interp, elemObj, 1, &indexArray[i]);
		Tcl_DecrRefCount(elemObj);
		elemObj = e2Obj;
	    }
	}
	if (elemObj == NULL) {
	    /*
	     * TclObjTypeIndex returns TCL_OK with NULL in elemObj if
	     * index was out of bounds.
	     */
	    TclNewObj(elemObj);
	}
	Tcl_IncrRefCount(elemObj);
	return elemObj;
    }

    Tcl_IncrRefCount(listObj);

    for (i=0 ; i<indexCount && listObj ; i++) {
	Tcl_Size index, listLen = 0;
	Tcl_Obj **elemPtrs = NULL;

	status = Tcl_ListObjLength(interp, listObj, &listLen);
	if (status != TCL_OK) {
	    Tcl_DecrRefCount(listObj);
	    return NULL;
	}

	if (TclGetIntForIndexM(interp, indexArray[i], /*endValue*/ listLen-1,
		&index) == TCL_OK) {
	    if (index < 0 || index >= listLen) {
		/*
		 * Index is out of range. Break out of loop with empty result.
		 * First check remaining indices for validity
		 */

		while (++i < indexCount) {
		    if (TclGetIntForIndexM(interp, indexArray[i],
			    TCL_SIZE_MAX - 1, &index) != TCL_OK) {
			Tcl_DecrRefCount(listObj);
			return NULL;
		    }
		}
		Tcl_DecrRefCount(listObj);
		TclNewObj(listObj);
		Tcl_IncrRefCount(listObj);
	    } else {
		Tcl_Obj *itemObj;
		/* TODO - this will cause shimmering of inner abstract lists! */
		/*
		 * Must set the internal rep again because it may have been
		 * changed by TclGetIntForIndexM. See test lindex-8.4.
		 */
		if (!TclHasInternalRep(listObj, &tclListType)) {
		    status = SetListFromAny(interp, listObj);
		    if (status != TCL_OK) {
			/* The list is not a list at all => error. */
			Tcl_DecrRefCount(listObj);
			return NULL;
		    }
		}

		ListObjGetElements(listObj, listLen, elemPtrs);
		/* increment this reference count first before decrementing
		 * just in case they are the same Tcl_Obj
		 */
		itemObj = elemPtrs[index];
		Tcl_IncrRefCount(itemObj);
		Tcl_DecrRefCount(listObj);
		/* Extract the pointer to the appropriate element. */
		listObj = itemObj;
	    }
	} else {
	    Tcl_DecrRefCount(listObj);
	    listObj = NULL;
	}
    }
    return listObj;
}

```


## Consumer bindings

- [runtime/rust/src/native_arithseries/tests.rs](../../../../runtime/rust/src/native_arithseries/tests.rs), `native_arithseries::tests::finite_arithmetic_index_paths_preserve_or_convert_the_original_primary` (linked): Four selected finite call fragments compare public results and the measured retained input type distinction: single arithseries, valid path/separate list, out-of-range arithseries. The Rust descriptor assertion grants no native refcount/member identity or huge operation behaviour.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
