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

Status: `observed`. Version: 8.4.20. Build: compile_status=0, process=0, library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011, executable c71657c8bbf057eff0ffc20c4e0555bfea6a0bff5250c9e7b9e691f361851613. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|11|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,; R|12|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,list:2,; R|13|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,list:3,; R|14|1|none|1|3|53544f50|storeScalar1,storeArray1,; R|23|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,; R|28|1|string|1|1|696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22|storeScalar1,storeArray1,

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: compile_status=0, process=0, library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df, executable 37fd698661eb52dc667ae952ccfccb2502690d9074843f1f5f676faae6ec8881. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|11|0|list|0|1|43|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|12|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar1,storeArray1,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|3|53544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|28|1|string|1|1|77726f6e67202320617267733a2073686f756c6420626520226c61737369676e206c697374207661724e616d65203f7661724e616d65202e2e2e3f22|storeScalar1,storeArray1,

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: compile_status=0, process=0, library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6, executable 759144f1aa71e1034935ef0a35456913c137a4746d955283107db3e022d72445. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|11|0|list|0|1|43|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|12|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar1,storeArray1,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|4|53544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|28|0|list|0|1|4120422043|storeScalar1,storeArray1,

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: compile_status=0, process=0, library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04, executable 7e0d5f8bc8dfb2aa8b2cdde680ebaf3cc9ba699288eb0bae6dd3ca05234a2ca4. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|11|0|list|0|1|43|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|12|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar1,storeArray1,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|4|53544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,dup,listIndexImm:1,storeScalar1,listRangeImm,; R|28|0|list|0|1|4120422043|storeScalar1,storeArray1,

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: compile_status=0, process=0, library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33, executable 2536f7e255d18ce9d87ed5414e85a4a96861586337cbf15832adb279146ff19e. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|11|0|list|0|1|43|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,; R|12|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,list:2,; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|storeScalar:2,storeArray:3,over:1,listIndexImm:0,storeStk,over:1,listIndexImm:1,storeStk,listRangeImm,list:3,; R|14|1|none|1|4|53544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,; R|23|0|none|1|2|41|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,dup,listIndexImm:1,storeScalar:5,listRangeImm,; R|28|0|list|0|1|4120422043|storeScalar:2,storeArray:3,

### jim

Status: `observed`. Version: Jim0.84. Build: Independent original compile/run0 and source, header, library and executable SHA in its Jim manifest. No newer launcher version is borrowed.. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Jim Tcl.

