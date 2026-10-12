/* tcl-lsp -- original list stride primitive/header observations.
 * Copyright (C) 2026 James Deucker (bitwisecook).
 * SPDX-License-Identifier: AGPL-3.0-or-later
 * Every input byte is counted. The original counted stride word has exactly one explicit argument-owner
 * reference; no additional observer Native reference is inserted.
 * No object getter is inserted between EvalObjv and reached snapshots.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef PROBE_JIM
#include "jim.h"
typedef Jim_Obj Object; typedef Jim_Interp Interpreter;
static Object *new_bytes(Interpreter *i,const unsigned char *b,int n){return Jim_NewStringObj(i,(const char *)b,n);}
static Object *new_int(Interpreter *i,long long n){return Jim_NewIntObj(i,n);}
static Object *new_list(Interpreter *i,Object **v,int n){return Jim_NewListObj(i,v,n);}
static void hold(Object *o){Jim_IncrRefCount(o);}
static void release(Interpreter *i,Object *o){Jim_DecrRefCount(i,o);}
static void snapshot(const char *s,Object *o){printf("%s\t%s\t%d\t%d\t%lld\n",s,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount,(long long)o->length);}
static void bytes(Interpreter *i,const char *s,Object *o){int n,k;const unsigned char *b=(const unsigned char *)Jim_GetString(o,&n);(void)i;printf("%s\t",s);for(k=0;k<n;k++)printf("%02x",b[k]);printf("\n");}
#else
#include "tcl.h"
typedef Tcl_Obj Object; typedef Tcl_Interp Interpreter;
static Object *new_bytes(Interpreter *i,const unsigned char *b,int n){(void)i;return Tcl_NewStringObj((const char *)b,n);}
static Object *new_int(Interpreter *i,long long n){(void)i;return Tcl_NewIntObj((int)n);}
static Object *new_list(Interpreter *i,Object **v,int n){(void)i;return Tcl_NewListObj(n,v);}
static void hold(Object *o){Tcl_IncrRefCount(o);}
static void release(Interpreter *i,Object *o){(void)i;Tcl_DecrRefCount(o);}
static void snapshot(const char *s,Object *o){printf("%s\t%s\t%d\t%lld\t%lld\n",s,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(long long)o->refCount,(long long)o->length);}
static void bytes(Interpreter *i,const char *s,Object *o){
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size n,k;
#else
 int n,k;
#endif
 const unsigned char *b=(const unsigned char *)Tcl_GetStringFromObj(o,&n);(void)i;printf("%s\t",s);for(k=0;k<n;k++)printf("%02x",b[k]);printf("\n");}
#endif

struct Input { const char *id; const unsigned char *b; int n; };
#define B(id,s) {id,(const unsigned char *)(s),(int)(sizeof(s)-1)}
static struct Input cases[] = {
 B("one","1"), B("two","2"), B("zero","0"), B("negative-one","-1"),
 B("unsigned-int-boundary","2147483648"), B("unsigned-int-maximum","4294967295"),
 B("outside-int-width","4294967296"), B("counted-nul-one","1\0tail"),
 B("expression-spelling-is-not-primitive","1+0")
};
static Object *owners[5]; static int ownerc;
static Object *own(Object *o) { if (ownerc >= 5) exit(66); hold(o); owners[ownerc++] = o; return o; }
static Object *word(Interpreter *i,const char *s) { return own(new_bytes(i,(const unsigned char *)s,(int)strlen(s))); }
int main(int argc,char **argv) {
 Interpreter *i; Object *objects[5],*stride,*container,*result; int c,n,sort,objc,code,k;
 if (argc != 2) return 64; c=atoi(argv[1]); if(c<0||c>=18) return 65; sort=c/9; n=c%9;
#ifdef PROBE_JIM
 i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); printf("provider\tJim\n");
#else
 Tcl_FindExecutable(argv[0]); i=Tcl_CreateInterp(); printf("provider\tC-Tcl\n");
#endif
 printf("case\t%d\t%s\noperation\t%s\n",c,cases[n].id,sort?"lsort":"lsearch");
 printf("input-stride\t"); for(k=0;k<cases[n].n;k++) printf("%02x",cases[n].b[k]); printf("\n");
 printf("input-roles\tstride-original-counted-string=1\tstride-explicit-argument-owner=1\tadditional-observer-native-holds=0\tinput-list-length=0\n");
 objects[0]=word(i,sort?"lsort":"lsearch"); objects[1]=word(i,"-stride");
 stride=own(new_bytes(i,cases[n].b,cases[n].n)); objects[2]=stride;
 container=own(new_list(i,NULL,0)); objects[3]=container; objc=4; if(!sort) objects[objc++]=word(i,"B");
 snapshot("before-stride",stride); snapshot("before-container",container);
#ifdef PROBE_JIM
 Jim_SetResultString(i,"PRIOR",5); code=Jim_EvalObjVector(i,objc,objects); result=i->result;
#else
 Tcl_SetObjResult(i,Tcl_NewStringObj("PRIOR",5)); code=Tcl_EvalObjv(i,objc,objects,TCL_EVAL_GLOBAL); result=Tcl_GetObjResult(i);
#endif
 printf("code\t%d\n",code); snapshot("reached-stride",stride); snapshot("reached-container",container); snapshot("reached-result",result);
 printf("result-alias\tstride=%d\tcontainer=%d\n",result==stride,result==container);
 bytes(i,"result-bytes",result); snapshot("materialised-result",result); snapshot("after-result-getter-stride",stride);
#ifndef PROBE_JIM
 if(code != TCL_OK) { Object *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY); Object *ei=Tcl_GetVar2Ex(i,"errorInfo",NULL,TCL_GLOBAL_ONLY); if(ec) bytes(i,"error-code-bytes",ec); if(ei) bytes(i,"error-info-bytes",ei); }
#endif
 bytes(i,"stride-bytes",stride); snapshot("materialised-stride",stride);
 for(k=ownerc-1;k>=0;k--) release(i,owners[k]);
#ifdef PROBE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i); Tcl_Finalize();
#endif
 return 0;
}
