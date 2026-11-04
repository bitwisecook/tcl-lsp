# naming.list.arithseries-original-multiple-index-conversion

Kind: `native-observation`

## Problem statement

A finite arithmetic sequence query needs its exact selected index spelling, public result and observed representation type. Result equality alone cannot establish representation retention or a huge-allocation/host-limit contract.

## Question

For the original nine-element lseq value, what caught result and reported representation type follow single-index, index-list path, separate-index and out-of-range path calls?

## Conclusion

C9.0.4/C9.1.0 return original completion code 0 with the single-index payload `0 0 arithseries`. Both valid index-list path and separate-index controls return `0 0 list`; the out-of-range index-list path returns `0 {} arithseries`. The first payload field is the original caught call code, the second its public result and the third the selected type word reported for the retained input sequence after the call. C8.4.20/C8.5.19/C8.6.18/Jim report the original UNAVAILABLE branch for each control. All 24 fresh external source processes and six driver compilations exit 0 with empty stderr. These finite reported types and values grant no object-address/refcount/private-member identity, compiler/opcode choice, general laziness, huge allocation or arbitrary out-of-range behaviour. Independently pinned C9 TclLindexFlat source inspections distinguish its single-index abstract special case from its generic multiple-index conversion loop; those text facts are a separate evidence channel. BIG-IP and Rust assertions are not tested.

## Scope

Four exact original ASCII LF source controls run independently in fresh full Tcl_Init or Jim core/static initialisation under the unchanged counted Native202/210/222 C API driver, with no added outer source envelope. Each available source creates lseq9, catches its exact original lindex spelling, reads ::tcl::unsupported::representation and emits only the extracted public type word with caught result/code. Addresses and complete representation descriptions are not emitted. Complete original source/requests, driver, launch script, actual commands/receipts/stdout/stderr/ELFs and provider source/header/archive/build pins are preserved. Two full pinned tclListObj.c copies and exact TclLindexFlat excerpts are inspected independently; source text does not identify private state during the executed call. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

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

## Historical references

- `native_arithseries_multiple_index231-8.4.20-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.4.20-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.4.20/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.5.19-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.5.19/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-8.6.18-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/8.6.18/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.0.4-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.0.4/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-9.1.0-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/9.1.0/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-capture.py` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/capture.py](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/capture.py). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-index-list-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/index-list-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/index-list-path.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/compile.stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-index-list-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/index-list-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-out-of-range-path-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/out-of-range-path/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/probe.elf). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-separate-indices-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/separate-indices/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-receipt.json` (provider): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/receipt.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/receipt.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-stderr` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stderr](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stderr). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-jim-single-index-stdout` (observation): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stdout](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/jim/single-index/stdout). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-out-of-range-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/out-of-range-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/out-of-range-path.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-probe.c` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/probe.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/probe.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request.json` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-index-list-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/index-list-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/index-list-path.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-out-of-range-path.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/out-of-range-path.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/out-of-range-path.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-request.json` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/request.json](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/request.json). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-separate-indices.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/separate-indices.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/separate-indices.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-single-index.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/single-index.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/single-index.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.0.4-TclLindexFlat.txt` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-TclLindexFlat.txt](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-TclLindexFlat.txt). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.0.4-tclListObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-tclListObj.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.0.4-tclListObj.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.1.0-TclLindexFlat.txt` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-TclLindexFlat.txt](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-TclLindexFlat.txt). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-request-tcl9.1.0-tclListObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-tclListObj.c](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/request/tcl9.1.0-tclListObj.c). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-separate-indices.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/separate-indices.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/separate-indices.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.
- `native_arithseries_multiple_index231-single-index.tcl` (input): [rust/tcl-registry/tests/data/native_arithseries_multiple_index231/single-index.tcl](../../../../rust/tcl-registry/tests/data/native_arithseries_multiple_index231/single-index.tcl). Exact unchanged input/actual provider artifact or whole command receipt/stdout/stderr; no selected rows or normalisation.

## Source inspection

tcl9.0 9.0.4, revision `Exact pinned original C release source/excerpt; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclListObj.c`, function `TclLindexFlat`, lines 2729–2833. Retained reference `native_arithseries_multiple_index231-request-tcl9.0.4-TclLindexFlat.txt`.

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

tcl9.1 9.1.0, revision `Exact pinned original C release source/excerpt; source-control revision unrecorded.`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclListObj.c`, function `TclLindexFlat`, lines 2730–2834. Retained reference `native_arithseries_multiple_index231-request-tcl9.1.0-TclLindexFlat.txt`.

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
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `abstract_index_key_clones_keep_fresh_zero_role_without_native_references_or_element_cache` (linked): Software source definition checks abstract index key clones keep fresh zero role without native references or element cache.
- [runtime/rust/src/value_ops/native_jim_index.rs](../../../../runtime/rust/src/value_ops/native_jim_index.rs), `nested_selected_member_has_actual_hold_before_created_parent_cleanup_and_detached_result` (linked): Software source definition checks nested selected member has actual hold before created parent cleanup and detached result.

A named test is a coverage binding, not a claim that it executed.

## Replay

Whole original inputs, actual commands/receipts, counted public outputs, executable/build/source pins and environment overrides are retained. Other inherited environment/compiler version are unrecorded. Absolute external paths are retained; portable replay is not promised. Re-execution yields new observations. Publication runs no native/compiler/Rust process.
