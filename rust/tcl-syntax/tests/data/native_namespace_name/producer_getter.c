#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj O; typedef Jim_Interp I; typedef int L;
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define STR(o,n) Jim_GetString(o,n)
#define RES(i) Jim_GetResult(i)
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define DUP(i,o) Jim_DuplicateObj(i,o)
#define EV(i,n,v) Jim_EvalObjVector(i,n,v)
#else
#include "tcl.h"
#include "tclInt.h"
typedef Tcl_Obj O; typedef Tcl_Interp I;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size L;
#else
typedef int L;
#endif
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define STR(o,n) Tcl_GetStringFromObj(o,n)
#define RES(i) Tcl_GetObjResult(i)
#define NEW(i,s,n) Tcl_NewStringObj(s,n)
#define DUP(i,o) Tcl_DuplicateObj(o)
#define EV(i,n,v) Tcl_EvalObjv(i,n,v,0)
#endif
static const char *ty(O *o) {return o->typePtr ? o->typePtr->name : "none";}
static void row(const char *phase,O *o,int code) {
    const char *before=ty(o); int refs=o->refCount;
#ifndef USE_JIM
    printf("{\"phase\":\"%s-primary-descriptor\",\"type\":\"%s\",\"resolved\":%s}\n",phase,before,
        !strcmp(before,"nsName") && o->internalRep.twoPtrValue.ptr1 != NULL ? "true" : "false");
#endif
    L n=0; const unsigned char *p=(const unsigned char *)STR(o,&n);
    printf("{\"phase\":\"%s\",\"code\":%d,\"type_before\":\"%s\",\"refcount_before\":%d,\"bytes\":\"",phase,code,before,refs);
    for(L k=0;k<n;k++)printf("%02x",p[k]);
    printf("\",\"type_after\":\"%s\"}\n",ty(o));
}
static int call(I *i,const char *phase,int n,O **v) {
    for(int k=0;k<n;k++)INC(v[k]);
    int code=EV(i,n,v); row(phase,RES(i),code);
    for(int k=0;k<n;k++)DEC(i,v[k]); return code;
}
static O *word(I *i,const char *s) {return NEW(i,s,-1);}
static O *current(I *i,const char *phase) {
    O *v[]={word(i,"namespace"),word(i,"current")}; call(i,phase,2,v);
    O *result=RES(i); INC(result); return result;
}
static void parent(I *i,O *o,const char *phase) {
    row("operand-before",o,0);
    O *v[]={word(i,"namespace"),word(i,"parent"),o}; call(i,phase,3,v);
    row("operand-after",o,0);
}
int main(void) {
    setvbuf(stdout,NULL,_IONBF,0);
#ifdef USE_JIM
    I *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i);
    if(Jim_InitStaticExtensions(i)!=JIM_OK)return 3;
    O *root=current(i,"root-current"); O *copy=DUP(i,root); row("root-duplicate",copy,0); INC(copy);
    parent(i,copy,"root-parent"); DEC(i,copy); DEC(i,root);
    const char *script="namespace eval ::N {namespace eval q {namespace current}}";
    O *body=word(i,script);INC(body);int code=Jim_EvalObj(i,body);row("nested-current",RES(i),code);DEC(i,body);
    puts("{\"phase\":\"native-nsName-getter\",\"unavailable\":true}"); Jim_FreeInterp(i);
#else
    I *i=Tcl_CreateInterp(); Tcl_CreateNamespace(i,"::a",NULL,NULL);
    O *root=current(i,"root-current"); O *copy=DUP(i,root); row("root-duplicate",copy,0); INC(copy);
    parent(i,copy,"root-parent"); DEC(i,copy); DEC(i,root);
    Tcl_Namespace *n=Tcl_CreateNamespace(i,"::a:",NULL,NULL); Tcl_CallFrame frame;
    if(!n||Tcl_PushCallFrame(i,&frame,n,0)!=TCL_OK)return 4;
    Tcl_Namespace *q=Tcl_CreateNamespace(i,"q",NULL,NULL); Tcl_PopCallFrame(i);
    if(!q||Tcl_PushCallFrame(i,&frame,q,0)!=TCL_OK)return 5;
    O *nested=current(i,"terminal-colon-child-current");
    O *nestedcopy=DUP(i,nested); row("nested-duplicate",nestedcopy,0);INC(nestedcopy);
    printf("{\"phase\":\"duplicate-primary-storage\",\"same\":%s}\n",
        nested->typePtr && !strcmp(ty(nested),"nsName") && nested->internalRep.twoPtrValue.ptr1==nestedcopy->internalRep.twoPtrValue.ptr1 ? "true":"false");
    Tcl_PopCallFrame(i);
    parent(i,nested,"current-object-parent-from-root");
    O *plain=NEW(i,"::a:::q",7);INC(plain);parent(i,plain,"reported-text-parent-from-root");DEC(i,plain);
    Tcl_DeleteNamespace(q);parent(i,nested,"deleted-current-object-parent");parent(i,nestedcopy,"deleted-duplicate-parent");
    if(Tcl_PushCallFrame(i,&frame,n,0)!=TCL_OK)return 6;
    q=Tcl_CreateNamespace(i,"q",NULL,NULL);Tcl_PopCallFrame(i);
    parent(i,nested,"recreated-current-object-parent");
    const char raw[]={':',':','a',0,'z'}; O *rawname=NEW(i,raw,5);INC(rawname);parent(i,rawname,"raw-NUL-parent");DEC(i,rawname);
    const char modified[]={':',':','a',(char)0xc0,(char)0x80,'z'};O *modname=NEW(i,modified,6);INC(modname);parent(i,modname,"modified-NUL-negative-parent");
    O *ba=Tcl_NewByteArrayObj((const unsigned char *)raw,5);INC(ba);parent(i,ba,"pure-ByteArray-negative-parent");
    const char modified_c[]={':',':','a',(char)0xc0,(char)0x80,'z',0};Tcl_CreateNamespace(i,modified_c,NULL,NULL);
    parent(i,modname,"modified-NUL-positive-parent");parent(i,ba,"pure-ByteArray-positive-parent");DEC(i,ba);DEC(i,modname);
    const char ff[]={':',':',(char)0xff,0};Tcl_CreateNamespace(i,ff,NULL,NULL);O *ffname=NEW(i,ff,3);INC(ffname);parent(i,ffname,"raw-FF-positive-parent");DEC(i,ffname);
    Tcl_Namespace *z=Tcl_CreateNamespace(i,"::Z",NULL,NULL);O *zname=word(i,"::Z");INC(zname);
    parent(i,zname,"active-retire-before");
    if(Tcl_PushCallFrame(i,&frame,z,0)!=TCL_OK)return 7;
    Tcl_DeleteNamespace(z);parent(i,zname,"active-retire-cached");
    Tcl_PopCallFrame(i);parent(i,zname,"active-retire-after-pop");DEC(i,zname);
    O *missing=word(i,"::missing");INC(missing);parent(i,missing,"fresh-missing");DEC(i,missing);
    DEC(i,nestedcopy);DEC(i,nested);Tcl_DeleteInterp(i);
#endif
    return 0;
}
