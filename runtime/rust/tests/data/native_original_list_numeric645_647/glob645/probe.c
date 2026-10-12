/* tcl-lsp -- original list numeric key/pattern getter observations.
 * Copyright (C) 2026 James Deucker (bitwisecook).
 * SPDX-License-Identifier: AGPL-3.0-or-later
 * Counted source Strings and genuinely typed integer/Double objects are independent
 * inputs. Each key has only its actual containing List reference, with no observer
 * hold; each argv object has one explicit caller reference before EvalObjv. No
 * getter is inserted between EvalObjv and reached original header snapshots.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#ifdef PROBE_JIM
#include "jim.h"
typedef Jim_Obj Object; typedef Jim_Interp Interpreter;
static Object *new_bytes(Interpreter *i,const unsigned char *b,int n){return Jim_NewStringObj(i,(const char *)b,n);}
static Object *new_wide(Interpreter *i,long long n){return Jim_NewIntObj(i,(jim_wide)n);}
static Object *new_double(Interpreter *i,double n){return Jim_NewDoubleObj(i,n);}
static Object *new_list(Interpreter *i,Object **v,int n){return Jim_NewListObj(i,v,n);}
static void hold(Object *o){Jim_IncrRefCount(o);}
static void release(Interpreter *i,Object *o){Jim_DecrRefCount(i,o);}
static void snapshot(const char *s,Object *o){if(!o){printf("%s\tabsent\n",s);return;}printf("%s\t%s\t%d\t%d\t%lld\n",s,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount,(long long)o->length);}
static void bytes(Interpreter *i,const char *s,Object *o){int n,k;const unsigned char *b=(const unsigned char *)Jim_GetString(o,&n);(void)i;printf("%s\t",s);for(k=0;k<n;k++)printf("%02x",b[k]);printf("\n");}
#else
#include "tcl.h"
typedef Tcl_Obj Object; typedef Tcl_Interp Interpreter;
static Object *new_bytes(Interpreter *i,const unsigned char *b,int n){(void)i;return Tcl_NewStringObj((const char *)b,n);}
static Object *new_wide(Interpreter *i,long long n){(void)i;return Tcl_NewWideIntObj((Tcl_WideInt)n);}
static Object *new_double(Interpreter *i,double n){(void)i;return Tcl_NewDoubleObj(n);}
static Object *new_list(Interpreter *i,Object **v,int n){(void)i;return Tcl_NewListObj(n,v);}
static void hold(Object *o){Tcl_IncrRefCount(o);}
static void release(Interpreter *i,Object *o){(void)i;Tcl_DecrRefCount(o);}
static void snapshot(const char *s,Object *o){if(!o){printf("%s\tabsent\n",s);return;}printf("%s\t%s\t%d\t%lld\t%lld\n",s,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(long long)o->refCount,(long long)o->length);}
static void bytes(Interpreter *i,const char *s,Object *o){
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size n,k;
#else
 int n,k;
#endif
 const unsigned char *b=(const unsigned char *)Tcl_GetStringFromObj(o,&n);(void)i;printf("%s\t",s);for(k=0;k<n;k++)printf("%02x",b[k]);printf("\n");}
#endif

/* kind: 0 counted String; 1 genuine wide integer; 2 genuine Double; 3 absent. */
struct Word { const unsigned char *b; int n,kind; long long integer; double real; };
#define B(s) {(const unsigned char *)(s),(int)(sizeof(s)-1),0,0,0.0}
#define I(n) {NULL,0,1,(n),0.0}
#define D(n) {NULL,0,2,0,(n)}
#define NONE {NULL,0,3,0,0.0}
struct Input { const char *id; int sort,real; const char *extra; int count; struct Word key0,key1,pattern; };
static struct Input cases[] = {
 {"search-integer-empty-invalid-pattern",0,0,NULL,0,NONE,NONE,B("1+0")},
 {"search-integer-empty-wide-pattern",0,0,NULL,0,NONE,NONE,B("4294967296")},
 {"search-integer-one-invalid-key",0,0,NULL,1,B("1+0"),NONE,B("1")},
 {"search-integer-one-unsigned-int-boundary",0,0,NULL,1,B("2147483648"),NONE,B("2147483648")},
 {"search-integer-one-outside-int-width",0,0,NULL,1,B("4294967296"),NONE,B("4294967296")},
 {"search-integer-one-counted-nul-key",0,0,NULL,1,B("1\0tail"),NONE,B("1")},
 {"search-integer-two-first-invalid-key",0,0,NULL,2,B("1+0"),B("1"),B("1")},
 {"search-integer-two-match-before-invalid-key",0,0,NULL,2,B("1"),B("1+0"),B("1")},
 {"search-integer-all-match-before-invalid-key",0,0,"-all",2,B("1"),B("1+0"),B("1")},
 {"search-integer-sorted-match-before-invalid-key",0,0,"-sorted",2,B("1"),B("1+0"),B("1")},
 {"search-real-empty-nan-pattern",0,1,NULL,0,NONE,NONE,B("NaN")},
 {"search-real-one-nan-key",0,1,NULL,1,B("NaN"),NONE,B("1.0")},
 {"search-real-one-counted-nul-key",0,1,NULL,1,B("1.0\0tail"),NONE,B("1.0")},
 {"search-real-two-match-before-invalid-key",0,1,NULL,2,B("1.0"),B("1+0"),B("1.0")},
 {"sort-integer-empty",1,0,NULL,0,NONE,NONE,NONE},
 {"sort-integer-one-invalid-key",1,0,NULL,1,B("1+0"),NONE,NONE},
 {"sort-integer-two-normal",1,0,NULL,2,B("2"),B("1"),NONE},
 {"sort-integer-two-outside-int-width",1,0,NULL,2,B("4294967296"),B("1"),NONE},
 {"sort-integer-two-first-invalid-key",1,0,NULL,2,B("1+0"),B("1"),NONE},
 {"sort-integer-two-second-invalid-key",1,0,NULL,2,B("1"),B("1+0"),NONE},
 {"sort-integer-two-counted-nul-key",1,0,NULL,2,B("1\0tail"),B("2"),NONE},
 {"sort-real-one-nan-key",1,1,NULL,1,B("NaN"),NONE,NONE},
 {"sort-real-two-nan-key",1,1,NULL,2,B("NaN"),B("1.0"),NONE},
 {"sort-real-two-infinities",1,1,NULL,2,B("+Inf"),B("-Inf"),NONE},
 {"sort-real-two-counted-nul-key",1,1,NULL,2,B("1.0\0tail"),B("2.0"),NONE},
 {"sort-real-two-first-invalid-key",1,1,NULL,2,B("1+0"),B("1.0"),NONE},
 {"sort-real-two-second-invalid-key",1,1,NULL,2,B("1.0"),B("1+0"),NONE},
 {"sort-integer-two-typed-wide-key",1,0,NULL,2,I(4294967296LL),B("1"),NONE},
 {"search-integer-one-typed-wide-key-pattern",0,0,NULL,1,I(4294967296LL),NONE,I(4294967296LL)},
 {"sort-real-two-typed-double-key",1,1,NULL,2,D(2.5),B("1.0"),NONE},
 {"search-real-one-typed-double-key-pattern",0,1,NULL,1,D(1.5),NONE,D(1.5)},
 {"sort-real-two-typed-nan-key",1,1,NULL,2,D(NAN),B("1.0"),NONE}
};
static Object *owners[8]; static int ownerc;
static Object *own(Object *o){if(ownerc>=8)exit(66);hold(o);owners[ownerc++]=o;return o;}
static Object *word(Interpreter *i,const char *s){return own(new_bytes(i,(const unsigned char *)s,(int)strlen(s)));}
static Object *make(Interpreter *i,struct Word w){if(w.kind==0)return new_bytes(i,w.b,w.n);if(w.kind==1)return new_wide(i,w.integer);if(w.kind==2)return new_double(i,w.real);return NULL;}
static void input(const char *s,struct Word w){int k;printf("%s\t",s);if(w.kind==3){printf("absent\n");return;}if(w.kind==0){printf("counted-string\t");for(k=0;k<w.n;k++)printf("%02x",w.b[k]);}else if(w.kind==1)printf("typed-wide\t%lld",w.integer);else if(isnan(w.real))printf("typed-double\tNaN");else printf("typed-double\t%.17g",w.real);printf("\n");}
int main(int argc,char **argv){
 Interpreter *i;Object *objects[8],*members[2],*key0=NULL,*key1=NULL,*pattern=NULL,*container,*result;int c,objc=0,code,k;struct Input in;
 if(argc!=2)return 64;c=atoi(argv[1]);if(c<0||c>=32)return 65;in=cases[c];
#ifdef PROBE_JIM
 i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);printf("provider\tJim\n");
