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

Status: `observed`. Version: 8.4.20. Build: Recorded original library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011 and executed untraced/trace binaries in separate compile/process receipts.. Channel: Original counted public string object vector after ASCII procedure source installation.. Dialect: Tcl.

untraced: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,storeScalar1,storeScalar1,list:2,; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; generic: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|1|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,storeScalar1,storeScalar1,list:2,; R|2|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,; R|3|1|none|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|storeScalar1,storeArray1,list:2,

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded original library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df and executed untraced/trace binaries in separate compile/process receipts.. Channel: Original counted public string object vector after ASCII procedure source installation.. Dialect: Tcl.

untraced: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded original library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6 and executed untraced/trace binaries in separate compile/process receipts.. Channel: Original counted public string object vector after ASCII procedure source installation.. Dialect: Tcl.

untraced: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded original library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04 and executed untraced/trace binaries in separate compile/process receipts.. Channel: Original counted public string object vector after ASCII procedure source installation.. Dialect: Tcl.

untraced: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,reverse:2,storeScalar1,list:2,; R|2|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeArray1,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar1,storeArray1,dup,listIndexImm:0,storeScalar1,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded original library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33 and executed untraced/trace binaries in separate compile/process receipts.. Channel: Original counted public string object vector after ASCII procedure source installation.. Dialect: Tcl.

untraced: R|0|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:5,over:1,listIndexImm:1,storeStk,listRangeImm,storeScalar:4,list:2,; R|2|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeArray:3,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; generic: R|0|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|1|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|2|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; R|3|1|string|1|1|63616e2774207265616420226669727374223a206e6f2073756368207661726961626c65|; inline-enabled-trace: R|0|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,; R|1|0|list|0|1|412053544f50|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:5,over:1,listIndexImm:1,storeStk,listRangeImm,storeScalar:4,list:2,; R|2|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeArray:3,listRangeImm,list:2,; R|3|0|list|0|1|412042|storeScalar:2,storeArray:3,dup,listIndexImm:0,storeScalar:4,over:1,listIndexImm:1,storeStk,listRangeImm,list:2,

### jim

Status: `not-tested`. Version: Jim0.84. Build: No Jim build used for the C trace-flag question.. Channel: Not tested for Tcl_CreateObjTrace flag selection.. Dialect: Jim Tcl.

