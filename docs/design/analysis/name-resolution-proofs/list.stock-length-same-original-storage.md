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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim Jim0.84.

34 route/mode rows retained. Malformed String reports Error; ordinary List primary stays List. Mode11–13 C8.4 substitute List, and mode9–10 Jim substitute String. Numeric/String and resident transitions are the exact before/after fields, not inferred from result bytes.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Retained reference `e1`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Retained reference `e3`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Retained reference `e5`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Retained reference `e7`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/probe.c`, function `main: original source/object invocation and observation`, lines 15–15. Retained reference `e9`.

```text
int main(int argc,char**argv){int mode,route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Tcl_Interp*i=Tcl_CreateInterp();char source[160];int code;Tcl_CreateObjCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Tcl_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Tcl_GetStringResult(i);for(;*s;s++)printf("%02x",*s);}puts("");Tcl_DecrRefCount(held);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/jim-probe.c`, function `main: original source/object invocation and observation`, lines 5–5. Retained reference `e11`.

```text
int main(void){int route,mode;for(route=0;route<2;route++)for(mode=0;mode<17;mode++){Jim_Interp*i=Jim_CreateInterp();char source[160];int code;Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"make",make,NULL,NULL);if(route)snprintf(source,sizeof(source),"proc f {} {set x [make %d]; llength $x}; f",mode);else snprintf(source,sizeof(source),"set x [make %d]; set cmd llength; $cmd $x",mode);code=Jim_Eval(i,source);printf("route=%d mode=%d before=%s beforestring=%d code=%d after=%s afterstring=%d result=",route,mode,before,before_string,code,held->typePtr?held->typePtr->name:"string",held->bytes!=NULL);{const unsigned char*s=(const unsigned char*)Jim_String(Jim_GetResult(i));for(;*s;s++)printf("%02x",*s);}puts("");Jim_DecrRefCount(i,held);Jim_FreeInterp(i);}return 0;}

```

## Consumer bindings

- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeStockListLengthProtocol::normal_cache_disposition`: Storage-class/normal-result cache selection independent of arbitrary object effects.
- [rust/tcl-registry/src/native_stock_list/tests.rs](../../../../rust/tcl-registry/src/native_stock_list/tests.rs), `native_stock_list::tests::stock_length_native_same_object_matrix` (linked): Compares204 retained same-object primary disposition rows with selected storage purpose.
- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `value_ops::tests::physical_list_length_matches_all_204_native_storage_rows` (linked): Compare actual runtime values and stored primary/residency to all204 original rows; substituted unavailable classes remain scoped.

## Evidence

- [rust/tcl-syntax/tests/data/native_list_methods/stock_length](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length)