#else
 Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();printf("provider\tC-Tcl\n");
#endif
 printf("case\t%d\t%s\noperation\t%s\nmode\t%s\nextra\t%s\n",c,in.id,in.sort?"lsort":"lsearch",in.real?"real":"integer",in.extra?in.extra:"absent");
 input("input-key0",in.key0);input("input-key1",in.key1);input("input-pattern",in.pattern);
 printf("input-roles\tkey-holds=containing-list-only\tadditional-observer-native-holds=0\targv-explicit-caller-hold=1\tinput-list-length=%d\n",in.count);
 if(in.count>0)members[0]=key0=make(i,in.key0);if(in.count>1)members[1]=key1=make(i,in.key1);
 /* Fresh child reference zero becomes exactly the genuine constructor List hold. */
 container=own(new_list(i,in.count?members:NULL,in.count));
 objects[objc++]=word(i,in.sort?"lsort":"lsearch");objects[objc++]=word(i,in.real?"-real":"-integer");if(in.extra)objects[objc++]=word(i,in.extra);objects[objc++]=container;
 if(!in.sort){pattern=own(make(i,in.pattern));objects[objc++]=pattern;}
 snapshot("before-key0",key0);snapshot("before-key1",key1);snapshot("before-pattern",pattern);snapshot("before-container",container);
