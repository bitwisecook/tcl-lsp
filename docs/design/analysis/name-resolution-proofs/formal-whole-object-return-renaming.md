# naming.formal.whole-object-return-renaming

Kind: `native-observation`

## Problem statement

A sole formal and a whole-word return may look safe to rename, but equal displayed text does not establish the same result object. Formal/body parsing also has independent byte purposes. The comparison retains one original argument object across both calls and samples result identity before a getter; only the listed ordinary native String inputs and formal spellings are suitable for this observation.

## Question

Do the tested non-raw-zero required formal spellings and their renamed a formal return the same retained original argument object and bytes?

## Conclusion

All six tested interpreters return completion0, the identical retained original argument object and exact counted bytes through both forms for ASCII, genuine UTF8 E9/supplementary, encoded C080, raw ff and D800 formal spellings, across six fresh ordinary String arguments. This supplies finite whole-object return observations; it does not prove arbitrary alpha equivalence, unknown object callback/release behavior, frame elimination, source compiler admission or Rust execution.

## Scope

Public counted original object-vector proc declarations, formal-list/body operands and calls. Body original is return ${<exact formal bytes>}; renamed body is return $a. Excludes raw-zero formal rows and introspection/trace rows, and excludes custom or initially non-String argument classes.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 baef50e823969f7b9bd582b6e33358668bf508e81c581b429bf32f15d86802cb; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 ecc119b27915973f32491ca1a08a6e05ff61105255288c7dcabe40f7767124f0; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 91d13b3ab90255989bdb46a0385c9a58a1c196a39fd22175cc61891c93284009; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 2d033022db43c1a077974f0e070587f567233ff86e95ba403414ae1cd7ef9a1b; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 a65fe0c770c5b471db5bee111c540f191414c55fa9091e5464df2250fb044893; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 9a0cfce6330d3a6377dcf1a0e82b85b0d86d329294b763dd5f461f5ea95e9b6f; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Jim Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact counted formal/body/argv question.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_formal_alpha/probe.c](../../../../rust/tcl-vm/tests/data/native_formal_alpha/probe.c). SHA-256 `c1eff69c678c69913d5599e5af840a2721f93ada5c3d79119cbfd5876e348633`. Exact public counted declarations/body/argv arrays and pointer equality sampled before the sole result string getter.
- `aggregate` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/receipt.json). SHA-256 `ff9718c43e604e3f9bb289dfd3b8d490eaf331387183a5df233018e3eef4118e`. Complete actual all-six native compile/process/header/library/Makefile/source-owner/executable/stream correspondence.
- `tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/receipt.json). SHA-256 `ceba9c94af9b57237a60fde69d5b4e243dca2cd856d58afc39c2a69533beaa8d`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv). SHA-256 `b2dc4528ee762ef4e6c23f937afc23192d444ba9337797c6799dda533c79c6b4`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/receipt.json). SHA-256 `7c07926f9d1e6ee803c9b2e6ca38351540edd18c5dba18d1c7d22958fc7695d4`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv). SHA-256 `4d4602282c693cf4a95957b7259a8036dd4191e11291ce31ed65d005b9d19ab5`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/receipt.json). SHA-256 `e83b27266b9efa3fd281954f4608ddfe7a535990f00a33352403226a3fa0e72e`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv). SHA-256 `13c593a7750ebeb29631705b29f427108c4625b47a1b72e1c013b331e327b1e4`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/receipt.json). SHA-256 `f58eaccae4567ed87e07ef4522a7125e7e36ad1ef9788e4f54b1cfe17dfb6464`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv). SHA-256 `13708a54e3492cd82b656684b8b083dc07d4a50933ff59ceb65a5532de3bbecd`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/receipt.json). SHA-256 `c4efe548d02e67d47adee7250157b4b6c83bc5c3de2132fd8c2c12c1c9bc0d3d`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv). SHA-256 `d39c4ba5c042d38bbaafc79657048aca782b7558d5241f88b8379f1c235f3bdf`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/receipt.json). SHA-256 `ee1fc5cb1033755ee7165d03c1a214e42a53053df2f169a30daeac43371be232`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stdout.tsv). SHA-256 `123909ffc3425489e4b9f13e8ab65cb15cb58843d76d22f8ed38def95c56510a`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `v1-capture-limitation` (limitation): [rust/tcl-vm/tests/data/native_formal_alpha/v1/attempt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v1/attempt.json). SHA-256 `2e8b4edc8e81ddb1acbfdfad1962af91ed80e0321e6069ae8344b1d2efafe23f`. First capture runner decoded the pointer field as a version and failed before a provider receipt; only the exact first C84 raw stream and traceback are retained, not a complete build/status association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/command/native_formal_alpha_tests.rs](../../../../rust/tcl-vm/src/command/native_formal_alpha_tests.rs), `command::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares original counted proc/formal/body/value operands, guest codes, original result identity before getter and bytes for the six independently selected native VM contexts.
- [runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs](../../../../runtime/rust/src/cmd_proc/native_formal_alpha_tests.rs), `cmd_proc::native_formal_alpha_tests::counted_formal_renaming_keeps_whole_object_return_and_binding_boundaries` (linked): Compares the exact original proc/formal/body/value operands and retained result pointer/bytes in the separately selected Runtime interpreter contexts. No execution result is inferred from this binding.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_formal_alpha/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-formal-alpha-reconfirmation"
]
```

Requires the exact retained public header/static library/Makefile/source owner/probe hashes. Rebuild and compare complete process exit/stdout/stderr, including all guest errors and identity rows. --verify-only validates inputs without launches. v1 adapter failure is retained separately and has no provider receipt. No minifier/Compiler/Runtime execution is inferred.
