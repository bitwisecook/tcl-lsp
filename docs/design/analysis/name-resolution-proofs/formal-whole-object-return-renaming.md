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

Status: `observed`. Version: 8.4.20. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Jim Tcl.

Both forms return the same held argv object before any result getter and preserve all six exact counted String argument byte sequences for the six non-raw-zero formal arrangements.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact counted formal/body/argv question.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_formal_alpha/probe.c](../../../../rust/tcl-vm/tests/data/native_formal_alpha/probe.c). Exact public counted declarations/body/argv arrays and pointer equality sampled before the sole result string getter.
- `aggregate` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/receipt.json). Complete actual all-six native compile/process/header/library/Makefile/source-owner/executable/stream correspondence.
- `tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.4.20/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.5.19/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/8.6.18/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.0.4/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/9.1.0/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/receipt.json). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stdout.tsv). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v2/jim/stderr). Actual retained reported version/build and native completion/identity/hex rows; guest errors are separate from successful process0 and empty stderr.
- `v1-capture-limitation` (limitation): [rust/tcl-vm/tests/data/native_formal_alpha/v1/attempt.json](../../../../rust/tcl-vm/tests/data/native_formal_alpha/v1/attempt.json). First capture runner decoded the pointer field as a version and failed before a provider receipt; only the exact first C84 raw stream and traceback are retained, not a complete build/status association.

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
