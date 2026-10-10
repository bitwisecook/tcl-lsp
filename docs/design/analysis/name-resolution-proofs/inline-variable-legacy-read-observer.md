# naming.refactor.variable-inline-legacy-read-observer

Kind: `native-observation`

## Problem statement

Legacy trace variable syntax has a release boundary; rejection must not be interpreted as a quiet successful variable read.

## Question

Which measured shells reach the legacy read observer and how does the literal replacement change that observation?

## Conclusion

C8.4 through C8.6 emit OBSERVED then 1 for the legacy traced read and only 1 for the literal replacement. C9.0/C9.1 report the retired variable option unavailable, and Jim reports trace unavailable. Unsupported registrations are retained as guest results and do not supply a quiet-read claim.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executed shell SHA-256 b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file legacy_watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. file legacy_watched_naive: exit 0, stdout '1\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executed shell SHA-256 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file legacy_watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. file legacy_watched_naive: exit 0, stdout '1\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executed shell SHA-256 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file legacy_watched_original: exit 0, stdout 'OBSERVED\n1\n', stderr ''. file legacy_watched_naive: exit 0, stdout '1\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.0

Status: `unsupported`. Version: 9.0.4. Build: Executed shell SHA-256 f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file legacy_watched_original: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. file legacy_watched_naive: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.1

Status: `unsupported`. Version: 9.1.0. Build: Executed shell SHA-256 c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file legacy_watched_original: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. file legacy_watched_naive: exit 0, stdout 'UNAVAILABLE\nbad option "variable": must be add, info, or remove\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Executed shell SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Jim Tcl.

file legacy_watched_original: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. file legacy_watched_naive: exit 0, stdout 'UNAVAILABLE\ninvalid command name "trace"\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP appliance result is attached for this precise native source question.

## Exact evidence

- `c84-file-8` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/8`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c84-file-8-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `c84-file-8-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.stdout). SHA-256 `58f2ed159e2b6afb60f5f5fe570e825f1ff9f9755099ec269313b3f6c5c429e7`. Unmodified exact original stdout bytes for the selected control.
- `c84-file-8-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c84-file-9` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/9`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c84-file-9-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `c84-file-9-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.stdout). SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`. Unmodified exact original stdout bytes for the selected control.
- `c84-file-9-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c85-file-20` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/20`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c85-file-20-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `c85-file-20-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.stdout). SHA-256 `58f2ed159e2b6afb60f5f5fe570e825f1ff9f9755099ec269313b3f6c5c429e7`. Unmodified exact original stdout bytes for the selected control.
- `c85-file-20-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c85-file-21` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/21`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c85-file-21-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `c85-file-21-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.stdout). SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`. Unmodified exact original stdout bytes for the selected control.
- `c85-file-21-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c86-file-32` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/32`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c86-file-32-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `c86-file-32-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.stdout). SHA-256 `58f2ed159e2b6afb60f5f5fe570e825f1ff9f9755099ec269313b3f6c5c429e7`. Unmodified exact original stdout bytes for the selected control.
- `c86-file-32-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c86-file-33` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/33`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c86-file-33-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `c86-file-33-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.stdout). SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`. Unmodified exact original stdout bytes for the selected control.
- `c86-file-33-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c90-file-44` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/44`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c90-file-44-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `c90-file-44-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.stdout). SHA-256 `8725912190ecfd31e3831bd0dee9c86dfeedbaaa9a6974ce62878e0db2220f53`. Unmodified exact original stdout bytes for the selected control.
- `c90-file-44-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c90-file-45` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/45`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c90-file-45-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `c90-file-45-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.stdout). SHA-256 `8725912190ecfd31e3831bd0dee9c86dfeedbaaa9a6974ce62878e0db2220f53`. Unmodified exact original stdout bytes for the selected control.
- `c90-file-45-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c91-file-56` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/56`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c91-file-56-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `c91-file-56-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.stdout). SHA-256 `8725912190ecfd31e3831bd0dee9c86dfeedbaaa9a6974ce62878e0db2220f53`. Unmodified exact original stdout bytes for the selected control.
- `c91-file-56-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `c91-file-57` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/57`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c91-file-57-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `c91-file-57-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.stdout). SHA-256 `8725912190ecfd31e3831bd0dee9c86dfeedbaaa9a6974ce62878e0db2220f53`. Unmodified exact original stdout bytes for the selected control.
- `c91-file-57-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `jim-file-68` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/68`. Exact original provider/source/status and raw-stream correspondence for this question.
- `jim-file-68-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.tcl). SHA-256 `d835ecfc7833c712df0c5b0ef839d26a244e0725222db32fb18bd7c386bed80b`. Unmodified exact original source bytes for the selected control.
- `jim-file-68-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.stdout). SHA-256 `7db4c1d237d4f87b1c3a9f9414d27127b96aadf1ec3699346987f78d4bfffca5`. Unmodified exact original stdout bytes for the selected control.
- `jim-file-68-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_original.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `jim-file-69` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). SHA-256 `bcce6142a4576df80edb92cd734285b33508e50f9d70354bef28a9faadca845f`. JSON pointer `/records/69`. Exact original provider/source/status and raw-stream correspondence for this question.
- `jim-file-69-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.tcl). SHA-256 `bd37347444cd903650681b7216c109c0ab65c4bbeb17dd02db062434d444a61b`. Unmodified exact original source bytes for the selected control.
- `jim-file-69-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.stdout). SHA-256 `7db4c1d237d4f87b1c3a9f9414d27127b96aadf1ec3699346987f78d4bfffca5`. Unmodified exact original stdout bytes for the selected control.
- `jim-file-69-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-legacy_watched_naive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unmodified exact original stderr bytes for the selected control.
- `maintained-replay` (implementation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py). SHA-256 `e77fdb5229f8dd36fe879a5941d1b790f34de2f85ceeb527d8c2cb1f10ba1203`. Maintained exact-source/stream verifier and explicit selected-build replay; its source is not an execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement` (linked): Keep observer-sensitive replacement refused, independently of unsupported release registration grammar. This source consumer assertion is independent of the recorded native shell outcomes.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py",
  "--verify-only",
  "--variant",
  "both"
]
```

Verify-only checks all exact retained source/stream associations without launching native shells. Actual replay requires all six explicitly selected executables with their original SHA and executed version, preserves file versus stdin channels, and adapts only the exact source filename in file-error stderr. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.
