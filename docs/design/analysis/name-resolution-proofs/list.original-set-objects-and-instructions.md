# naming.list.original-set-objects-and-instructions

Kind: `native-observation`

## Problem statement

A source set operation can use an inline instruction or an ordinary handler, and either route can change the same original list operand differently. Completion bytes alone omit its primary representation, resident spelling and reference count. These exact procedure/object-vector controls distinguish source ordering and native compile selection without turning their byte values into compiler authority.

## Question

Which completion, original-object header and instruction windows occur for the retained set list cases 15,16,17,18,19,20,21,22,26,27?

## Conclusion

The six retained native programs supply exact result/header pairs for the selected cases. C instruction projections vary by release; Jim has its own object and result rows and supplies no C instruction evidence. Errors, absent C8.4 lassign, no-element insert behavior and dynamic substitutions remain version-specific. The shared recipe must select the actual native purpose and retained original operands independently of current bytes or reported primary.

## Scope

ASCII source installed through C Tcl_EvalEx(length=-1, GLOBAL) or Jim_Eval, then original {A} B C object passed to p by public object-vector evaluation. R fields are captured before obtaining result bytes; O fields follow the result getter and precede their own getter; H fields precede their own getter. Cases 15,16,17,18,19,20,21,22,26,27. These rows do not contain opaque names, raw NUL or a general read/normal/world-effect guarantee.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: compile_status=0, process=0, library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011, executable c71657c8bbf057eff0ffc20c4e0555bfea6a0bff5250c9e7b9e691f361851613. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|15|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|16|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|17|0|none|1|3|58|storeScalar1,storeArray1,lsetList,storeScalar1,; R|18|0|none|1|3|58|storeScalar1,storeArray1,lsetFlat:2,storeScalar1,; R|19|0|list|0|1|7b41205820437d207b3120587d|storeScalar1,storeArray1,lsetList,storeScalar1,list:2,; R|20|1|none|1|3|53544f50|storeScalar1,storeArray1,lsetList,storeScalar1,; R|21|0|list|0|1|4120582043|storeScalar1,storeArray1,over:2,lsetList,storeArray1,; R|22|0|list|0|1|5820422043|storeScalar1,storeArray1,lsetFlat:4,storeScalar1,; R|26|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|27|1|none|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,lsetList,storeScalar1,

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: compile_status=0, process=0, library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df, executable 37fd698661eb52dc667ae952ccfccb2502690d9074843f1f5f676faae6ec8881. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|15|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|16|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|17|0|none|1|3|58|storeScalar1,storeArray1,lsetList,storeScalar1,; R|18|0|none|1|3|58|storeScalar1,storeArray1,lsetFlat:2,storeScalar1,; R|19|0|list|0|1|7b41205820437d207b3120587d|storeScalar1,storeArray1,lsetList,storeScalar1,list:2,; R|20|1|none|1|3|53544f50|storeScalar1,storeArray1,lsetList,storeScalar1,; R|21|0|list|0|1|4120582043|storeScalar1,storeArray1,over:2,lsetList,storeArray1,; R|22|0|list|0|1|5820422043|storeScalar1,storeArray1,lsetFlat:4,storeScalar1,; R|26|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|27|1|string|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,lsetList,storeScalar1,

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: compile_status=0, process=0, library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6, executable 759144f1aa71e1034935ef0a35456913c137a4746d955283107db3e022d72445. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|15|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|16|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|17|0|none|1|3|58|storeScalar1,storeArray1,lsetList,storeScalar1,; R|18|0|none|1|3|58|storeScalar1,storeArray1,lsetFlat:2,storeScalar1,; R|19|0|list|0|1|7b41205820437d207b3120587d|storeScalar1,storeArray1,lsetList,storeScalar1,list:2,; R|20|1|none|1|4|53544f50|storeScalar1,storeArray1,lsetList,storeScalar1,; R|21|0|list|0|1|4120582043|storeScalar1,storeArray1,over:2,lsetList,storeArray1,; R|22|0|list|0|1|5820422043|storeScalar1,storeArray1,lsetFlat:4,storeScalar1,; R|26|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|27|1|string|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,lsetList,storeScalar1,

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: compile_status=0, process=0, library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04, executable 7e0d5f8bc8dfb2aa8b2cdde680ebaf3cc9ba699288eb0bae6dd3ca05234a2ca4. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|15|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|16|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|17|0|none|1|3|58|storeScalar1,storeArray1,lsetList,storeScalar1,; R|18|0|none|1|3|58|storeScalar1,storeArray1,lsetFlat:2,storeScalar1,; R|19|0|list|0|1|7b41205820437d207b3120587d|storeScalar1,storeArray1,lsetList,storeScalar1,list:2,; R|20|1|none|1|4|53544f50|storeScalar1,storeArray1,lsetList,storeScalar1,; R|21|0|list|0|1|4120582043|storeScalar1,storeArray1,over:2,lsetList,storeArray1,; R|22|0|list|0|1|5820422043|storeScalar1,storeArray1,lsetFlat:4,storeScalar1,; R|26|0|list|0|1|4120582043|storeScalar1,storeArray1,lsetList,storeScalar1,; R|27|1|string|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,lsetList,storeScalar1,

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: compile_status=0, process=0, library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33, executable 2536f7e255d18ce9d87ed5414e85a4a96861586337cbf15832adb279146ff19e. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Tcl.

