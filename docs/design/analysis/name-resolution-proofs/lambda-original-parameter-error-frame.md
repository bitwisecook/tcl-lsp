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

Status: `unsupported`. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter. Version: tcl8.4 8.4.20.

Actual invalid command name apply; converter not reached.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

C8.5.19/8.6.18/9.0.4/9.1.0 fail invalid three-field formals and append a parsing-lambda errorInfo frame ending at raw zero inside the valid body. This is independent of C8.5 counted conversion diagnostics. C8.5 error code is NONE; C8.6/C9 use TCL OPERATION PROC FORMALARGUMENTFORMAT. Jim has the same measured result error but its C return options/errorInfo are not queried. C8.4 apply is unavailable.

### jim

Status: `not-tested`. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter. Version: jim 0.84-9-g5bac7c9.

Jim result errors observed, but C errorInfo/return-options API not queried.

### bigip

Status: `not-tested`.

No appliance observation or source implementation claim.

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

## Evidence

- [rust/tcl-registry/tests/data/native_lambda_diagnostics/v1](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v1)
- [rust/tcl-registry/tests/data/native_lambda_diagnostics/v2](../../../../rust/tcl-registry/tests/data/native_lambda_diagnostics/v2)
