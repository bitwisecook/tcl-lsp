# naming.lambda.original-parameter-error-frame

Kind: `native-observation`

## Problem statement

Both ports appended the entire lambda value to the C formal-parse frame, although C formats that frame through a CString after parameter validation.

## Question

Does a raw zero in an otherwise valid lambda body truncate the C parsing-lambda errorInfo frame independently of conversion result extent?

## Conclusion

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

## Scope

PARAM_FIELDS and v2 PARAM_BODY_RAW_ZERO under direct counted original vector and counted source. Full return-options bytes retained; claim is only the named errorInfo frame operand and observed error code, not complete errorstack equality, namespace context or successful procedure-body behavior.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Tcl.

Actual invalid command name apply; converter not reached.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

### jim

Status: `not-tested`. Version: 0.84-9-g5bac7c9. Build: Actual reported patchlevel; exact source/header/Makefile/library/probe/executable digests and compile/process0 association retained per capture.. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter.. Dialect: Jim Tcl.

Jim result errors observed, but C errorInfo/return-options API not queried.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation or source implementation claim.

## Historical references

- `v1-probe-c` (input): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/probe.c](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/probe.c). Exact v1 retained probe.c.
- `v1-inputs-json` (input): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/inputs.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/inputs.json). Exact v1 retained inputs.json.
- `v1-capture-py` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/capture.py](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/capture.py). Exact v1 retained capture.py.
- `v1-queue-json` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/queue.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/queue.json). Exact v1 retained queue.json.
- `v1-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/receipt.json). Exact v1 retained receipt.json.
- `v1-tcl8.4-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/receipt.json). Original v1 tcl8.4 receipt.json; process/build association is only that receipt.
- `v1-tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/stdout.tsv). Original v1 tcl8.4 stdout.tsv; process/build association is only that receipt.
- `v1-tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/stderr). Original v1 tcl8.4 stderr; process/build association is only that receipt.
- `v1-tcl8.4-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/compile.stdout). Original v1 tcl8.4 compile.stdout; process/build association is only that receipt.
- `v1-tcl8.4-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.4.20/compile.stderr). Original v1 tcl8.4 compile.stderr; process/build association is only that receipt.
- `v1-tcl8.5-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/receipt.json). Original v1 tcl8.5 receipt.json; process/build association is only that receipt.
- `v1-tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/stdout.tsv). Original v1 tcl8.5 stdout.tsv; process/build association is only that receipt.
- `v1-tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/stderr). Original v1 tcl8.5 stderr; process/build association is only that receipt.
- `v1-tcl8.5-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/compile.stdout). Original v1 tcl8.5 compile.stdout; process/build association is only that receipt.
- `v1-tcl8.5-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.5.19/compile.stderr). Original v1 tcl8.5 compile.stderr; process/build association is only that receipt.
- `v1-tcl8.6-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/receipt.json). Original v1 tcl8.6 receipt.json; process/build association is only that receipt.
- `v1-tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/stdout.tsv). Original v1 tcl8.6 stdout.tsv; process/build association is only that receipt.
- `v1-tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/stderr). Original v1 tcl8.6 stderr; process/build association is only that receipt.
- `v1-tcl8.6-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/compile.stdout). Original v1 tcl8.6 compile.stdout; process/build association is only that receipt.
- `v1-tcl8.6-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/8.6.18/compile.stderr). Original v1 tcl8.6 compile.stderr; process/build association is only that receipt.
- `v1-tcl9.0-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/receipt.json). Original v1 tcl9.0 receipt.json; process/build association is only that receipt.
- `v1-tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/stdout.tsv). Original v1 tcl9.0 stdout.tsv; process/build association is only that receipt.
- `v1-tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/stderr). Original v1 tcl9.0 stderr; process/build association is only that receipt.
- `v1-tcl9.0-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/compile.stdout). Original v1 tcl9.0 compile.stdout; process/build association is only that receipt.
- `v1-tcl9.0-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.0.4/compile.stderr). Original v1 tcl9.0 compile.stderr; process/build association is only that receipt.
- `v1-tcl9.1-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/receipt.json). Original v1 tcl9.1 receipt.json; process/build association is only that receipt.
- `v1-tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/stdout.tsv). Original v1 tcl9.1 stdout.tsv; process/build association is only that receipt.
- `v1-tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/stderr). Original v1 tcl9.1 stderr; process/build association is only that receipt.
- `v1-tcl9.1-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/compile.stdout). Original v1 tcl9.1 compile.stdout; process/build association is only that receipt.
- `v1-tcl9.1-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/9.1.0/compile.stderr). Original v1 tcl9.1 compile.stderr; process/build association is only that receipt.
- `v1-jim-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/receipt.json). Original v1 jim receipt.json; process/build association is only that receipt.
- `v1-jim-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/stdout.tsv). Original v1 jim stdout.tsv; process/build association is only that receipt.
- `v1-jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/stderr). Original v1 jim stderr; process/build association is only that receipt.
- `v1-jim-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/compile.stdout). Original v1 jim compile.stdout; process/build association is only that receipt.
- `v1-jim-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1/jim/compile.stderr). Original v1 jim compile.stderr; process/build association is only that receipt.
- `v2-probe-c` (input): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/probe.c). Exact v2 retained probe.c.
- `v2-inputs-json` (input): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/inputs.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/inputs.json). Exact v2 retained inputs.json.
- `v2-capture-py` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/capture.py](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/capture.py). Exact v2 retained capture.py.
- `v2-queue-json` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/queue.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/queue.json). Exact v2 retained queue.json.
- `v2-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/receipt.json). Exact v2 retained receipt.json.
- `v2-tcl8.4-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/receipt.json). Original v2 tcl8.4 receipt.json; process/build association is only that receipt.
- `v2-tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/stdout.tsv). Original v2 tcl8.4 stdout.tsv; process/build association is only that receipt.
- `v2-tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/stderr). Original v2 tcl8.4 stderr; process/build association is only that receipt.
- `v2-tcl8.4-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/compile.stdout). Original v2 tcl8.4 compile.stdout; process/build association is only that receipt.
- `v2-tcl8.4-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.4.20/compile.stderr). Original v2 tcl8.4 compile.stderr; process/build association is only that receipt.
- `v2-tcl8.5-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/receipt.json). Original v2 tcl8.5 receipt.json; process/build association is only that receipt.
- `v2-tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/stdout.tsv). Original v2 tcl8.5 stdout.tsv; process/build association is only that receipt.
- `v2-tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/stderr). Original v2 tcl8.5 stderr; process/build association is only that receipt.
- `v2-tcl8.5-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/compile.stdout). Original v2 tcl8.5 compile.stdout; process/build association is only that receipt.
- `v2-tcl8.5-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.5.19/compile.stderr). Original v2 tcl8.5 compile.stderr; process/build association is only that receipt.
- `v2-tcl8.6-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/receipt.json). Original v2 tcl8.6 receipt.json; process/build association is only that receipt.
- `v2-tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/stdout.tsv). Original v2 tcl8.6 stdout.tsv; process/build association is only that receipt.
- `v2-tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/stderr). Original v2 tcl8.6 stderr; process/build association is only that receipt.
- `v2-tcl8.6-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/compile.stdout). Original v2 tcl8.6 compile.stdout; process/build association is only that receipt.
- `v2-tcl8.6-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/8.6.18/compile.stderr). Original v2 tcl8.6 compile.stderr; process/build association is only that receipt.
- `v2-tcl9.0-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/receipt.json). Original v2 tcl9.0 receipt.json; process/build association is only that receipt.
- `v2-tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/stdout.tsv). Original v2 tcl9.0 stdout.tsv; process/build association is only that receipt.
- `v2-tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/stderr). Original v2 tcl9.0 stderr; process/build association is only that receipt.
- `v2-tcl9.0-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/compile.stdout). Original v2 tcl9.0 compile.stdout; process/build association is only that receipt.
- `v2-tcl9.0-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.0.4/compile.stderr). Original v2 tcl9.0 compile.stderr; process/build association is only that receipt.
- `v2-tcl9.1-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/receipt.json). Original v2 tcl9.1 receipt.json; process/build association is only that receipt.
- `v2-tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/stdout.tsv). Original v2 tcl9.1 stdout.tsv; process/build association is only that receipt.
- `v2-tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/stderr). Original v2 tcl9.1 stderr; process/build association is only that receipt.
- `v2-tcl9.1-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/compile.stdout). Original v2 tcl9.1 compile.stdout; process/build association is only that receipt.
- `v2-tcl9.1-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/9.1.0/compile.stderr). Original v2 tcl9.1 compile.stderr; process/build association is only that receipt.
- `v2-jim-receipt-json` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/receipt.json). Original v2 jim receipt.json; process/build association is only that receipt.
- `v2-jim-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stdout.tsv). Original v2 jim stdout.tsv; process/build association is only that receipt.
- `v2-jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/stderr). Original v2 jim stderr; process/build association is only that receipt.
- `v2-jim-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/compile.stdout). Original v2 jim compile.stdout; process/build association is only that receipt.
- `v2-jim-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2/jim/compile.stderr). Original v2 jim compile.stderr; process/build association is only that receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::select`: Independently selected exact engine diagnostic recipe.
- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::conversion_error`: Counted C85 versus later C/Jim CString guest wording and error-code action.
- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `NativeLambdaDiagnosticProtocol::parameter_error_frame`: Separate C-only CString formal parse frame.
- [rust/tcl-syntax/src/formal_params/bytes.rs](../../../../rust/tcl-syntax/src/formal_params/bytes.rs), `ByteFormalParameterError::error_code_for_definition`: Selects the structural formal-format error code separately from diagnostic text; C8.6+ sets FORMALARGUMENTFORMAT and C8.4/C8.5/Jim leave the default code.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `parse_params_value`: Reports the selected structural formal error through completed invocation error-code publication.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `parse_params_object`: Reports the selected structural formal error through completed invocation error-code publication.
- [rust/tcl-vm/src/command/native_lambda_tests.rs](../../../../rust/tcl-vm/src/command/native_lambda_tests.rs), `command::native_lambda_tests::lambda_conversion_errors_match_original_counted_values_and_parameter_frames` (linked): 60 direct original bad-lambda code/result comparisons,48 C errorcodes and4 CString C parameter frames; successful activation, whole options/cache/header and arbitrary getter hooks remain independent.
- [runtime/rust/src/cmd_proc/native_lambda_tests.rs](../../../../runtime/rust/src/cmd_proc/native_lambda_tests.rs), `cmd_proc::native_lambda_tests::lambda_conversion_errors_match_original_counted_values_and_parameter_frames` (linked): 60 direct original bad-lambda code/result comparisons,48 C errorcodes and4 CString C parameter frames; successful activation, whole options/cache/header and arbitrary getter hooks remain independent.
- [rust/tcl-registry/src/native_lambda.rs](../../../../rust/tcl-registry/src/native_lambda.rs), `native_lambda::tests::lambda_diagnostics_keep_conversion_and_parameter_frame_purposes_separate` (linked): Pure selected extents/error code/frame recipe and unknown/mismatched engine/operational-refusal negatives; actual original-object/activation remains independent.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3 rust/tcl-registry/tests/data/native_lambda_diagnostics/replay.py --verify-only"
]
```

Verify-only checks retained exact receipt/stream/input hashes and named fields; zero native/Rust launches. Native replay requires exact original configured provider inputs recorded in each immutable queue. Source windows reproduce exact LF range/full-source/snippet association. No unmeasured type/cache/body claim.
