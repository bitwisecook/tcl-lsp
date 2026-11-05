# naming.list.assignment-object-trace-inline-selection

Kind: `native-observation`

## Problem statement

Installing a native command object trace can force generic invocation even when the source would normally inline lassign. The explicit allow-inline flag selects a different compiler policy, so the mere presence of a trace or fixed command name cannot identify the assignment order. These inert observer controls isolate flags0 from TCL_ALLOW_INLINE_COMPILATION without callback mutation or argv retention.

## Question

Does Tcl_CreateObjTrace flags0 versus TCL_ALLOW_INLINE_COMPILATION change lassign compilation and target-substitution order for the four retained C controls?

## Conclusion

On C8.5 through C9.1, flags0 suppresses inline assignment and every control errors while reading the still-missing first target. TCL_ALLOW_INLINE_COMPILATION retains the compiled earlier-store order: cases0,2,3 return A B and case1 returns A STOP. C8.4 lacks lassign in both trace modes. Jim has no measured Tcl_CreateObjTrace counterpart in this C-only question. The exact trace flag is an independent admission input, not a deduction from source syntax or observer presence.

## Scope

Ten actual C runs of one retained inert Tcl_CreateObjTrace program, five releases times generic flags0 and allow-inline flags. Original {A} B C object, four ASCII procedure controls and native instruction/header snapshots. The independent untraced/Jim programs are retained as related context only; they do not measure a Jim trace API, arbitrary callback effects or execution-trace handler mutation.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl8.4 8.4.20.

untraced: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,storeScalar1,storeScalar1,list:2,; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; generic: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,storeScalar1,storeScalar1,list:2,; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,

### tcl8.5, tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4.

untraced: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### tcl9.1

Status: `observed`. Channel: Original counted public string object vector after ASCII procedure source installation. Version: tcl9.1 9.1.0.

untraced: R|0|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:5,over:1,listIndexImm:1,storeStk,listRangeImm,storeScalar:4,list:2,; R|2|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeArray:3,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:5,over:1,listIndexImm:1,storeStk,listRangeImm,storeScalar:4,list:2,; R|2|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeArray:3,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### jim

Status: `not-tested`. Channel: Not tested for Tcl_CreateObjTrace flag selection. Version: jim Jim0.84.

No Tcl_CreateObjTrace flag counterpart is measured for Jim by this program. Its separate untraced ordering stream is related context only.

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

## Evidence

- [rust/tcl-registry/tests/data/native_list_assignment_order](../../../../rust/tcl-registry/tests/data/native_list_assignment_order)