No Tcl_CreateObjTrace flag counterpart is measured for Jim by this program. Its separate untraced ordering stream is related context only.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_list_assignment_order/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/manifest.json). SHA-256 `33d27c5048bff4c625711a4c3ffbd638139e5f9c5399e21a28c5d0ca333565be`. Untraced original C source/build/stream receipt.
- `e1` (provider): [rust/tcl-registry/tests/data/native_list_assignment_order/jim-manifest.json](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/jim-manifest.json). SHA-256 `763459489b2d9d60857c9dd474e0f686c6b1d4d842e8e324d7cb46da5252a5de`. Independent Jim original program/build/output receipt.
- `e2` (provider): [rust/tcl-registry/tests/data/native_list_assignment_order/trace-manifest.json](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/trace-manifest.json). SHA-256 `be07955445b52686f6b957449f5976f90ea54fc5dd4b78553a8b313301bedb90`. Ten distinct C source-trace builds/process streams and allow-inline versus generic runtime arguments.
- `e3` (input): [rust/tcl-registry/tests/data/native_list_assignment_order/probe.c](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/probe.c). SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`. Four original substitutions and before/after header observation order.
- `e4` (input): [rust/tcl-registry/tests/data/native_list_assignment_order/trace-probe.c](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/trace-probe.c). SHA-256 `70920f2f8f2808ee0c50aade7107739e2a42d3d7a6744ffc83e5b3d173cb3660`. Actual inert observer and independently selected flags; source differs from untraced program.
- `e5` (input): [rust/tcl-registry/tests/data/native_list_assignment_order/jim-probe.c](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/jim-probe.c). SHA-256 `d3883a8b40f65358ea0bb8c839552f02508942e83671c7e5f2d27864b9bd6a15`. Independent Jim program; no native C bytecode assumption.
- `e6` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20.txt). SHA-256 `b06cb99183069a170f9592fe876c83ead333a8013df54dfc6f40bfdce2b71b7f`. Original untraced four cases and source object snapshots.
- `e7` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20-generic.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20-generic.txt). SHA-256 `46b3ba3825cb53f369514e7ed81888115bbc75414a4a90c7a429656ab4780977`. Exact generic four outcomes and original header pairs.
- `e8` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20-inline-enabled-trace.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.4.20-inline-enabled-trace.txt). SHA-256 `b06cb99183069a170f9592fe876c83ead333a8013df54dfc6f40bfdce2b71b7f`. Exact inline-enabled-trace four outcomes and original header pairs.
- `e9` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19.txt). SHA-256 `baac8dda0263ae4a9359c4d15d9e9c6b7eae821d70eb6fccb2848051b0a275b5`. Original untraced four cases and source object snapshots.
- `e10` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19-generic.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19-generic.txt). SHA-256 `c6498f6d5c0023215aa05b74b05151a83302c592c77b3e7eefd309c8fd9192fb`. Exact generic four outcomes and original header pairs.
- `e11` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19-inline-enabled-trace.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.5.19-inline-enabled-trace.txt). SHA-256 `baac8dda0263ae4a9359c4d15d9e9c6b7eae821d70eb6fccb2848051b0a275b5`. Exact inline-enabled-trace four outcomes and original header pairs.
- `e12` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18.txt). SHA-256 `cc8e3677ba54f3dc25337497a91544ddac021c408e96bbb1505f392748b0eeee`. Original untraced four cases and source object snapshots.
- `e13` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18-generic.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18-generic.txt). SHA-256 `1f472792a1a90ede274034c3268c0f5f1010693cd24c080a5af5de6bb4f964d8`. Exact generic four outcomes and original header pairs.
- `e14` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18-inline-enabled-trace.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/8.6.18-inline-enabled-trace.txt). SHA-256 `cc8e3677ba54f3dc25337497a91544ddac021c408e96bbb1505f392748b0eeee`. Exact inline-enabled-trace four outcomes and original header pairs.
- `e15` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4.txt). SHA-256 `cc8e3677ba54f3dc25337497a91544ddac021c408e96bbb1505f392748b0eeee`. Original untraced four cases and source object snapshots.
- `e16` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4-generic.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4-generic.txt). SHA-256 `1f472792a1a90ede274034c3268c0f5f1010693cd24c080a5af5de6bb4f964d8`. Exact generic four outcomes and original header pairs.
- `e17` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4-inline-enabled-trace.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.0.4-inline-enabled-trace.txt). SHA-256 `cc8e3677ba54f3dc25337497a91544ddac021c408e96bbb1505f392748b0eeee`. Exact inline-enabled-trace four outcomes and original header pairs.
- `e18` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0.txt). SHA-256 `4c415b1486b7e72f07cd29d6b76d7f29874c1562a0585887a867c0e466c485a0`. Original untraced four cases and source object snapshots.
- `e19` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0-generic.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0-generic.txt). SHA-256 `1f472792a1a90ede274034c3268c0f5f1010693cd24c080a5af5de6bb4f964d8`. Exact generic four outcomes and original header pairs.
- `e20` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0-inline-enabled-trace.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/9.1.0-inline-enabled-trace.txt). SHA-256 `4c415b1486b7e72f07cd29d6b76d7f29874c1562a0585887a867c0e466c485a0`. Exact inline-enabled-trace four outcomes and original header pairs.
- `e21` (observation): [rust/tcl-registry/tests/data/native_list_assignment_order/jim.txt](../../../../rust/tcl-registry/tests/data/native_list_assignment_order/jim.txt). SHA-256 `e3c71f0276164549be3980f8ebc9e272997c7f3c0b3197ce089d7411a5b65992`. Exact Jim four missing-first results and original snapshots; no C object trace API measured.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`; snippet SHA-256 `b374015746364573b1970a29a60c3907d3b4635587895b954848ccf96fc5cf6d`; retained evidence `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`; snippet SHA-256 `b374015746364573b1970a29a60c3907d3b4635587895b954848ccf96fc5cf6d`; retained evidence `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`; snippet SHA-256 `b374015746364573b1970a29a60c3907d3b4635587895b954848ccf96fc5cf6d`; retained evidence `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`; snippet SHA-256 `b374015746364573b1970a29a60c3907d3b4635587895b954848ccf96fc5cf6d`; retained evidence `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/probe.c`, function `main: original source/object invocation and observation`, lines 15–18. Full-source SHA-256 `41ad70151861b4690b055cf4826c92de18b8c02b1625864422ffc3330b6cd8cf`; snippet SHA-256 `b374015746364573b1970a29a60c3907d3b4635587895b954848ccf96fc5cf6d`; retained evidence `e3`.

