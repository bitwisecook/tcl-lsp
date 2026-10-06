#include <stdio.h>
#include <string.h>
#include <stddef.h>
#include <stdlib.h>
static const char *stage;
static void hex(const char *p,size_t n){putchar('"');for(size_t k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);putchar('"');}
#ifdef USE_JIM
#include "jim.h"
static void inventory(Jim_Interp *ip){
 Jim_HashTable *t=&ip->topFramePtr->vars;printf("{\"stage\":\"%s\",\"kind\":\"table\",\"buckets\":%u,\"entries\":%u}\n",stage,t->size,t->used);
 for(unsigned b=0;b<t->size;b++)for(Jim_HashEntry*h=t->table[b];h;h=h->next){Jim_Obj *key=(Jim_Obj*)h->key;Jim_VarVal*v=Jim_GetHashEntryVal(h);int n;const char*s=Jim_GetString(key,&n);printf("{\"stage\":\"%s\",\"kind\":\"variable\",\"name\":",stage);hex(s,n);printf(",\"defined\":%d,\"linked\":%d,\"type\":\"%s\"}\n",v->objPtr!=NULL,v->linkFramePtr!=NULL,v->objPtr&&v->objPtr->typePtr?v->objPtr->typePtr->name:"none");}
 t=&ip->commands;for(unsigned b=0;b<t->size;b++)for(Jim_HashEntry*h=t->table[b];h;h=h->next){Jim_Obj*key=(Jim_Obj*)h->key;Jim_Cmd*c=Jim_GetHashEntryVal(h);int n;const char*s=Jim_GetString(key,&n);printf("{\"stage\":\"%s\",\"kind\":\"command\",\"name\":",stage);hex(s,n);printf(",\"proc\":%d}\n",!!(c->flags&JIM_CMD_ISPROC));}
}
int main(void){Jim_Interp*ip=Jim_CreateInterp();stage="create";inventory(ip);Jim_RegisterCoreCommands(ip);stage="core-registered";inventory(ip);int code=Jim_InitStaticExtensions(ip);printf("{\"stage\":\"extensions-init\",\"kind\":\"completion\",\"code\":%d}\n",code);stage="extensions-init";inventory(ip);Jim_FreeInterp(ip);return code;}
#else
#include "tcl.h"
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void namespace_inventory(Namespace *ns){
 Tcl_HashTable *t;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 t=&ns->varTable;
#else
 t=&ns->varTable.table;
#endif
 printf("{\"stage\":\"%s\",\"kind\":\"namespace\",\"name\":",stage);hex(ns->fullName,strlen(ns->fullName));printf(",\"buckets\":%ld,\"entries\":%ld,\"commands\":%ld}\n",(long)t->numBuckets,(long)t->numEntries,(long)ns->cmdTable.numEntries);
 Tcl_HashSearch q;for(Tcl_HashEntry*h=Tcl_FirstHashEntry(t,&q);h;h=Tcl_NextHashEntry(&q)){Var*v;const char*s;Len n;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 s=Tcl_GetHashKey(t,h);n=strlen(s);v=Tcl_GetHashValue(h);
#else
 s=Tcl_GetStringFromObj(h->key.objPtr,&n);v=(Var*)((char*)h - offsetof(VarInHash,entry));
#endif
 printf("{\"stage\":\"%s\",\"kind\":\"variable\",\"namespace\":",stage);hex(ns->fullName,strlen(ns->fullName));printf(",\"name\":");hex(s,n);printf(",\"flags\":%lu,\"defined\":%d,\"array\":%d,\"linked\":%d",(unsigned long)v->flags,!TclIsVarUndefined(v),!!TclIsVarArray(v),!!TclIsVarLink(v));
 if(!TclIsVarUndefined(v)&&!TclIsVarArray(v)&&!TclIsVarLink(v)){Tcl_Obj*o=v->value.objPtr;printf(",\"type\":\"%s\",\"resident\":",o->typePtr?o->typePtr->name:"none");if(o->bytes)hex(o->bytes,o->length);else printf("null");}
 puts("}");}
 for(Tcl_HashEntry*h=Tcl_FirstHashEntry(&ns->cmdTable,&q);h;h=Tcl_NextHashEntry(&q)){Command*c=Tcl_GetHashValue(h);const char*s=Tcl_GetHashKey(&ns->cmdTable,h);printf("{\"stage\":\"%s\",\"kind\":\"command\",\"namespace\":",stage);hex(ns->fullName,strlen(ns->fullName));printf(",\"name\":");hex(s,strlen(s));printf(",\"proc\":%d}\n",
#if TCL_MAJOR_VERSION >= 9 && TCL_MINOR_VERSION >= 1
 c->objProc2==TclObjInterpProc2
#else
 c->objProc==TclObjInterpProc
#endif
 );}
 for(Tcl_HashEntry*h=Tcl_FirstHashEntry(&ns->childTable,&q);h;h=Tcl_NextHashEntry(&q))namespace_inventory(Tcl_GetHashValue(h));
}
static void inventory(Tcl_Interp*ip){namespace_inventory((Namespace*)Tcl_GetGlobalNamespace(ip));}
int main(int argc,char**argv){Tcl_Interp*ip=Tcl_CreateInterp();stage="create-before-find-executable";inventory(ip);Tcl_DeleteInterp(ip);Tcl_FindExecutable(argv[0]);
 ip=Tcl_CreateInterp();stage="create";inventory(ip);int code=Tcl_Init(ip);printf("{\"stage\":\"init\",\"kind\":\"completion\",\"code\":%d}\n",code);if(code!=TCL_OK){fprintf(stderr,"Init: %s\n",Tcl_GetStringResult(ip));return 2;}stage="init";inventory(ip);Tcl_DeleteInterp(ip);
 ip=Tcl_CreateInterp();Tcl_SetVar2Ex(ip,"argv0",NULL,Tcl_NewStringObj("fixed.tcl",-1),TCL_GLOBAL_ONLY);Tcl_SetVar2Ex(ip,"argc",NULL,Tcl_NewIntObj(0),TCL_GLOBAL_ONLY);Tcl_SetVar2Ex(ip,"argv",NULL,Tcl_NewListObj(0,NULL),TCL_GLOBAL_ONLY);Tcl_SetVar2Ex(ip,"tcl_interactive",NULL,Tcl_NewBooleanObj(0),TCL_GLOBAL_ONLY);stage="main-before-app-init";inventory(ip);code=Tcl_Init(ip);printf("{\"stage\":\"main-after-app-init\",\"kind\":\"completion\",\"code\":%d}\n",code);stage="main-after-app-init";inventory(ip);Tcl_DeleteInterp(ip);return code;}
#endif
