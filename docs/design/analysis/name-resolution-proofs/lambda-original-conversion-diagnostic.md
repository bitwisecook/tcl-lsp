# naming.lambda.original-conversion-diagnostic

Kind: `native-observation`

## Problem statement

VM bad-arity lambda formatting reconstructed display text and both ports exposed list-parser wording, losing the original raw-zero release boundary.

## Question

What result and error-code bytes do original counted lambda values produce on conversion failure?

## Conclusion

C8.5.19 conversion diagnostics retain full raw-zero bytes; C8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 format the CString operand. Encoded C080, FF and D800 bytes remain distinct. Malformed outer lists are reported as lambda conversion failures. C8.6/C9 conversion error code is TCL VALUE LAMBDA, C8.5 fresh failures report NONE; Jim C return options are not queried. C8.4 reports apply unavailable.

## Scope

13 values v1 and 14 v2, each direct original counted string-object vector and counted source. Includes separate valid controls and formal-stage failures; 54 operation attempts/provider across immutable variants, 324 total. No Document/read ingress, arbitrary object hooks, representation/cache/header ownership, body/compiler/namespace/activation or completion-equivalence grant.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter. Version: tcl8.4 8.4.20.

Actual invalid command name apply; converter not reached.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Direct original counted string-object vector via Tcl_EvalObjv/Jim_EvalObjVector, and exact counted quoted source via Tcl_EvalEx/Jim_EvalObj. Public counted result getters after completion; C8.5+ return-options getter. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

C8.5.19 conversion diagnostics retain full raw-zero bytes; C8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 format the CString operand. Encoded C080, FF and D800 bytes remain distinct. Malformed outer lists are reported as lambda conversion failures. C8.6/C9 conversion error code is TCL VALUE LAMBDA, C8.5 fresh failures report NONE; Jim C return options are not queried. C8.4 reports apply unavailable.

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