R|15|0|list|0|1|4120582043|storeScalar:2,storeArray:3,lsetList,storeScalar:2,; R|16|0|list|0|1|4120582043|storeScalar:2,storeArray:3,lsetList,storeScalar:2,; R|17|0|none|1|3|58|storeScalar:2,storeArray:3,lsetList,storeScalar:2,; R|18|0|none|1|3|58|storeScalar:2,storeArray:3,lsetFlat:2,storeScalar:2,; R|19|0|list|0|1|7b41205820437d207b3120587d|storeScalar:2,storeArray:3,lsetList,storeScalar:2,list:2,; R|20|1|none|1|4|53544f50|storeScalar:2,storeArray:3,lsetList,storeScalar:2,; R|21|0|list|0|1|4120582043|storeScalar:2,storeArray:3,over:2,lsetList,storeArray:3,; R|22|0|list|0|1|5820422043|storeScalar:2,storeArray:3,lsetFlat:4,storeScalar:2,; R|26|0|list|0|1|4120582043|storeScalar:2,storeArray:3,lsetList,storeScalar:2,; R|27|1|string|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|storeScalar:2,storeArray:3,lsetList,storeScalar:4,

### jim

Status: `observed`. Version: Jim0.84. Build: Independent original compile/run0 and source, header, library and executable SHA in its Jim manifest. No newer launcher version is borrowed.. Channel: Fresh original public string object passed to Tcl_EvalObjv or Jim_EvalObjVector; ASCII procedure source.. Dialect: Jim Tcl.

R|15|0|list|0|1|4120582043|; R|16|0|list|0|1|4120582043|; R|17|1|none|1|1|62616420696e6465782022223a206d75737420626520696e7465787072206f7220656e643f5b2b2d5d696e74657870723f|; R|18|0|source|1|2|58|; R|19|0|list|0|1|7b41205820437d207b3120587d|; R|20|1|source|1|3|53544f50|; R|21|0|list|0|1|4120582043|; R|22|0|list|0|1|5820422043|; R|26|0|list|0|1|4120582043|; R|27|1|none|1|1|63616e2774207265616420226d697373696e67223a206e6f2073756368207661726961626c65|

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [rust/tcl-registry/tests/data/native_list_operations/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_operations/manifest.json). Original C compile/process rows, exact program/static library/executable/output hashes and30 result windows per provider.
- `e1` (provider): [rust/tcl-registry/tests/data/native_list_operations/jim-manifest.json](../../../../rust/tcl-registry/tests/data/native_list_operations/jim-manifest.json). Independent Jim compiler and source/header/library/executable/output identity; exact30 original result windows.
- `e2` (input): [rust/tcl-registry/tests/data/native_list_operations/probe.c](../../../../rust/tcl-registry/tests/data/native_list_operations/probe.c). Actual C input program, original public object-vector entry and private opcode/header observation order.
- `e3` (input): [rust/tcl-registry/tests/data/native_list_operations/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_list_operations/jim-probe.c). Independent Jim original object-vector program and same-object snapshots.
- `e4` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.4.20.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.
- `e5` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.5.19.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.
- `e6` (observation): [rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/8.6.18.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.
- `e7` (observation): [rust/tcl-registry/tests/data/native_list_operations/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/9.0.4.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.
- `e8` (observation): [rust/tcl-registry/tests/data/native_list_operations/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/9.1.0.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.
- `e9` (observation): [rust/tcl-registry/tests/data/native_list_operations/jim.txt](../../../../rust/tcl-registry/tests/data/native_list_operations/jim.txt). Selected cases [15, 16, 17, 18, 19, 20, 21, 22, 26, 27] within exact full original stream; other operation rows belong to independent questions.

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
- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `native_list_operations_compilation::tests::original_list_operations_match_150_native_compile_windows` (linked): Compare all150 retained C opcode admission windows; does not assert Jim compiler or object-header parity.

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
