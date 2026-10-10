#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
static void report(Jim_Interp *i,const char *tag,int code) { int n; const unsigned char *s=(const unsigned char *)Jim_GetString(Jim_GetResult(i),&n); printf("%s|%d|",tag,code); for(int k=0;k<n;k++)printf("%02x",s[k]); puts(""); }
int main(int argc,char **argv) {
 if(argc!=2)return 2; int selected=atoi(argv[1]); if(selected<0 || selected>0)return 2;
 Jim_Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK)return 3;
 report(i,"VERSION",Jim_Eval(i,"info patchlevel"));
 report(i,"AVAILABILITY",Jim_Eval(i,"list [llength [info commands encoding]] [catch {encoding convertto utf-8 VALUE} r] $r"));
 Jim_FreeInterp(i); return 0;
}
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static const unsigned char eacute[]={0xc3,0xa9};
static const unsigned char modified_zero[]={0xc0,0x80};
static const unsigned char literal_zero[]={0};
static const unsigned char ff[]={0xff};
static const unsigned char pair[]={0xed,0xa0,0xbd,0xed,0xb8,0x80};
static const unsigned char high[]={0xed,0xa0,0xbd};
static const unsigned char astral[]={0xf0,0x9f,0x98,0x80};
static const unsigned char prefixed_high[]={0xc3,0xa9,0xed,0xa0,0xbd};
static const struct Input { const char *name;const unsigned char *bytes;int length;int binary; } inputs[]={
 {"string-eacute-single-high-surrogate",prefixed_high,5,0}
};
static const char *type(Tcl_Obj *o) { return o->typePtr ? o->typePtr->name : "none"; }
static void hex(const unsigned char *s,Count n) { for(Count k=0;k<n;k++)printf("%02x",s[k]); }
int main(int argc,char **argv) {
 if(argc!=2)return 2; int selected=atoi(argv[1]); if(selected<0 || selected>0)return 2;
 Tcl_FindExecutable(argv[0]); Tcl_Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK)return 3;
 int version=Tcl_EvalEx(i,"info patchlevel",15,0);Count n;const unsigned char *s=(const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);printf("VERSION|%d|",version);hex(s,n);puts("");
 const struct Input *input=&inputs[selected];
 Tcl_Obj *words[4]={Tcl_NewStringObj("encoding",-1),Tcl_NewStringObj("convertto",-1),Tcl_NewStringObj("utf-8",-1),input->binary ? Tcl_NewByteArrayObj(input->bytes,input->length) : Tcl_NewStringObj((const char *)input->bytes,input->length)};
 for(int k=0;k<4;k++)Tcl_IncrRefCount(words[k]);
 printf("INPUT|%s|%s|%d|",input->name,type(words[3]),words[3]->bytes!=NULL);hex(input->bytes,input->length);puts("");
 int code=Tcl_EvalObjv(i,4,words,TCL_EVAL_GLOBAL); Tcl_Obj *result=Tcl_GetObjResult(i);Tcl_IncrRefCount(result);
 printf("RETURN|%d|%s|%d\n",code,type(result),result->bytes!=NULL);
 printf("INPUT_AFTER|%s|%d\n",type(words[3]),words[3]->bytes!=NULL);
 if(code==TCL_OK) { s=Tcl_GetByteArrayFromObj(result,&n);printf("RAW|%lld|",(long long)n);hex(s,n);puts(""); }
 else { s=(const unsigned char *)Tcl_GetStringFromObj(result,&n);printf("MESSAGE|");hex(s,n);puts("");Tcl_Obj *ec=Tcl_GetVar2Ex(i,"errorCode",NULL,TCL_GLOBAL_ONLY);if(ec) { s=(const unsigned char *)Tcl_GetStringFromObj(ec,&n);printf("ERROR_CODE|");hex(s,n);puts(""); } }
 Tcl_DecrRefCount(result);for(int k=0;k<4;k++)Tcl_DecrRefCount(words[k]);Tcl_DeleteInterp(i);return 0;
}
#endif
