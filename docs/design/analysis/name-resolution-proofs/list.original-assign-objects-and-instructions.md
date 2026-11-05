# naming.list.original-assign-objects-and-instructions

Kind: `native-observation`

## Problem statement

A source assign operation can use an inline instruction or an ordinary handler, and either route can change the same original list operand differently. Completion bytes alone omit its primary representation, resident spelling and reference count. These exact procedure/object-vector controls distinguish source ordering and native compile selection without turning their byte values into compiler authority.

## Question

Which completion, original-object header and instruction windows occur for the retained assign list cases 11,12,13,14,23,28?

## Conclusion

The six retained native programs supply exact result/header pairs for the selected cases. C instruction projections vary by release; Jim has its own object and result rows and supplies no C instruction evidence. Errors, absent C8.4 lassign, no-element insert behavior and dynamic substitutions remain version-specific. The shared recipe must select the actual native purpose and retained original operands independently of current bytes or reported primary. Structured WrongArguments presentation delegates NativeWrongArgumentsProtocol::string_result to select the C append-based String producer. A generic message lookalike cannot select this receipt; Jim and explicit Logical protocol rows have no C String producer. Runtime command_error_string_result/report_cmd_error and VM completion_from_cmd_error consume the same selected recipe. Linked presenter controls establish implementation coverage, not another native object/header observation or an executed Rust result. NativeVariableNameProtocol::diagnostic_resets_interpreter_result selects the C8.4 variable-diagnostic reset before publishing the genuine variable error; Runtime original_c_variable_failure_input and VM variable_access_error_at consume that same recipe. The independently inspected C8.4 VarErrMsg reset/append path differs from C8.5 ObjVarErrMsg setting the result. The unchanged original lassign/error/missing-variable windows remain the measured premise. This source implementation boundary neither changes Catch/formal semantics nor infers a header from bytes, relaxes an append extent guard or supplies a missing primitive failure.

## Scope

ASCII source installed through C Tcl_EvalEx(length=-1, GLOBAL) or Jim_Eval, then original {A} B C object passed to p by public object-vector evaluation. R fields are captured before obtaining result bytes; O fields follow the result getter and precede their own getter; H fields precede their own getter. Cases 11,12,13,14,23,28. These rows do not contain opaque names, raw NUL or a general read/normal/world-effect guarantee.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.4 8.4.20.

R|11|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,; R|12|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,list:2,; R|13|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,list:3,; R|14|1|none|1|3|53544f50|storeScalar1,storeArray1,; R|23|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,; R|28|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,

### tcl8.5

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.5 8.5.19.

R|11|0|list|0|1|43|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|12|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar1,storeArray1,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|3|53544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|28|1|string|1|1|77726f6e67202320617267733a2073686f756c6420626520226c61737369676e206c697374207661724e616d65203f7661724e616d65202e2e2e3f22|storeScalar1,storeArray1,

### tcl8.6, tcl9.0

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

R|11|0|list|0|1|43|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|12|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar1,storeArray1,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|4|53544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|28|0|list|0|1|4120422043|storeScalar1,storeArray1,

### tcl9.1

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl9.1 9.1.0.

R|11|0|list|0|1|43|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,; R|12|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar:2,storeArray:3,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|4|53544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,; R|28|0|list|0|1|4120422043|storeScalar:2,storeArray:3,

### jim

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: jim Jim0.84.

R|11|0|list|0|1|43|; R|12|0|list|0|1|412042|; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|; R|14|1|source|1|3|53544f50|; R|23|0|source|1|2|41|; R|28|1|none|1|1|77726f6e67202320617267733a2073686f756c6420626520226c61737369676e207661724c697374206c697374203f7661724e616d65202e2e2e3f22|

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/jim-probe.c`, function `main: original source/object invocation and observation`, lines 8–8. Retained reference `e3`.

```text
int main(void){Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"mark",Mark,NULL,NULL);const char*cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){Jim_Eval(i,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}");char source[4096];snprintf(source,sizeof source,"proc p {name idx} {set x $name;set a(1) $name;%s}",cases[caseId]);if(Jim_Eval(i,source)!=JIM_OK){printf("SETUP|%d\n",caseId);continue;}Jim_Obj*v[]={Jim_NewStringObj(i,"p",-1),Jim_NewStringObj(i,"{A} B C",-1),Jim_NewStringObj(i,"1",-1)};for(int k=0;k<3;k++)Jim_IncrRefCount(v[k]);markCount=0;int code=Jim_EvalObjVector(i,3,v);printf("R|%d|%d|",caseId,code);state(Jim_GetResult(i));puts("|");printf("O|%d|",caseId);state(v[1]);puts("");for(int k=0;k<3;k++)Jim_DecrRefCount(i,v[k]);}Jim_FreeInterp(i);return 0;}

```

## Consumer bindings

- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `compile_native_list_operation`: Selected original compiler-word recipe; source/static geometry does not grant ordinary handler dispatch.
- [rust/tcl-registry/src/native_wrong_arguments.rs](../../../../rust/tcl-registry/src/native_wrong_arguments.rs), `NativeWrongArgumentsProtocol::string_result`: Select the C String producer only from structured actual WrongArguments purpose, without message-based recovery.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `Interp::command_error_string_result`: Consume the selected structured shared String-result producer in Runtime error presentation.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `completion_from_cmd_error`: Consume the same structured error presentation receipt in VM; source implementation only.
- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::diagnostic_resets_interpreter_result`: Select only the C8.4 reached variable-diagnostic reset purpose; later object error presenters remain independent.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `Interp::original_c_variable_failure_input`: Reset the prior error episode only when the genuine reached variable diagnostic selects that release purpose.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Interp::variable_access_error_at`: Consume the same selected variable-diagnostic reset and independent error presenter without changing Catch or formal protocols.
- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `native_list_operations_compilation::tests::original_list_operations_match_150_native_compile_windows` (linked): Compare all150 retained C opcode admission windows; does not assert Jim compiler or object-header parity.
- [rust/tcl-vm/src/exec/native_list_operations_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_operations_tests.rs), `exec::native_list_operations_tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Compare the included operation subset against retained six-engine result/header pairs, preserving error completions.
- [runtime/rust/src/interp/native_body_artifact/native_list_operations.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_operations.rs), `interp::native_body_artifact::native_list_operations::tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Runtime compares included subset original object, header and completion; other operation/header windows are not covered by this selector.
- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `interp::tests::native_wrong_arguments_string_result_requires_the_structured_presenter` (linked): Structured C WrongArguments retains the selected String result; generic equal message bytes keep their independent untyped producer.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `command::tests::native_wrong_arguments_completion_preserves_the_selected_string_producer` (linked): VM consumes the same structured C String producer and refuses message-lookalike recovery; no new native header observation.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 1 | 0 | 1 |
| `isolated` | 1 | 1 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_list_operations](../../../../rust/tcl-registry/tests/data/native_list_operations)
