#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK) return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { return Jim_NewStringObj(i,p,n); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { Jim_DecrRefCount(i,o); }
static int invoke(Interp *i,int n,Obj **v) { return Jim_EvalObjVector(i,n,v); }
static int source(Interp *i,const char *p,int n) { Obj *o=string(i,p,n); retain(o); int c=Jim_EvalObj(i,o); release(i,o); return c; }
static void result(Interp *i,const char *label,int c) { int n; const char *p=Jim_GetString(Jim_GetResult(i),&n); printf("%s|%d|",label,c); for(int k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
static void options(Interp *i,const char *label,int c) { (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-C-return-options-api\n",label,c); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i=Tcl_CreateInterp(); if(Tcl_Init(i)!=TCL_OK) return NULL; return i; }
static Obj *string(Interp *i,const char *p,int n) { (void)i; return Tcl_NewStringObj(p,n); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i,Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i,int n,Obj **v) { return Tcl_EvalObjv(i,n,v,0); }
static int source(Interp *i,const char *p,int n) { return Tcl_EvalEx(i,p,n,0); }
static void result(Interp *i,const char *label,int c) { Count n; const char *p=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n); printf("%s|%d|",label,c); for(Count k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
static void options(Interp *i,const char *label,int c) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
 (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-Tcl_GetReturnOptions\n",label,c);
#else
 Obj *o=Tcl_GetReturnOptions(i,c); retain(o); Count n; const char *p=Tcl_GetStringFromObj(o,&n); printf("%s_OPTIONS|%d|",label,c); for(Count k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); release(i,o);
#endif
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif

static void input(const char *label,const char *p,int n) { printf("%s_INPUT|0|",label); for(int k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); puts(""); }
struct Case { const char *label; const char *bytes; int length; };
static const struct Case cases[]={
{"ONE_ASCII","\157\160\141\161\165\145",6},
{"RAW_ZERO","\157\160\141\161\165\145\000\164\141\151\154",11},
{"ENCODED_ZERO","\157\160\141\161\165\145\300\200\164\141\151\154",12},
{"OPAQUE_FF","\157\160\141\161\165\145\377\164\141\151\154",11},
{"SURROGATE_D800","\157\160\141\161\165\145\355\240\200",9},
{"UNMATCHED_BRACE","\173",1},
{"EMPTY_LIST","",0},
{"FOUR_ELEMENTS","\141\040\142\040\143\040\144",7},
{"BRACED_FF","\173\170\377\175",4},
{"VALID_TWO","\173\175\040\173\162\145\164\165\162\156\040\117\113\175",14},
{"VALID_THREE","\173\175\040\173\162\145\164\165\162\156\040\117\113\175\040\072\072",17},
{"PARAM_FIELDS","\173\173\141\040\142\040\143\175\175\040\173\175",12},
{"PARAM_FIELDS_RAW_ZERO","\173\173\141\040\142\040\143\175\175\040\173\175\000\124\101\111\114",17},
{"PARAM_BODY_RAW_ZERO","\173\173\141\040\142\040\143\175\175\040\173\170\000\124\101\111\114\175",18}
};
static void error_code(Interp *i,const char *label,int c) {
#if !defined(JIM_PROBE) && !(TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4)
 Obj *o=Tcl_GetReturnOptions(i,c), *key=Tcl_NewStringObj("-errorcode",10), *value=NULL;
 retain(o); retain(key); int code=Tcl_DictObjGet(NULL,o,key,&value);
 printf("%s_ERRORCODE|%d|",label,code);
 if(value) { Count n; const char *p=Tcl_GetStringFromObj(value,&n); for(Count k=0;k<n;k++) printf("%02x",(unsigned char)p[k]); }
 else printf("ABSENT"); puts(""); release(i,key); release(i,o);
#else
 (void)i; printf("%s_ERRORCODE_NOT_TESTED|%d|no-selected-return-options-api\n",label,c);
#endif
}
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i) return 2; int c=source(i,"info patchlevel",15); result(i,"VERSION",c); destroy(i);
 for(unsigned x=0;x<sizeof(cases)/sizeof(cases[0]);x++) for(int mode=0;mode<2;mode++) {
  char label[100],script[256]; const struct Case *item=&cases[x];
  snprintf(label,sizeof(label),"%s_%s",mode?"COUNTED_SOURCE":"DIRECT",item->label);
  i=fresh(); if(!i) return 2;
  if(mode==0) {
   Obj *v[2]; v[0]=string(i,"apply",5); v[1]=string(i,item->bytes,item->length); retain(v[0]); retain(v[1]);
   input(label,item->bytes,item->length); c=invoke(i,2,v); result(i,label,c); options(i,label,c); error_code(i,label,c);
   release(i,v[1]); release(i,v[0]);
  } else {
   const char *prefix="apply \""; int n=(int)strlen(prefix); memcpy(script,prefix,n);
   memcpy(script+n,item->bytes,item->length); n+=item->length; script[n++]='"';
   input(label,script,n); c=source(i,script,n); result(i,label,c); options(i,label,c); error_code(i,label,c);
  }
  destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
