# naming.variable.catch-original-output-name

Kind: `native-observation`

## Problem statement

A catch output name can be opaque native bytes even when its display is invalid or contains zero. Reconstructing it from a display can write the replacement-character name or clip a counted operand. The output result, plain-name sentinel and replacement-character sentinel must be checked separately, and the old C8.4 setter extent cannot be generalized to later engines.

## Question

Where does public catch store BOOM when its original output name contains ff, binary zero or encoded C080, and do plain or replacement-character sibling cells change?

## Conclusion

All six providers return catch status 1 and reading the exact original output name returns BOOM. The replacement-character sibling remains REPLACEMENT. Raw ff and C080 preserve the plain r sentinel on all six. With binary zero, only C8.4 changes plain r to BOOM; C8.5/C8.6/C9.0/C9.1/Jim keep plain r as PLAIN. These are direct output-setter/result observations, not alias, trace, normal-completion, header/cache or complete namespace laws.

## Scope

Each fresh interpreter starts with r=PLAIN and r plus U+FFFD UTF8 bytes=REPLACEMENT. Public catch receives original counted r+ff, r+00+tail or r+C080+tail and ASCII error BOOM body. Each result is inspected before the next command replaces it. No optionVarName, callbacks, aliases, array names or missing namespace is tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked executable SHA-256 dec5a1145666fc09a8dbc88d4e578a8217c0cac944609e98437d2e5f07afacea. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output also changes plain r to BOOM. Raw ff and C080 keep the plain sentinel.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked executable SHA-256 ea84d60c6082d53500b319e7b3332e66abeb320c6f8f3c148af8ffdde7c2c9ba. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked executable SHA-256 d5d21b3296d1dba64a270e49ba8a8a4e891782b3ee707702764975edbd90439e. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked executable SHA-256 2f739dd46af4d5d49e65dfa02b56075bf0ddb2457b5a085f5ab0bd12f41ad219. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked executable SHA-256 8a0c3677a2f31d3250cf000fb647b03ee67adbc84d06f0d6894224ea42aeb7bf. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked executable SHA-256 0517736ce44f158e8dd505ee9c8ce5f592942800aa34f549a54d32fe2e757019. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Jim_EvalObjVector; ASCII Jim_Eval setup/version source.. Dialect: Jim Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for these exact inputs.

## Exact evidence

- `v1-input` (input): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/probe.c](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/probe.c). SHA-256 `def9f407e74ef9689014f3550e35257edb404abfa04d5ba1b1c0f2e319a382b7`. Complete exact C/Jim probe source. Counted object construction and each immediate result getter are explicit. Separate variants are not declared identical.
- `v1-aggregate` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/receipt.json). SHA-256 `6012feb436d11dc1088cedb0e1350ead9d50c00df103561ee7271acc03edff7e`. Exact complete original six-provider receipts for this probe variant.
- `v2-input` (input): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/probe.c). SHA-256 `6a2c8b55b40958f334bf66e3b293730e8704bff7df902a4cabe776b78f442a3f`. Complete exact C/Jim probe source. Counted object construction and each immediate result getter are explicit. Separate variants are not declared identical.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/receipt.json). SHA-256 `aa7b5bc815fa85e29962ae5882ae48a7861e4a4e1b4ab0a09ef1f6fb242b0780`. Exact complete original six-provider receipts for this probe variant.
- `v1-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/receipt.json). SHA-256 `ef276e274763c15efecc49884ba5eca2ba1e361cfeda272a52d7b68565071622`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stdout.tsv). SHA-256 `7ddd5e4a8f4fc06e99be84de33e5ff9d2af6df3feea64dd8c3d16d88b3f7ff68`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/receipt.json). SHA-256 `3f1bd5a8ec5a9e37fc70fe89403d1da5c6b7e2dd6dd62eff749495c386ec076e`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv). SHA-256 `1309e1d28935e960ddd6667152e206506c3fe66a19512670309d7976a9ad14c5`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/receipt.json). SHA-256 `6ca2955791750bc37683e11780d04cd70a8f567017e38a86a66d31634ec58c40`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stdout.tsv). SHA-256 `4d47b418bcf52be70de56eaf7e6a70dd7dbe4c49a2407fdf136218edfe62619b`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/receipt.json). SHA-256 `fd3c2e3623354f42bfa272c2dfa192b13c4eee4bf2643b1b15efb75c8ef20c5c`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv). SHA-256 `18358d1693a09ada011ac9677909c18e8cbc7d2c08ba912dfc8c7e7be4cf9dc3`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/receipt.json). SHA-256 `a4c55f6d2446b06ca04f2b809db1cb4bcd8d87a544494a3ad76739df1ae49dfa`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stdout.tsv). SHA-256 `184feaec9a1347f271c82ad060dae0d0602950bbc3228fac876d708765b80298`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/receipt.json). SHA-256 `9a247eabf27620bf559f3de4a3615404a4b74736af47d31319d7855dc7cae463`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv). SHA-256 `d81d6cb52119f07bdb3121ae161116e8186c99f075d9396f1a1bec4f719c2ce3`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/receipt.json). SHA-256 `3c539e6a31d695c0362dacadfeec3b050c8db4dd531ae4452f02b5d17a774016`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stdout.tsv). SHA-256 `89f93f3ad10b6331e59a319da91d766a1d0fd76a38efbdd707a89ba7a10b55d1`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/receipt.json). SHA-256 `3f393680a5def423599c871e4c0f4c7ada977f9bfd5fc0db1776fb5a138e76d7`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv). SHA-256 `4c16e88a975c0e70e40685ab467467448686fa9967a93ac6d016e0743546be44`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/receipt.json). SHA-256 `16fed58162b475e547354abd05cec9267be2dffaed12b0b7d5b936017b76bd63`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stdout.tsv). SHA-256 `138b57c23a14618e6e24aa55f95fd58a5d0d64ab15ce94606aa20dada7f9ba01`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/receipt.json). SHA-256 `546bab8bb33d3e3f1c8b6ac4e66a93cb30471f99e29c01aaef6e92091431e4fb`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv). SHA-256 `6266205ad6ec43da4adaa09a5ca44c8bc1e3bfea0f786c9b31c1a38e5bfddc88`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/receipt.json). SHA-256 `410a8227ccc4dd66e4943dc6492912f311af4164bc9e030c0aa7ce098ef5e2c3`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stdout.tsv). SHA-256 `40e04bf45bbad35786eeabca6fc6e72614f84215dccd96e9cf43e4cd02b0c566`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/receipt.json). SHA-256 `d7e7513c64b615f37f7a78e12cb16d811452e71ba4cf83de0ca29d0d6bbbfbf7`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stdout.tsv). SHA-256 `14d5d232efa465b7fc710c433ebbbd1371f1f090deef429343665bc2f34a11d9`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `finish_catch`: Store through the original result-variable object using the selected native setter purpose.
- [rust/tcl-vm/src/command/native_subst_operand_tests.rs](../../../../rust/tcl-vm/src/command/native_subst_operand_tests.rs), `command::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.
- [runtime/rust/src/builtins/native_subst_operand_tests.rs](../../../../runtime/rust/src/builtins/native_subst_operand_tests.rs), `builtins::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_subst_catch_operands/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-subst-catch-reconfirmation"
]
```

Requires exact pinned probe/public header/static library/Makefile/source-owner and retained stream hashes. Recompiles both independent variants and compares complete exit/stdout/stderr; guest result errors are expected. --verify-only checks correspondence without any native or Rust execution. This question has no cache/header/errorCode windows.
