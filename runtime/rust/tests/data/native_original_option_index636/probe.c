/* tcl-lsp -- original -index option path/header observations.
 * Copyright (C) 2026 James Deucker (bitwisecook).
 * SPDX-License-Identifier: AGPL-3.0-or-later
 * Every input byte is counted. Observed original children have one explicit
 * independent observer reference in addition to genuine containing Lists.
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
struct Input{const unsigned char *b;int n;};
#define B(s) {(const unsigned char *)(s),(int)(sizeof(s)-1)}
struct Case{const char *id;struct Input first,last;int path_kind,nested,empty,repeat_option;};
static struct Case cases[]={
 {"original-string-single",B("0"),B(""),0,0,0,0},
 {"original-list-string-member",B("0"),B(""),1,0,0,0},
 {"original-list-native-int-member",B("0"),B(""),2,0,0,0},
 {"original-list-end",B("end"),B(""),1,0,0,0},
 {"original-list-end-minus-one",B("end-1"),B(""),1,0,0,0},
 {"original-list-arithmetic",B("0+0"),B(""),1,0,0,0},
 {"original-list-counted-nul-integer",B("0\0tail"),B(""),1,0,0,0},
 {"original-list-counted-nul-end",B("end-1\0tail"),B(""),1,0,0,0},
 {"original-list-nested-path",B("0"),B("1"),3,1,0,0},
 {"original-list-first-invalid",B("bad"),B("0"),3,1,0,0},
 {"original-list-last-invalid",B("0"),B("bad"),3,1,0,0},
 {"original-list-encoded-width-low",B("1073741823"),B(""),1,0,1,0},
 {"original-list-encoded-width-high",B("1073741824"),B(""),1,0,1,0},
 {"original-list-native-wide-width",B("2147483648"),B(""),1,0,1,0},
 {"original-list-same-member-two-slots",B("0"),B(""),4,1,0,0},
 {"two-original-index-options",B("0"),B("end"),1,0,0,1}
};
static Object *owners[40];static int ownerc;
static Object *own(Object *o){if(ownerc>=(int)(sizeof(owners)/sizeof(owners[0])))exit(66);hold(o);owners[ownerc++]=o;return o;}
static Object *word(Interpreter *i,const char *s){return own(new_bytes(i,(const unsigned char *)s,(int)strlen(s)));}
static void input(const char *s,struct Input x){int k;printf("%s\t",s);for(k=0;k<x.n;k++)printf("%02x",x.b[k]);printf("\n");}
int main(int argc,char **argv){Interpreter *i;Object *objects[7],*path,*path2=NULL,*p0=NULL,*p1=NULL,*keys[4],*inner[2],*rows[2],*container,*result;int c,sort,objc,code,k,n;
 if(argc!=2)return 64;c=atoi(argv[1]);if(c<0||c>=32)return 65;sort=c/16;n=c%16;
#ifdef PROBE_JIM
 i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);printf("provider\tJim\n");
#else
 Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();printf("provider\tC-Tcl\n");
#endif
 printf("case\t%d\t%s\noperation\t%s\n",c,cases[n].id,sort?"lsort":"lsearch");input("input-first",cases[n].first);input("input-last",cases[n].last);
 printf("input-roles\tpath-kind=%d\tnested=%d\tempty=%d\trepeat-option=%d\tobserved-child-extra-owner=1\n",cases[n].path_kind,cases[n].nested,cases[n].empty,cases[n].repeat_option);
 if(cases[n].path_kind==0)path=own(new_bytes(i,cases[n].first.b,cases[n].first.n));
 else{Object *pv[2];p0=own(cases[n].path_kind==2?new_int(i,0):new_bytes(i,cases[n].first.b,cases[n].first.n));pv[0]=p0;k=1;if(cases[n].path_kind==3){p1=own(new_bytes(i,cases[n].last.b,cases[n].last.n));pv[k++]=p1;}if(cases[n].path_kind==4){p1=p0;pv[k++]=p1;}path=own(new_list(i,pv,k));}
 if(cases[n].repeat_option){Object *pv[1];p1=own(new_bytes(i,cases[n].last.b,cases[n].last.n));pv[0]=p1;path2=own(new_list(i,pv,1));}
 keys[0]=word(i,"B");keys[1]=word(i,"A");keys[2]=word(i,"A");keys[3]=word(i,"B");
 inner[0]=own(new_list(i,&keys[0],2));inner[1]=own(new_list(i,&keys[2],2));
 if(cases[n].nested){Object *rv[2];rv[0]=inner[0];rv[1]=word(i,"tail0");rows[0]=own(new_list(i,rv,2));rv[0]=inner[1];rv[1]=word(i,"tail1");rows[1]=own(new_list(i,rv,2));}else{rows[0]=inner[0];rows[1]=inner[1];}
 container=own(new_list(i,rows,cases[n].empty?0:2));objects[0]=word(i,sort?"lsort":"lsearch");objects[1]=word(i,"-index");objects[2]=path;objc=3;
 if(path2){objects[objc++]=word(i,"-index");objects[objc++]=path2;}objects[objc++]=container;if(!sort)objects[objc++]=word(i,"B");
 snapshot("before-path",path);if(path2)snapshot("before-path2",path2);if(p0)snapshot("before-path-member0",p0);if(p1)snapshot("before-path-member1",p1);snapshot("before-container",container);for(k=0;k<4;k++){char s[40];sprintf(s,"before-key%d",k);snapshot(s,keys[k]);}
#ifdef PROBE_JIM
 Jim_SetResultString(i,"PRIOR",5);code=Jim_EvalObjVector(i,objc,objects);result=i->result;
#else
 Tcl_SetObjResult(i,Tcl_NewStringObj("PRIOR",5));code=Tcl_EvalObjv(i,objc,objects,TCL_EVAL_GLOBAL);result=Tcl_GetObjResult(i);
#endif
 printf("code\t%d\n",code);snapshot("reached-path",path);if(path2)snapshot("reached-path2",path2);if(p0)snapshot("reached-path-member0",p0);if(p1)snapshot("reached-path-member1",p1);snapshot("reached-container",container);for(k=0;k<4;k++){char s[40];sprintf(s,"reached-key%d",k);snapshot(s,keys[k]);}snapshot("reached-result",result);printf("result-alias\tcontainer=%d\tpath=%d\n",result==container,result==path);bytes(i,"result-bytes",result);snapshot("materialised-result",result);
#ifndef PROBE_JIM
 if(code!=TCL_OK){Object *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);Object *ei=Tcl_GetVar2Ex(i,"errorInfo",NULL,TCL_GLOBAL_ONLY);if(ec)bytes(i,"error-code-bytes",ec);if(ei)bytes(i,"error-info-bytes",ei);}
#endif
 bytes(i,"path-bytes",path);if(p0)bytes(i,"path-member0-bytes",p0);if(p1)bytes(i,"path-member1-bytes",p1);for(k=ownerc-1;k>=0;k--)release(i,owners[k]);
#ifdef PROBE_JIM
 Jim_FreeInterp(i);
#else
 Tcl_DeleteInterp(i);Tcl_Finalize();
#endif
 return 0;
}
