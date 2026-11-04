# naming.formal.rename-observer-name-differences

Kind: `native-observation`

## Problem statement

Renaming a formal changes name-observing operations even when a whole-word return still yields the same argument object. A transform that considers only returned bytes can miss info locals, info args or trace callback subjects. These explicit observer controls delimit the simple return comparison; they do not authorize a transform in any body with introspection or observers.

## Question

Which tested local/formal introspection and variable read-trace outputs differ after longargument is renamed a?

## Conclusion

All six report longargument versus a from info locals and info args. All five C interpreters still return the same held VALUE object in the read-traced return bodies, but callbacks report longargument versus a. Jim's actual trace command query is empty, so its trace-body question is unsupported. Observed whole-object return identity does not erase these naming differences.

## Scope

ASCII original object-vector declarations and same VALUE argv; independent info args source query. C8.4 trace variable ... r and C8.5+ trace add variable ... read are separate actual source forms. Jim only queries trace availability, with no callback execution. No arbitrary external observer, execution trace or rename implementation is proved.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Tcl.

info locals/info args and read-trace callback subjects report longargument versus a, while traced returns retain the original VALUE argument object.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable; compile command, public-header/static-library/Makefile/source-owner hashes and process streams are retained in the provider receipt.. Channel: Original counted Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector declarations/formals/body/argv; independent ASCII NUL-terminated Tcl_Eval/Jim_Eval queries.. Dialect: Jim Tcl.

info locals and info args report longargument versus a. Trace command query is empty; no trace callback executes.

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

- [rust/tcl-vm/src/command/native_formal_alpha_tests.rs](../../../../rust/tcl-vm/src/command/native_formal_alpha_tests.rs), `command::native_formal_alpha_tests::formal_renaming_observer_controls_report_the_distinct_names` (linked): Compares actual local/formal introspection and C trace-name controls, with independent Jim trace absence; this does not claim arbitrary observer equivalence.

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
