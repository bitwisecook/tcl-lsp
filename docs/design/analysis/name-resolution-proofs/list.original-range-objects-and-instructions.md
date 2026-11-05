# naming.list.original-range-objects-and-instructions

Kind: `native-observation`

## Problem statement

A source range operation can use an inline instruction or an ordinary handler, and either route can change the same original list operand differently. Completion bytes alone omit its primary representation, resident spelling and reference count. These exact procedure/object-vector controls distinguish source ordering and native compile selection without turning their byte values into compiler authority.

## Question

Which completion, original-object header and instruction windows occur for the retained range list cases 0,1,2,3,4,24?

## Conclusion

The six retained native programs supply exact result/header pairs for the selected cases. C instruction projections vary by release; Jim has its own object and result rows and supplies no C instruction evidence. Errors, absent C8.4 lassign, no-element insert behavior and dynamic substitutions remain version-specific. The shared recipe must select the actual native purpose and retained original operands independently of current bytes or reported primary. Shared CmdCore command_range and command_range_action keep command-result birth separate from instruction-result birth. A reached empty command range selects an empty List primary/store in C9.0, while C9.1 selects a fresh plain empty object. The retained original R3 header fields are list|0|1 for9.0.4 and none|1|1 for9.1.0, captured before the result getter; neither original instruction stream includes listRangeImm for this case. The separately selected opcode-empty action cannot donate the C9.0 command List birth. Runtime and VM select NativeListRangeSelection with the genuine original list/index purposes, then consume the selected result action rather than recover a producer from empty bytes. The three linked software controls compare actual result primary/store and independent release/command/opcode purposes; the unchanged original84 windows remain separate native observations and supply no additional private allocator, member identity or lifetime grant.

## Scope

ASCII source installed through C Tcl_EvalEx(length=-1, GLOBAL) or Jim_Eval, then original {A} B C object passed to p by public object-vector evaluation. R fields are captured before obtaining result bytes; O fields follow the result getter and precede their own getter; H fields precede their own getter. Cases 0,1,2,3,4,24. These rows do not contain opaque names, raw NUL or a general read/normal/world-effect guarantee. Linked Rust definitions are software coverage obligations. Their source assertions require independent execution receipts; they add no provider observation, feature availability, native object/frame or edit authority.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.4 8.4.20.

R|0|0|list|0|1|4120422043|storeScalar1,storeArray1,; R|1|0|list|0|1|42|storeScalar1,storeArray1,; R|2|0|list|0|1|4120422043|storeScalar1,storeArray1,; R|3|1|string|1|1|62616420696e6465782022656e642b3130223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f|storeScalar1,storeArray1,; R|4|0|list|0|1|422043|storeScalar1,storeArray1,; R|24|0|none|1|1||storeScalar1,storeArray1,

### tcl8.5

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.5 8.5.19.

R|0|0|list|0|1|4120422043|storeScalar1,storeArray1,; R|1|0|list|0|1|42|storeScalar1,storeArray1,; R|2|0|list|0|1|4120422043|storeScalar1,storeArray1,; R|3|0|none|1|1||storeScalar1,storeArray1,; R|4|0|list|0|1|422043|storeScalar1,storeArray1,; R|24|0|none|1|1||storeScalar1,storeArray1,

### tcl8.6

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl8.6 8.6.18.

R|0|0|list|0|1|4120422043|storeScalar1,storeArray1,listRangeImm,; R|1|0|list|0|1|42|storeScalar1,storeArray1,listRangeImm,; R|2|0|list|0|1|4120422043|storeScalar1,storeArray1,listRangeImm,; R|3|0|none|1|1||storeScalar1,storeArray1,listRangeImm,; R|4|0|list|0|1|422043|storeScalar1,storeArray1,; R|24|0|none|1|1||storeScalar1,storeArray1,listRangeImm,

### tcl9.0

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl9.0 9.0.4.

R|0|0|list|0|1|4120422043|storeScalar1,storeArray1,listRangeImm,; R|1|0|list|0|1|42|storeScalar1,storeArray1,listRangeImm,; R|2|0|list|0|1|4120422043|storeScalar1,storeArray1,listRangeImm,; R|3|0|list|0|1||storeScalar1,storeArray1,; R|4|0|list|0|1|422043|storeScalar1,storeArray1,; R|24|0|none|1|1||storeScalar1,storeArray1,listRangeImm,

