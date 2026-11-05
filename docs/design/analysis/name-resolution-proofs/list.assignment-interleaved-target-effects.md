# naming.list.assignment-interleaved-target-effects

Kind: `native-observation`

## Problem statement

A later lassign target substitution can read or replace an earlier target or the lassign implementation. Compiled element stores and generic argument evaluation need not occur at the same time. A command object trace can additionally suppress inlining; its allow-inline flag cannot be inferred from the presence of a trace alone.

## Question

Do earlier lassign stores precede later original target substitutions, and how do actual object trace flags affect this ordering?

## Conclusion

Untraced C8.5 through C9.1 store the first target before the later read/error/redefinition controls. Generic object-trace evaluation instead encounters the missing first variable before invoking lassign. The allow-inline trace mode retains the untraced compiled order. C8.4 has no lassign instruction or command, and Jim original evaluation reaches missing first before any assignment. These exact differences require retained native compilation and observation state; current command-name bytes alone cannot choose them.

## Scope

Four ASCII procedure bodies with the original {A} B C argument. Targets include [set first], [error STOP], an array-index read and a nested lassign redefinition. C source uses public Tcl_EvalEx/Tcl_EvalObjv and private instruction/header reads; Jim uses its own public object vector. Trace mode uses Tcl_CreateObjTrace flags0 versus TCL_ALLOW_INLINE_COMPILATION. No callback changes the command or retains argv in the trace itself.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl8.4 8.4.20.

untraced: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,storeScalar1,storeScalar1,list:2,; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,

### tcl8.5, tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4.

untraced: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### tcl9.1

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl9.1 9.1.0.

untraced: R|0|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:5,over:1,listIndexImm:1,storeStk,listRangeImm,storeScalar:4,list:2,; R|2|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeArray:3,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### jim

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: jim Jim0.84.

Four resultError rows: cannot read first before any stores. No C object-trace mode measured.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Retained reference `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/jim-probe.c`, function `main: original source/object invocation and observation`, lines 8–8. Retained reference `e5`.

```text
int main(void){Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"mark",Mark,NULL,NULL);const char*cases[]={"lassign $name first [set first]; list $first $A", "catch {lassign $name first [error STOP]} caught; list $first $caught", "lassign $name first a([set first]); list $first $a(A)", "lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){Jim_Eval(i,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}");char source[4096];snprintf(source,sizeof source,"proc p {name idx} {set x $name;set a(1) $name;%s}",cases[caseId]);if(Jim_Eval(i,source)!=JIM_OK){printf("SETUP|%d\n",caseId);continue;}Jim_Obj*v[]={Jim_NewStringObj(i,"p",-1),Jim_NewStringObj(i,"{A} B C",-1),Jim_NewStringObj(i,"1",-1)};for(int k=0;k<3;k++)Jim_IncrRefCount(v[k]);markCount=0;int code=Jim_EvalObjVector(i,3,v);printf("R|%d|%d|",caseId,code);state(Jim_GetResult(i));puts("|");printf("O|%d|",caseId);state(v[1]);puts("");for(int k=0;k<3;k++)Jim_DecrRefCount(i,v[k]);}Jim_FreeInterp(i);return 0;}

```

## Consumer bindings

- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `compile_native_list_operation`: Native Assign recipe retains each original target operation; trace admission is independently selected.
- [rust/tcl-vm/src/exec/native_list_operations_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_operations_tests.rs), `exec::native_list_operations_tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Includes all four untraced assignment-order cases for all six engines and compares original header/result pairs; command-trace modes require separate native evidence.
- [runtime/rust/src/interp/native_body_artifact/native_list_operations.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_operations.rs), `interp::native_body_artifact::native_list_operations::tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Includes all four untraced assignment-order cases for all six engines and compares original header/result pairs; command-trace modes require separate native evidence.
- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `native_list_operations_compilation::tests::list_assignment_retains_interleaved_original_target_geometry` (linked): Preserves original target operands and compound array geometry, separately from execution order.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 1 | 0 | 0 |
| `isolated` | 0 | 1 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_list_assignment_order](../../../../rust/tcl-registry/tests/data/native_list_assignment_order)