R|11|0|list|0|1|43|; R|12|0|list|0|1|412042|; R|13|0|list|0|1|412042207b6669727374207365636f6e647d|; R|14|1|source|1|3|53544f50|; R|23|0|source|1|2|41|; R|28|1|none|1|1|77726f6e67202320617267733a2073686f756c6420626520226c61737369676e207661724c697374206c697374203f7661724e616d65202e2e2e3f22|

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_list_operations/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_operations/manifest.json). SHA-256 `f65701313dbc41590938988b3a959bc511813a2e550672b7283e263802c13e0b`. Original C compile/process rows, exact program/static library/executable/output hashes and30 result windows per provider.
- `e1` (provider): [rust/tcl-registry/tests/data/native_list_operations/jim-manifest.json](../../../../rust/tcl-registry/tests/data/native_list_operations/jim-manifest.json). SHA-256 `d759911fcf14817b32de11ff5f7d049f45ae01461bc86c949ca588b387657850`. Independent Jim compiler and source/header/library/executable/output identity; exact30 original result windows.
- `e2` (input): [rust/tcl-registry/tests/data/native_list_operations/probe.c](../../../../rust/tcl-registry/tests/data/native_list_operations/probe.c). SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`. Actual C input program, original public object-vector entry and private opcode/header observation order.
- `e3` (input): [rust/tcl-registry/tests/data/native_list_operations/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_list_operations/jim-probe.c). SHA-256 `53a9b93a4e38d27e5a4693e613bfb25b81b2a54520b5cbf85ea68e3e11929152`. Independent Jim original object-vector program and same-object snapshots.
- `e4` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.4.20.txt). SHA-256 `cca6c25a39449b64b5a5c9fdb5da2eb5a0deda8f99bc157a6079743f74ab7ba0`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.
- `e5` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.5.19.txt). SHA-256 `83e731a5b392cb87dd6f7f8c6de9e6dc5e21e80b41e55f2ba0763d1d010716c4`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.
- `e6` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt). SHA-256 `e6d872d320e8becf464c430e382ac07add5297b5d804f811116a4a6d84446b74`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.
- `e7` (observation): [rust/tcl-registry/tests/data/native_list_operations/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/9.0.4.txt). SHA-256 `12fa60819a0b3ec0d2800826b14622815b7a3d798dc5485d9be77de14c23dfd1`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.
- `e8` (observation): [rust/tcl-registry/tests/data/native_list_operations/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/9.1.0.txt). SHA-256 `8ea8571ca185c5c6879a8398b3e4516174b67227a2b860e71ef34efc27b7599f`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.
- `e9` (observation): [rust/tcl-registry/tests/data/native_list_operations/jim.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/jim.txt). SHA-256 `c6d78db36f9a442da2b02d3fe2ea86512765ef332732d16774d1c536d26ca528`. Selected cases [11, 12, 13, 14, 23, 28] within exact full original stream; other operation rows belong to independent questions.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`; snippet SHA-256 `ace728733568d27896d6147e259d0e39f2ae8f0fe91e2fb5ee9389b9e5ce4fe6`; retained evidence `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`; snippet SHA-256 `ace728733568d27896d6147e259d0e39f2ae8f0fe91e2fb5ee9389b9e5ce4fe6`; retained evidence `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`; snippet SHA-256 `ace728733568d27896d6147e259d0e39f2ae8f0fe91e2fb5ee9389b9e5ce4fe6`; retained evidence `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`; snippet SHA-256 `ace728733568d27896d6147e259d0e39f2ae8f0fe91e2fb5ee9389b9e5ce4fe6`; retained evidence `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `358eb2953215724d155329449a3452d3179f0ca37a94b3113fc1c59b157b02e9`; snippet SHA-256 `ace728733568d27896d6147e259d0e39f2ae8f0fe91e2fb5ee9389b9e5ce4fe6`; retained evidence `e2`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lrange $name 0 end","lrange $name 1 end-1","lrange $name -9 999","lrange $name end+10 end","lrange $name $idx end","linsert $name 0 X","linsert $name end X","linsert $name 1 X Y","linsert $name end-1 X","linsert $name $idx X","linsert $name 1","lassign $name first second","lassign $name first second; list $first $second","lassign $name [tap first] [tap second]; list $first $second $::log","lassign $name first [error STOP]","lset x 1 X","lset x {1} X","lset x {} X","lset x X","lset x [tap 1] [tap X]; list $x $::log","lset x [error STOP] [tap X]","lset a($idx) 1 X","lset x 0 0 X","lassign $name first second; mark $name; set first","lrange $name 1 end; mark $name","linsert $name 1 X; mark $name","lset x 1 X; mark $name; set x","lset missing 1 X","lassign $name","linsert $name -8 X"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_operations/jim-probe.c`, function `main: original source/object invocation and observation`, lines 8–8. Full-source SHA-256 `53a9b93a4e38d27e5a4693e613bfb25b81b2a54520b5cbf85ea68e3e11929152`; snippet SHA-256 `35891aa3b796b131f781de02bcd1644c50dfbfce365edd4a6c1d944598104482`; retained evidence `e3`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-native-list-object-probes.py",
  "--corpus",
  "operations",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Requires exact original probe and library hashes; independently provision the matching C release or Jim static build. The runner records its fresh compiler arguments, executable hash, process status and streams and compares stdout byte for byte. Original stderr is absent from these manifests; the current replay separately requires empty stderr and does not claim retained stderr equality. Private C instruction probes require matching tclInt.h/tclCompile.h ABI. Missing originals and mismatched libraries are adapter refusals, not native guest answers. C library callbacks require the matching Tcl library directory. Every process has a60-second budget; run serially or obey a global maximum of two native processes. Reconfirmation of the oracle does not establish Rust parity or a native capability.