### tcl9.1

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: tcl9.1 9.1.0.

R|0|0|list|0|1|4120422043|storeScalar:2,storeArray:3,listRangeImm,; R|1|0|list|0|1|42|storeScalar:2,storeArray:3,listRangeImm,; R|2|0|list|0|1|4120422043|storeScalar:2,storeArray:3,listRangeImm,; R|3|0|none|1|1||storeScalar:2,storeArray:3,; R|4|0|list|0|1|422043|storeScalar:2,storeArray:3,; R|24|0|none|1|1||storeScalar:2,storeArray:3,listRangeImm,

### jim

Status: `observed`. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source. Version: jim Jim0.84.

R|0|0|list|0|1|4120422043|; R|1|0|list|0|1|42|; R|2|0|list|0|1|4120422043|; R|3|0|list|0|1||; R|4|0|list|0|1|422043|; R|24|0|none|1|38||

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
- [rust/tcl-cmd-core/src/native_list_index.rs](../../../../rust/tcl-cmd-core/src/native_list_index.rs), `command_range`: Run genuine original command-range index/list purposes while preserving the selected command-result action independently of opcode empty-result production.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `command_range_action`: Select command-specific empty range result/store birth without reusing a distinct opcode-empty purpose.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `NativeListRangeSelection`: Carry the selected result action so Runtime and VM do not infer result-store birth from empty bytes.
- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `native_list_operations_compilation::tests::original_list_operations_match_150_native_compile_windows` (linked): Compare all150 retained C opcode admission windows; does not assert Jim compiler or object-header parity.
- [rust/tcl-vm/src/exec/native_list_operations_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_operations_tests.rs), `exec::native_list_operations_tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Compare the included operation subset against retained six-engine result/header pairs, preserving error completions.
- [runtime/rust/src/interp/native_body_artifact/native_list_operations.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_operations.rs), `interp::native_body_artifact::native_list_operations::tests::original_range_and_assignment_match_84_native_completion_and_original_header_pairs` (linked): Runtime compares included subset original object, header and completion; other operation/header windows are not covered by this selector.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `native_list_storage::tests::command_empty_range_keeps_c90_list_birth_separate_from_c91_and_opcode_empty` (linked): The unchanged original C9.0/C9.1 R3 rows have list|0|1 versus none|1|1 and neither has listRangeImm. Selected command actions keep C9.0 EmptyList versus C9.1 FreshEmpty separate from the independent opcode FreshEmpty action; no private allocation/lifetime or Native admission is inferred.
- [runtime/rust/src/list/native_list_storage.rs](../../../../runtime/rust/src/list/native_list_storage.rs), `list::native_list_storage::tests::command_range_empty_result_has_actual_list_store_only_in_c90` (linked): Genuine selected command/index/list purposes retain a software List store only in C9.0; C9.1 and earlier empty-command results have no donated List store. Independent opcode-empty action and original public header rows remain separate.
- [rust/tcl-vm/src/value/native_list_storage.rs](../../../../rust/tcl-vm/src/value/native_list_storage.rs), `value::native_list_storage::tests::command_range_empty_result_has_actual_list_store_only_in_c90` (linked): VM consumes the shared reached-command result action: C9.0 has an empty List store while C9.1 selects a fresh plain empty result; opcode-empty production stays independent. No native allocator/member/lifetime identity is inferred.
- [runtime/rust/src/value_ops/native_list_index.rs](../../../../runtime/rust/src/value_ops/native_list_index.rs), `c_returned_member_transport_preserves_original_member_reference_and_actual_result_hold` (linked): Software source definition checks c returned member transport preserves original member reference and actual result hold.
- [runtime/rust/src/value_ops/native_list_index.rs](../../../../runtime/rust/src/value_ops/native_list_index.rs), `c_fresh_empty_returned_transport_has_zero_count_and_exact_fresh_cleanup` (linked): Software source definition checks c fresh empty returned transport has zero count and exact fresh cleanup.

## Evidence

- [rust/tcl-registry/tests/data/native_list_operations](../../../../rust/tcl-registry/tests/data/native_list_operations)
