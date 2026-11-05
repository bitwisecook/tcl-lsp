# naming.variable.original-upvar-error-publication

Kind: `native-observation`

## Problem statement

An upvar rejection needs its actual original guest result and errorCode. The selected provider may allow a local alias-element-looking spelling or reject the trace registration itself; those completions cannot be collapsed into a universal upvar failure.

## Question

What exact caught result/errorCode do the original inverted, local-element, self, existing and traced upvar controls publish in each provider, and does C8.4 permit alias(k) as the local key?

## Conclusion

C8.4.20 permits the local_element source with catch0/result0 and unchanged SENTINEL errorCode; its other four original failures publish NONE. C8.5.19 rejects all five original upvar controls with NONE. C8.6.18 rejects all five with TCL UPVAR INVERTED/LOCAL_ELEMENT/SELF/EXISTS/TRACED respectively. C9.0.4/C9.1.0 publish those first four UPVAR causes; their fifth control fails earlier at original legacy trace variable syntax with TCL LOOKUP INDEX option variable, so it observes no traced-upvar rejection. Jim reports the original inverted/local_element/self/existing failures with NONE and its fifth control fails at missing trace with NONE; its self result is variable x already exists. All six external processes and six driver compilations exit0 with empty stderr. Whole five-row caught result/errorCode values retain exact release-specific wording. A successful operation may leave the separately seeded SENTINEL unchanged. These finite public fields grant no physical variable/object/header identity, alias-cell/store/frame guarantee, compiled opcode or error-trace behaviour. BIG-IP and Rust assertions are not tested. VM original C alias diagnostics also retain the typed UpvarLinkError cause separately from rendered operand bytes. present_original_c_alias_error selects Set versus Default from the actual C name protocol and keeps namespace lookup failures separate; compiled linking passes the same original target/local operands and actual cause. This linked source implementation does not observe a native variable cell or strengthen the five captured public error rows. Shared NativeJimAliasFailure and alias_failure_message retain authentic Jim local-defined/inversion/self rejection causes before presentation. Runtime and VM binders consume that same actual cause instead of classifying result strings. Whole original six-provider upvar source controls retain the C9 missing legacy trace API row separately; implementation linkage supplies no additional native alias-cell identity.

## Scope

The exact original ASCII LF source defines p independently for each original body, seeds ::errorCode SENTINEL, catches p, retains its original result and errorCode in a list row, and deletes p before the next body. The unchanged counted C API driver adds no catch/eval/puts envelope; full Tcl_Init or Jim core/static initialisation and selected original source are separately pinned. The traced source executes its own original trace registration inside p, so missing or incompatible registration is its measured row. The final driver ORIGINAL hex is the complete five-row public result; no arbitrary selected row or byte normalisation replaces it.

## Provider answers

### tcl8.4

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20.

