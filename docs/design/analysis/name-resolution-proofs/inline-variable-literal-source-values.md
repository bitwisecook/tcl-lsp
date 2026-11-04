# naming.refactor.variable-inline-literal-source-values

Kind: `native-observation`

## Problem statement

A literal-variable rewrite needs the same actually parsed read and literal output; equal reporting labels do not establish that source relationship.

## Question

Do the pure ASCII and braced Unicode original reads return the same value as the exact literal replacement control?

## Conclusion

All six measured shells return 7 with process0 and empty stderr for ASCII $x, braced ${café}, and the literal puts 7 control. These finite pure scripts establish value equality for those exact sources, independently of a rewrite permission for other cells, values or observers.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executed shell; configure/compiler/library metadata not recorded for this experiment.. Channel: UTF-8 file script passed as the public shell argv filename. Dialect: Jim Tcl.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP appliance result is attached for this precise native source question.

## Historical references

- `c84-file-0` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/0`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c84-file-0-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `c84-file-0-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c84-file-0-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c84-file-2` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/2`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c84-file-2-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `c84-file-2-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c84-file-2-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c84-file-3` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/3`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c84-file-3-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `c84-file-3-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `c84-file-3-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c84-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `c85-file-12` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/12`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c85-file-12-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `c85-file-12-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c85-file-12-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c85-file-14` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/14`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c85-file-14-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `c85-file-14-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c85-file-14-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c85-file-15` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/15`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c85-file-15-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `c85-file-15-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `c85-file-15-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c85-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `c86-file-24` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/24`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c86-file-24-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `c86-file-24-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c86-file-24-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c86-file-26` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/26`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c86-file-26-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `c86-file-26-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c86-file-26-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c86-file-27` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/27`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c86-file-27-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `c86-file-27-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `c86-file-27-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c86-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `c90-file-36` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/36`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c90-file-36-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `c90-file-36-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c90-file-36-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c90-file-38` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/38`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c90-file-38-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `c90-file-38-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c90-file-38-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c90-file-39` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/39`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c90-file-39-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `c90-file-39-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `c90-file-39-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c90-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `c91-file-48` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/48`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c91-file-48-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `c91-file-48-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c91-file-48-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c91-file-50` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/50`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c91-file-50-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `c91-file-50-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `c91-file-50-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `c91-file-51` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/51`. Exact original provider/source/status and raw-stream correspondence for this question.
- `c91-file-51-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `c91-file-51-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `c91-file-51-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/c91-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `jim-file-60` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/60`. Exact original provider/source/status and raw-stream correspondence for this question.
- `jim-file-60-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.tcl). Unmodified exact original source bytes for the selected control.
- `jim-file-60-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `jim-file-60-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_ascii_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `jim-file-62` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/62`. Exact original provider/source/status and raw-stream correspondence for this question.
- `jim-file-62-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.tcl). Unmodified exact original source bytes for the selected control.
- `jim-file-62-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.stdout). Unmodified exact original stdout bytes for the selected control.
- `jim-file-62-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_unicode_braced_original.stderr). Unmodified exact original stderr bytes for the selected control.
- `jim-file-63` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/receipts.json). JSON pointer `/records/63`. Exact original provider/source/status and raw-stream correspondence for this question.
- `jim-file-63-source` (input): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.tcl](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.tcl). Unmodified exact original source bytes for the selected control.
- `jim-file-63-stdout` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.stdout](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.stdout). Unmodified exact original stdout bytes for the selected control.
- `jim-file-63-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.stderr](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file/jim-pure_inlined.stderr). Unmodified exact original stderr bytes for the selected control.
- `maintained-replay` (implementation): [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py). Maintained exact-source/stream verifier and explicit selected-build replay; its source is not an execution receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_requires_exact_definition_and_unobserved_read_without_ui_maps` (linked): Select the exact braced Unicode read and original literal store without reporting variables; refuse stale source. This source consumer assertion is independent of the recorded native shell outcomes.

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