#ifdef PROBE_JIM
 Jim_SetResultString(i,"PRIOR",5);code=Jim_EvalObjVector(i,objc,objects);result=i->result;
#else
 Tcl_SetObjResult(i,Tcl_NewStringObj("PRIOR",5));code=Tcl_EvalObjv(i,objc,objects,TCL_EVAL_GLOBAL);result=Tcl_GetObjResult(i);
#endif
 printf("code\t%d\n",code);snapshot("reached-key0",key0);snapshot("reached-key1",key1);snapshot("reached-pattern",pattern);snapshot("reached-container",container);snapshot("reached-result",result);
 printf("result-alias\tkey0=%d\tkey1=%d\tpattern=%d\tcontainer=%d\n",key0&&result==key0,key1&&result==key1,pattern&&result==pattern,result==container);
 bytes(i,"result-bytes",result);snapshot("materialised-result",result);snapshot("after-result-getter-key0",key0);snapshot("after-result-getter-key1",key1);snapshot("after-result-getter-pattern",pattern);snapshot("after-result-getter-container",container);
#ifndef PROBE_JIM
 if(code!=TCL_OK){Object *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);Object *ei=Tcl_GetVar2Ex(i,"errorInfo",NULL,TCL_GLOBAL_ONLY);if(ec)bytes(i,"error-code-bytes",ec);if(ei)bytes(i,"error-info-bytes",ei);}
#endif
 for(k=ownerc-1;k>=0;k--)release(i,owners[k]);
#ifdef PROBE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i);Tcl_Finalize();
#endif
 return 0;
}