Exact ORIGINAL code0 complete public result: {inverted 1 {bad variable name "::alias": upvar won't create namespace variable that refers to procedure variable} NONE} {local_element 0 0 SENTINEL} {self 1 {can't upvar from variable to itself} NONE} {existing 1 {variable "alias" already exists} NONE} {traced 1 {variable "alias" has traces: can't use for upvar} NONE}. Process exit0/empty stderr. The trace row retains its actually reached registration or upvar failure; whole original streams and pins preserved.

### tcl8.5

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.5 8.5.19.

Exact ORIGINAL code0 complete public result: {inverted 1 {bad variable name "::alias": can't create namespace variable that refers to procedure variable} NONE} {local_element 1 {bad variable name "alias(k)": can't create a scalar variable that looks like an array element} NONE} {self 1 {can't upvar from variable to itself} NONE} {existing 1 {variable "alias" already exists} NONE} {traced 1 {variable "alias" has traces: can't use for upvar} NONE}. Process exit0/empty stderr. The trace row retains its actually reached registration or upvar failure; whole original streams and pins preserved.

### tcl8.6

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.6 8.6.18.

Exact ORIGINAL code0 complete public result: {inverted 1 {bad variable name "::alias": can't create namespace variable that refers to procedure variable} {TCL UPVAR INVERTED}} {local_element 1 {bad variable name "alias(k)": can't create a scalar variable that looks like an array element} {TCL UPVAR LOCAL_ELEMENT}} {self 1 {can't upvar from variable to itself} {TCL UPVAR SELF}} {existing 1 {variable "alias" already exists} {TCL UPVAR EXISTS}} {traced 1 {variable "alias" has traces: can't use for upvar} {TCL UPVAR TRACED}}. Process exit0/empty stderr. The trace row retains its actually reached registration or upvar failure; whole original streams and pins preserved.

### tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact ORIGINAL code0 complete public result: {inverted 1 {bad variable name "::alias": can't create namespace variable that refers to procedure variable} {TCL UPVAR INVERTED}} {local_element 1 {bad variable name "alias(k)": can't create a scalar variable that looks like an array element} {TCL UPVAR LOCAL_ELEMENT}} {self 1 {can't upvar from variable to itself} {TCL UPVAR SELF}} {existing 1 {variable "alias" already exists} {TCL UPVAR EXISTS}} {traced 1 {bad option "variable": must be add, info, or remove} {TCL LOOKUP INDEX option variable}}. Process exit0/empty stderr. The trace row retains its actually reached registration or upvar failure; whole original streams and pins preserved.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Exact ORIGINAL code0 complete public result: {inverted 1 {bad variable name "::alias": upvar won't create namespace variable that refers to procedure variable} NONE} {local_element 1 {bad variable name "alias(k)": upvar won't create a scalar variable that looks like an array element} NONE} {self 1 {variable "x" already exists} NONE} {existing 1 {variable "alias" already exists} NONE} {traced 1 {invalid command name "trace"} NONE}. Process exit0/empty stderr. The trace row retains its actually reached registration or upvar failure; whole original streams and pins preserved.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `present_original_c_alias_error`: Present the actual typed C alias failure using the selected protocol rather than classifying message text; source implementation only.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `link_original_c_compiled_upvar`: Retain original operands and actual link-error cause before shared typed presentation; no additional native cell identity observation.
- [rust/tcl-syntax/src/native_jim_lookup.rs](../../../../rust/tcl-syntax/src/native_jim_lookup.rs), `NativeJimAliasFailure`: Keep actual Jim alias rejection cause independently typed before rendering original operand units.
- [rust/tcl-syntax/src/native_jim_lookup.rs](../../../../rust/tcl-syntax/src/native_jim_lookup.rs), `alias_failure_message`: Render only the genuine selected Jim alias failure and original units; no C rejection classification from text.
- [runtime/rust/src/cmd_var.rs](../../../../runtime/rust/src/cmd_var.rs), `cmd_var::tests::original_upvar_error_publication_matches_all_six_native_sources` (linked): Compare complete unchanged original five-row source/catch/result/errorCode controls for all six provider APIs; C9 missing legacy trace stays independent from upvar rejection.
- [rust/tcl-vm/src/interp/native_jim_links.rs](../../../../rust/tcl-vm/src/interp/native_jim_links.rs), `interp::native_jim_links::tests::original_upvar_error_publication_matches_all_six_native_sources` (linked): VM compares the same untouched whole original six-provider source controls using typed actual alias causes, without cell/header/frame inference.

## Evidence

- [rust/tcl-registry/tests/data/native_upvar_error_publication307](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/8.4.20/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/8.4.20/original-upvar-errors)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/8.5.19/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/8.5.19/original-upvar-errors)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/8.6.18/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/8.6.18/original-upvar-errors)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/9.0.4/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/9.0.4/original-upvar-errors)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/9.1.0/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/9.1.0/original-upvar-errors)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_upvar_error_publication307/jim/original-upvar-errors](../../../../rust/tcl-registry/tests/data/native_upvar_error_publication307/jim/original-upvar-errors)
