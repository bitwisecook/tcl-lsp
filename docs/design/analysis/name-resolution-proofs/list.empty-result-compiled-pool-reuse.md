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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original source/output hashes and native static library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=0 second=string

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original source/output hashes and native static library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=0 second=string

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original source/output hashes and native static library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=0 second=string

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original source/output hashes and native static library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=1 second=list

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original source/output hashes and native static library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33 retained; executable SHA, startup version and compiler argv are not recorded in this manifest.. Channel: Native API factory objects retained while ASCII Tcl_Eval/Jim_Eval executes the actual selected route.. Dialect: Tcl.

route=0 first=string primed=list same=0 second=string; route=1 first=string primed=list same=1 second=list

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/manifest.json). SHA-256 `2c6bbf318c9a5c963507367b520c1d000b985bd4f13f0030c22da4a68d5f2e42`. Each input/source/library/output hash and exact row count; original executed binary/version startup not retained here.
- `e1` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c). SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`. Exact original factory classes, native invocation routes and getter order.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.4.20.txt). SHA-256 `5824f1e874dc81eb92339a9a9d8d51e74c6f3f07155f144b14a05df3e620023f`. Exact original 2 rows with native primary/residency/result fields.
- `e3` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c). SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`. Exact original factory classes, native invocation routes and getter order.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.5.19.txt). SHA-256 `5824f1e874dc81eb92339a9a9d8d51e74c6f3f07155f144b14a05df3e620023f`. Exact original 2 rows with native primary/residency/result fields.
- `e5` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c). SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`. Exact original factory classes, native invocation routes and getter order.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-8.6.18.txt). SHA-256 `5824f1e874dc81eb92339a9a9d8d51e74c6f3f07155f144b14a05df3e620023f`. Exact original 2 rows with native primary/residency/result fields.
- `e7` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c). SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`. Exact original factory classes, native invocation routes and getter order.
- `e8` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.0.4.txt). SHA-256 `7b93cbf762174b090d2c50a4506bf5c37f3bf20d10dad479e7d148c865de5a11`. Exact original 2 rows with native primary/residency/result fields.
- `e9` (input): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c). SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`. Exact original factory classes, native invocation routes and getter order.
- `e10` (observation): [rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-9.1.0.txt). SHA-256 `7b93cbf762174b090d2c50a4506bf5c37f3bf20d10dad479e7d148c865de5a11`. Exact original 2 rows with native primary/residency/result fields.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Full-source SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`; snippet SHA-256 `25d7886f78fe6ca4e168d8c3c56628e89019d95c74d06ce5eb33290e658b5dc5`; retained evidence `e1`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Full-source SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`; snippet SHA-256 `25d7886f78fe6ca4e168d8c3c56628e89019d95c74d06ce5eb33290e658b5dc5`; retained evidence `e3`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Full-source SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`; snippet SHA-256 `25d7886f78fe6ca4e168d8c3c56628e89019d95c74d06ce5eb33290e658b5dc5`; retained evidence `e5`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Full-source SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`; snippet SHA-256 `25d7886f78fe6ca4e168d8c3c56628e89019d95c74d06ce5eb33290e658b5dc5`; retained evidence `e7`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/stock_length/empty-result-probe.c`, function `main: original source/object invocation and observation`, lines 3–3. Full-source SHA-256 `915533c26b821b5df4c1a9abe933863e53efcb7d184e6bb142299c42d6fd9d0d`; snippet SHA-256 `25d7886f78fe6ca4e168d8c3c56628e89019d95c74d06ce5eb33290e658b5dc5`; retained evidence `e9`.

```text
int main(int argc,char**argv){int route;(void)argc;Tcl_FindExecutable(argv[0]);for(route=0;route<2;route++){Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj *first,*second;const char*call=route?"f":"$cmd";Tcl_Eval(i,"proc f {} {list}; set cmd list");Tcl_Eval(i,call);first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);printf("route=%d first=%s ",route,first->typePtr?first->typePtr->name:"string");(void)Tcl_ConvertToType(NULL,first,Tcl_GetObjType("list"));printf("primed=%s ",first->typePtr?first->typePtr->name:"string");Tcl_Eval(i,call);second=Tcl_GetObjResult(i);printf("same=%d second=%s\n",first==second,second->typePtr?second->typePtr->name:"string");Tcl_DecrRefCount(first);Tcl_DeleteInterp(i);}Tcl_Finalize();return 0;}

```


## Consumer bindings

- [rust/tcl-registry/src/native_stock_list.rs](../../../../rust/tcl-registry/src/native_stock_list.rs), `NativeStockListLengthProtocol::normal_cache_disposition`: Storage-class/normal-result cache selection independent of arbitrary object effects.
- [rust/tcl-registry/src/native_stock_list/tests.rs](../../../../rust/tcl-registry/src/native_stock_list/tests.rs), `native_stock_list::tests::empty_list_root_provider_requires_generic_normal_result` (linked): Generic empty-provider class closure is independently selected; no compiled pooled object or source freshness grant.

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
