/*
 * tcl-lsp — a language server and toolchain for Tcl
 * Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 * 
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 * 
 * SPDX-License-Identifier: AGPL-3.0-or-later
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

struct Input { const char *id; int command,search,real,count; const char *index; int unique,error; struct Word keys[4],returned; };
static struct Input cases[] = {
 {"stride-integer-no-index",0,0,0,4,NULL,0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-index0",0,0,0,4,"0",0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-index1",0,0,0,4,"1",0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-ascii-no-index",0,0,2,4,NULL,0,0,{B("2"),B("B"),B("1"),B("A")},NONE},
 {"stride-ascii-index1",0,0,2,4,"1",0,0,{B("2"),B("B"),B("1"),B("A")},NONE},
 {"stride-integer-nested-index00",0,0,0,4,"0 0",0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-index-end",0,0,0,4,"end",0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-three-members",0,0,0,3,"0",0,0,{B("2"),B("A"),B("1"),NONE},NONE},
 {"stride-integer-one-invalid-member",0,0,0,1,"0",0,0,{B("1+0"),NONE,NONE,NONE},NONE},
 {"stride-integer-empty",0,0,0,0,"0",0,0,{NONE,NONE,NONE,NONE},NONE},
 {"stride-integer-index0-counted-nul",0,0,0,4,"0",0,0,{B("2\0tail"),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-index0-typed-wide",0,0,0,4,"0",0,0,{I(4294967296LL),B("A"),B("1"),B("B")},NONE},
 {"stride-integer-index0-unique",0,0,0,4,"0",1,0,{B("1"),B("A"),B("1"),B("B")},NONE},
 {"stride-real-index0-typed-double",0,0,1,4,"0",0,0,{D(2.5),B("A"),D(1.5),B("B")},NONE},
 {"stride-search-integer-exact-index0",0,1,0,4,"0",0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"stride-search-ascii-exact-no-index",0,1,2,4,NULL,0,0,{B("2"),B("A"),B("1"),B("B")},NONE},
 {"command-typed-negative-wide",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},I(-1)},
 {"command-typed-outside-int-wide",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},I(4294967296LL)},
 {"command-typed-unsigned-int-boundary",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},I(2147483648LL)},
 {"command-string-negative",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},B("-1")},
 {"command-string-outside-int-wide",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},B("4294967296")},
 {"command-string-expression-looking",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},B("1+0")},
 {"command-string-counted-nul",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},B("1\0tail")},
 {"command-string-real",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},B("1.0")},
 {"command-typed-exact-double",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},D(1.0)},
 {"command-typed-fractional-double",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},D(1.5)},
 {"command-typed-nan",1,0,0,2,NULL,0,0,{B("B"),B("A"),NONE,NONE},D(NAN)},
 {"command-original-guest-completion",1,0,0,2,NULL,0,1,{B("B"),B("A"),NONE,NONE},B("CALLBACK")},
 {"command-empty-unused-invalid-result",1,0,0,0,NULL,0,0,{NONE,NONE,NONE,NONE},B("1+0")},
 {"command-one-unused-invalid-result",1,0,0,1,NULL,0,0,{B("B"),NONE,NONE,NONE},B("1+0")},
 {"command-stride-whole-group",2,0,0,4,NULL,0,0,{B("2"),B("B"),B("1"),B("A")},I(-1)},
 {"command-stride-index1",2,0,0,4,"1",0,0,{B("2"),B("B"),B("1"),B("A")},I(-1)}
};
static Object *owners[24];static int ownerc,callback_count,callback_error;static Object *callback_return;
static Object *own(Object *o){if(ownerc>=24)exit(66);hold(o);owners[ownerc++]=o;return o;}
static Object *word(Interpreter *i,const char *s){return own(new_bytes(i,(const unsigned char *)s,(int)strlen(s)));}
static Object *make(Interpreter *i,struct Word w){if(w.kind==0)return new_bytes(i,w.b,w.n);if(w.kind==1)return new_wide(i,w.integer);if(w.kind==2)return new_double(i,w.real);return NULL;}
static void input(const char *s,struct Word w){int k;printf("%s\t",s);if(w.kind==3){printf("absent\n");return;}if(w.kind==0){printf("counted-string\t");for(k=0;k<w.n;k++)printf("%02x",w.b[k]);}else if(w.kind==1)printf("typed-wide\t%lld",w.integer);else if(isnan(w.real))printf("typed-double\tNaN");else printf("typed-double\t%.17g",w.real);printf("\n");}
#ifdef PROBE_JIM
static int compare(Interpreter *i,int objc,Object *const *objv){
#else
static int compare(ClientData unused,Interpreter *i,int objc,Object *const *objv){(void)unused;
#endif
 char label[64];int call=callback_count++;if(objc!=3)return 1;
 snprintf(label,sizeof(label),"callback-%d-left-before",call);snapshot(label,objv[1]);snprintf(label,sizeof(label),"callback-%d-right-before",call);snapshot(label,objv[2]);snprintf(label,sizeof(label),"callback-%d-return-before",call);snapshot(label,callback_return);
 snprintf(label,sizeof(label),"callback-%d-left-bytes",call);bytes(i,label,objv[1]);snprintf(label,sizeof(label),"callback-%d-right-bytes",call);bytes(i,label,objv[2]);
#ifdef PROBE_JIM
 Jim_SetResult(i,callback_return);
#else
 Tcl_SetObjResult(i,callback_return);
#endif
 return callback_error?1:0;
}
int main(int argc,char **argv){
 Interpreter *i;Object *objects[24],*members[4],*keys[4]={NULL,NULL,NULL,NULL},*container,*result,*prefix=NULL,*pattern=NULL;int c,objc=0,code,k;char label[64];struct Input in;
 if(argc!=2)return 64;c=atoi(argv[1]);if(c<0||c>=32)return 65;in=cases[c];callback_error=in.error;
#ifdef PROBE_JIM
 i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"r2286cmp",compare,NULL,NULL);printf("provider\tJim\n");
#else
 Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();Tcl_CreateObjCommand(i,"r2286cmp",compare,NULL,NULL);printf("provider\tC-Tcl\n");
#endif
 printf("case\t%d\t%s\noperation\t%s\nmode\t%s\nindex\t%s\n",c,in.id,in.search?"lsearch":"lsort",in.command?"command":in.real==2?"ascii":in.real?"real":"integer",in.index?in.index:"absent");
 printf("input-roles\tkey-holds=sole-containing-list\tadditional-key-observer-holds=0\targv-caller-hold=1\treturned-original-explicit-hold=%d\tflat-length=%d\n",in.command?1:0,in.count);
 for(k=0;k<4;k++){snprintf(label,sizeof(label),"input-key%d",k);input(label,in.keys[k]);if(k<in.count)members[k]=keys[k]=make(i,in.keys[k]);}
 input("input-callback-return",in.returned);container=own(new_list(i,in.count?members:NULL,in.count));objects[objc++]=word(i,in.search?"lsearch":"lsort");
 if(in.command){Object *command_member=new_bytes(i,(const unsigned char *)"r2286cmp",8);prefix=own(new_list(i,&command_member,1));callback_return=own(make(i,in.returned));objects[objc++]=word(i,"-command");objects[objc++]=prefix;}
 else if(!(in.search&&in.real==2))objects[objc++]=word(i,in.real==2?"-ascii":in.real?"-real":"-integer");
 if(!in.command||in.command==2){objects[objc++]=word(i,"-stride");objects[objc++]=word(i,"2");}
 if(in.search)objects[objc++]=word(i,"-exact");if(in.index){objects[objc++]=word(i,"-index");objects[objc++]=word(i,in.index);}if(in.unique)objects[objc++]=word(i,"-unique");objects[objc++]=container;if(in.search){pattern=word(i,"1");objects[objc++]=pattern;}
 for(k=0;k<4;k++){snprintf(label,sizeof(label),"before-key%d",k);snapshot(label,keys[k]);}snapshot("before-container",container);snapshot("before-prefix",prefix);snapshot("before-callback-return",callback_return);snapshot("before-pattern",pattern);
#ifdef PROBE_JIM
 Jim_SetResultString(i,"PRIOR",5);code=Jim_EvalObjVector(i,objc,objects);result=i->result;
#else
 Tcl_SetObjResult(i,Tcl_NewStringObj("PRIOR",5));code=Tcl_EvalObjv(i,objc,objects,TCL_EVAL_GLOBAL);result=Tcl_GetObjResult(i);
#endif
 printf("code\t%d\ncallback-count\t%d\n",code,callback_count);for(k=0;k<4;k++){snprintf(label,sizeof(label),"reached-key%d",k);snapshot(label,keys[k]);}snapshot("reached-container",container);snapshot("reached-prefix",prefix);snapshot("reached-callback-return",callback_return);snapshot("reached-pattern",pattern);snapshot("reached-result",result);printf("result-alias\tcontainer=%d\tcallback-return=%d\n",result==container,callback_return&&result==callback_return);
 bytes(i,"result-bytes",result);snapshot("materialised-result",result);snapshot("after-result-getter-container",container);snapshot("after-result-getter-callback-return",callback_return);
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
