# naming.list.empty-result-compiled-pool-reuse

Kind: `native-observation`

## Problem statement

Length of a retained original numeric, container or string object can preserve or replace its native primary without changing bytes. An empty list result can also reuse a primed pooled object in compiled code, so source value equality cannot establish freshness or current stock class.

## Question

Does a second empty list result reuse the first primed original object in generic and procedure evaluation?

## Conclusion

All five C generic controls return a distinct second plain String. C8.4 through C8.6 compiled controls also return a distinct String; C9.0 and C9.1 compiled controls return the same primed List. Jim has no empty-result identity rows in this fixture. The distinction forbids granting generic allocation freshness from compiled or source-only cache metadata.

## Scope

Two native empty list results, first explicitly converted to List. Pointer equality and second primary are inspected before getters, generic versus compiled C invocation. No arbitrary allocator identity or source Native header grant.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=0 second=string

### tcl9.0, tcl9.1

Status: `observed`. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=1 second=list

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Retained reference `e1`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Retained reference `e3`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Retained reference `e5`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Retained reference `e7`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Retained reference `e9`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

## Consumer bindings

- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeStockListLengthProtocol::normal_cache_disposition`: Storage-class/normal-result cache selection independent of arbitrary object effects.
- [rust/tcl-registry/src/native_stock_list/tests.rs](../../../../rust/tcl-registry/src/native_stock_list/tests.rs), `native_stock_list::tests::empty_list_root_provider_requires_generic_normal_result` (linked): Generic empty-provider class closure is independently selected; no compiled pooled object or source freshness grant.

## Evidence

- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.1.0.txt)
