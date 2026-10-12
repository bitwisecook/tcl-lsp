#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); return Jim_InitStaticExtensions(i)==JIM_OK?i:NULL; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static Obj *list(Interp *i,int n,Obj **v) { return Jim_NewListObj(i,v,n); }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int eval(Interp *i,const char *p) { return Jim_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Jim_EvalObjVector(i,n,v); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o,long *n) { int length; const char *p=Jim_GetString(o,&length); *n=length; return p; }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); return Tcl_Init(i)==TCL_OK?i:NULL; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static Obj *list(Interp *i,int n,Obj **v) { (void)i; return Tcl_NewListObj(n,v); }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int eval(Interp *i,const char *p) { return Tcl_Eval(i,p); }
static int invoke(Interp *i,int n,Obj **v) { for(int k=0;k<n;k++)hold(v[k]); int c=Tcl_EvalObjv(i,n,v,0); for(int k=0;k<n;k++)release(i,v[k]); return c; }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o,long *n) {
#if TCL_MAJOR_VERSION >= 9
 Tcl_Size length;
#else
 int length;
#endif
 const char *p=Tcl_GetStringFromObj(o,&length); *n=(long)length; return p;
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif
static void hex(const char *p,long n) { for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]); }
static void row(const char *label,int code,Obj *o) { long n; const char *p=bytes(o,&n); printf("%s|%d|",label,code); hex(p,n); puts(""); }
struct Case { const char *name; const char *value; int length; };
static const struct Case cases[]={
 {"PLAIN","ns",2},{"COLON",":ns",3},{"ABSOLUTE","::ns",4},
 {"ABSOLUTE_COLON",":::ns",5},{"EMPTY","",0},
 {"RELATIVE_ZERO",":ns\000suffix",10},{"ABSOLUTE_ZERO","::ns\000suffix",11},
 {"LEADING_ZERO","\000suffix",7},{"UNICODE_COLON",":\303\251",3}
};
static int setup(Interp *i) { return eval(i,"namespace eval ns {}; namespace eval :ns {}; namespace eval \\u00e9 {}; namespace eval :\\u00e9 {}"); }
static void original_case(const struct Case *item,int byte_array) {
 char label[160]; const char *constructor=byte_array?"BYTE_ARRAY":"RAW_STRING";
#ifdef JIM_PROBE
 if(byte_array) { printf("BYTE_ARRAY_%s_NOT_TESTED|no-native-Jim-byte-array-constructor\n",item->name); return; }
#endif
 Interp *i=fresh(); if(!i)return;
 snprintf(label,sizeof(label),"%s_%s_SETUP",constructor,item->name); int code=setup(i); row(label,code,result(i));
 if(code!=0) { destroy(i); return; }
 Obj *namespace_object;
#ifdef JIM_PROBE
 namespace_object=string(i,item->value,item->length);
#else
 namespace_object=byte_array?Tcl_NewByteArrayObj((const unsigned char *)item->value,item->length):string(i,item->value,item->length);
#endif
 hold(namespace_object);
 printf("%s_%s_INPUT|%d|",constructor,item->name,item->length); hex(item->value,item->length); puts("");
 /* These SDK fields are sampled without requesting a string representation. */
 printf("%s_%s_BEFORE|%d|%s\n",constructor,item->name,namespace_object->bytes!=NULL,namespace_object->typePtr?namespace_object->typePtr->name:"none");
 Obj *elements[]={string(i,"",0),string(i,"namespace current",17),namespace_object};
 Obj *lambda=list(i,3,elements); hold(lambda);
 Obj *call[]={string(i,"apply",5),lambda}; code=invoke(i,2,call);
 snprintf(label,sizeof(label),"%s_%s_APPLY",constructor,item->name); row(label,code,result(i));
 /* The original third list element is still held; Apply's getter can have
  * materialised its resident representation. This is sampled only afterwards. */
 printf("%s_%s_AFTER|%d|%s\n",constructor,item->name,namespace_object->bytes!=NULL,namespace_object->typePtr?namespace_object->typePtr->name:"none");
 snprintf(label,sizeof(label),"%s_%s_GETTER",constructor,item->name); row(label,0,namespace_object);
 release(i,lambda); release(i,namespace_object); destroy(i);
}
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i)return 2; int code=eval(i,"info patchlevel"); row("VERSION",code,result(i)); destroy(i);
 for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++) { original_case(&cases[k],0); original_case(&cases[k],1); }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