```text
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"lassign $name first [set first]; list $first $A","catch {lassign $name first [error STOP]} caught; list $first $caught","lassign $name first a([set first]); list $first $a(A)","lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-registry/tests/data/native_list_assignment_order/jim-probe.c`, function `main: original source/object invocation and observation`, lines 8–8. Full-source SHA-256 `d3883a8b40f65358ea0bb8c839552f02508942e83671c7e5f2d27864b9bd6a15`; snippet SHA-256 `0a14759f5d2a37deb04343a84b97aa7e0751ec0e931cdfb8f8f7005611e7b061`; retained evidence `e5`.

```text
int main(void){Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"mark",Mark,NULL,NULL);const char*cases[]={"lassign $name first [set first]; list $first $A", "catch {lassign $name first [error STOP]} caught; list $first $caught", "lassign $name first a([set first]); list $first $a(A)", "lassign $name first [proc lassign args {return CUSTOM}; set first]; list $first $A"};for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){Jim_Eval(i,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}");char source[4096];snprintf(source,sizeof source,"proc p {name idx} {set x $name;set a(1) $name;%s}",cases[caseId]);if(Jim_Eval(i,source)!=JIM_OK){printf("SETUP|%d\n",caseId);continue;}Jim_Obj*v[]={Jim_NewStringObj(i,"p",-1),Jim_NewStringObj(i,"{A} B C",-1),Jim_NewStringObj(i,"1",-1)};for(int k=0;k<3;k++)Jim_IncrRefCount(v[k]);markCount=0;int code=Jim_EvalObjVector(i,3,v);printf("R|%d|%d|",caseId,code);state(Jim_GetResult(i));puts("|");printf("O|%d|",caseId);state(v[1]);puts("");for(int k=0;k<3;k++)Jim_DecrRefCount(i,v[k]);}Jim_FreeInterp(i);return 0;}

```


## Consumer bindings

- [rust/tcl-registry/src/native_list_operations_compilation.rs](../../../../rust/tcl-registry/src/native_list_operations_compilation.rs), `compile_native_list_operation`: Native Assign recipe retains each original target operation; trace admission is independently selected.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-native-list-object-probes.py",
  "--corpus",
  "assignment",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Requires exact original probe and library hashes; independently provision the matching C release or Jim static build. The runner records its fresh compiler arguments, executable hash, process status and streams and compares stdout byte for byte. Original stderr is absent from these manifests; the current replay separately requires empty stderr and does not claim retained stderr equality. Private C instruction probes require matching tclInt.h/tclCompile.h ABI. Missing originals and mismatched libraries are adapter refusals, not native guest answers. C library callbacks require the matching Tcl library directory. Every process has a60-second budget; run serially or obey a global maximum of two native processes. Reconfirmation of the oracle does not establish Rust parity or a native capability.
