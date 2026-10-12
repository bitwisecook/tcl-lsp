# naming.list.stock-length-same-original-storage

Kind: `native-observation`

## Problem statement

Length of a retained original numeric, container or string object can preserve or replace its native primary without changing bytes. An empty list result can also reuse a primed pooled object in compiled code, so source value equality cannot establish freshness or current stock class.

## Question

Which original primary and resident-string transitions occur during native Length?

## Conclusion

The204 same-original Length rows preserve release-specific cache and resident-string changes, including malformed-string errors. The probe substitutes List for C8.4 Dict and String for Jim ByteArray; those rows do not measure the absent classes.

## Scope

Public native factory returns the exact held original object, then ASCII Tcl_Eval/Jim_Eval invokes dynamic generic or procedure Length. Cache fields are captured before result getter.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original source/output hashes and native static library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original source/output hashes and native static library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original source/output hashes and native static library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original source/output hashes and native static library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original source/output hashes and native static library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### jim

Status: `observed`. Version: Jim0.84. Build: Original source/output hashes and native static library 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Jim Tcl.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json). SHA-256 `2c6bbf318c9a5c963507367b520c1d000b985bd4f13f0030c22da4a68d5f2e42`. Each input/source/library/output hash and exact row count; original executed binary/version startup not retained here.
- `e1` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c). SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`. Exact original factory classes, native invocation routes and getter order.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.4.20.txt). SHA-256 `6d0a66337e3d297873c2dd60a3e6e2212b8fadf26b754dd9addd76d9805085ec`. Exact original 34 rows with native primary/residency/result fields.
- `e3` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c). SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`. Exact original factory classes, native invocation routes and getter order.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.5.19.txt). SHA-256 `68c3cd71fb48debe80ff7e1c7bc0d8f4dec6077916df55ac755d1dfc995baa92`. Exact original 34 rows with native primary/residency/result fields.
- `e5` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c). SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`. Exact original factory classes, native invocation routes and getter order.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/8.6.18.txt). SHA-256 `68c3cd71fb48debe80ff7e1c7bc0d8f4dec6077916df55ac755d1dfc995baa92`. Exact original 34 rows with native primary/residency/result fields.
- `e7` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c). SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`. Exact original factory classes, native invocation routes and getter order.
- `e8` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.0.4.txt). SHA-256 `21545219f4db9e8ff8aa81b57af610d11dc3557779c4dd7831bfeded9cf57ffb`. Exact original 34 rows with native primary/residency/result fields.
- `e9` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c). SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`. Exact original factory classes, native invocation routes and getter order.
- `e10` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/9.1.0.txt). SHA-256 `21545219f4db9e8ff8aa81b57af610d11dc3557779c4dd7831bfeded9cf57ffb`. Exact original 34 rows with native primary/residency/result fields.
- `e11` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim-probe.c). SHA-256 `45159e8e3a4daddbc22860eb2de36eeef1f55d1780cbf0d2fb632c8a3dd36de6`. Exact original factory classes, native invocation routes and getter order.
- `e12` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim.txt). SHA-256 `cc5c61e7f9abaafb9d4d0990a92aa3b18a2eadc76d07ed8195cd442589923def`. Exact original 34 rows with native primary/residency/result fields.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Full-source SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`; snippet SHA-256 `f55c5f5b904c068e494061c8860dec2d68bd0f15fae566fca215842c5eb5f327`; retained evidence `e1`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Full-source SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`; snippet SHA-256 `f55c5f5b904c068e494061c8860dec2d68bd0f15fae566fca215842c5eb5f327`; retained evidence `e3`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Full-source SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`; snippet SHA-256 `f55c5f5b904c068e494061c8860dec2d68bd0f15fae566fca215842c5eb5f327`; retained evidence `e5`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Full-source SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`; snippet SHA-256 `f55c5f5b904c068e494061c8860dec2d68bd0f15fae566fca215842c5eb5f327`; retained evidence `e7`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Full-source SHA-256 `3d4d3e03181a22ef6df8a6eafadebe3c46e978dff058c914368d7f6ee1d82ea6`; snippet SHA-256 `f55c5f5b904c068e494061c8860dec2d68bd0f15fae566fca215842c5eb5f327`; retained evidence `e9`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim-probe.c`, function `main: original source/object invocation and observation`, lines 5–5. Full-source SHA-256 `45159e8e3a4daddbc22860eb2de36eeef1f55d1780cbf0d2fb632c8a3dd36de6`; snippet SHA-256 `f7f9a73c98a9e9016e392c060c410a93a604f5d9e9b45c0503b9259c2937f2a8`; retained evidence `e11`.

```text
int main(void){int route,mode;for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Jim_Interp*i=Jim_CreateInterp();char source[160];int code;Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Jim_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Jim_String(Jim_GetResult(i));for(;*s;s++)printf("%02x",*s);}puts("");Jim_DecrRefCount(i,held);Jim_FreeInterp(i);}return 0;}

```


## Consumer bindings

- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeStockListLengthProtocol::normal_cache_disposition`: Storage-class/normal-result cache selection independent of arbitrary object effects.
- [rust/tcl-registry/src/native_stock_list/tests.rs](../../../../rust/tcl-registry/src/native_stock_list/tests.rs), `native_stock_list::tests::stock_length_native_same_object_matrix` (linked): Compares204 retained same-object primary disposition rows with selected storage purpose.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `value_ops::tests::physical_list_length_matches_all_204_native_storage_rows` (linked): Compare actual runtime values and stored primary/residency to all204 original rows; substituted unavailable classes remain scoped.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-native-list-object-probes.py",
  "--corpus",
  "stock-length",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Requires exact original probe and library hashes; independently provision the matching C release or Jim static build. The runner records its fresh compiler arguments, executable hash, process status and streams and compares stdout byte for byte. Original stderr is absent from these manifests; the current replay separately requires empty stderr and does not claim retained stderr equality. Private C instruction probes require matching tclInt.h/tclCompile.h ABI. Missing originals and mismatched libraries are adapter refusals, not native guest answers. C library callbacks require the matching Tcl library directory. Every process has a60-second budget; run serially or obey a global maximum of two native processes. Reconfirmation of the oracle does not establish Rust parity or a native capability.
