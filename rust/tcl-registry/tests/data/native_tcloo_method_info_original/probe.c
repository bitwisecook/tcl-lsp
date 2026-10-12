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
struct Case { const char *label; int option_count; const char *options[4]; int lengths[4]; const char *target; };
static const struct Case cases[]={
{"BASE",0,{},{},NULL},
{"ALL",1,{"\055\141\154\154"},{4},NULL},
{"ALL_PREFIX",1,{"\055\141"},{2},NULL},
{"LOCALPRIVATE",1,{"\055\154\157\143\141\154\160\162\151\166\141\164\145"},{13},NULL},
{"PRIVATE",1,{"\055\160\162\151\166\141\164\145"},{8},NULL},
{"PRIVATE_PREFIX",1,{"\055\160"},{2},NULL},
{"PRIVATE_RAW_ZERO",1,{"\055\160\162\151\166\141\164\145\000\377"},{10},NULL},
{"PRIVATE_ENCODED_ZERO",1,{"\055\160\162\151\166\141\164\145\300\200"},{10},NULL},
{"AMBIGUOUS",1,{"\055"},{1},NULL},
{"BAD_RAW_ZERO",1,{"\055\142\141\144\377\000\124\101\111\114"},{10},NULL},
{"SCOPE_PUBLIC",2,{"\055\163\143\157\160\145","\160\165\142\154\151\143"},{6,6},NULL},
{"SCOPE_UNEXPORTED",2,{"\055\163\143\157\160\145","\165\156\145\170\160\157\162\164\145\144"},{6,10},NULL},
{"SCOPE_PRIVATE",2,{"\055\163\143\157\160\145","\160\162\151\166\141\164\145"},{6,7},NULL},
{"SCOPE_AMBIGUOUS",2,{"\055\163","\160"},{2,1},NULL},
{"SCOPE_RAW_ZERO",2,{"\055\163\143\157\160\145\000\130","\160\165\142\154\151\143\000\377"},{8,8},NULL},
{"SCOPE_ENCODED_ZERO",2,{"\055\163\143\157\160\145","\160\165\142\154\151\143\300\200"},{6,8},NULL},
{"SCOPE_MISSING",1,{"\055\163\143\157\160\145"},{6},NULL},
{"SCOPE_BAD",2,{"\055\163\143\157\160\145","\055\141\154\154"},{6,4},NULL},
{"SCOPE_LAST",4,{"\055\163\143\157\160\145","\160\165\142\154\151\143","\055\163\143\157\160\145","\165\156\145\170\160\157\162\164\145\144"},{6,6,6,10},NULL},
{"SCOPE_THEN_ALL",3,{"\055\163\143\157\160\145","\165\156\145\170\160\157\162\164\145\144","\055\141\154\154"},{6,10,4},NULL},
{"ALL_THEN_SCOPE",3,{"\055\141\154\154","\055\163\143\157\160\145","\165\156\145\170\160\157\162\164\145\144"},{4,6,10},NULL},
{"LOCAL_THEN_PRIVATE",2,{"\055\154\157\143\141\154\160\162\151\166\141\164\145","\055\160\162\151\166\141\164\145"},{13,8},NULL},
{"PRIVATE_THEN_LOCAL",2,{"\055\160\162\151\166\141\164\145","\055\154\157\143\141\154\160\162\151\166\141\164\145"},{8,13},NULL},
{"MISSING_BAD",1,{"\055\142\141\144\377\000\124\101\111\114"},{10},"::missing"},
{"MISSING_SCOPE",1,{"\055\163\143\157\160\145"},{6},"::missing"}
};
int main(int argc,char **argv) {
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh(); if(!i) return 2; int c=source(i,"info patchlevel",15); result(i,"VERSION",c); destroy(i);
 const char *kinds[]={"object","class"};
 for(int kind=0;kind<2;kind++) for(unsigned x=0;x<sizeof(cases)/sizeof(cases[0]);x++) for(int mode=0;mode<2;mode++) {
  char label[120],detail[160],script[512]; const struct Case *item=&cases[x];
  snprintf(label,sizeof(label),"%s_%s_%s",mode?"COUNTED_SOURCE":"DIRECT",kinds[kind],item->label);
  snprintf(detail,sizeof(detail),"%s_SETUP",label);
  i=fresh(); if(!i) return 2;
  const char *setup="oo::class create B {method inherited {} {}; export inherited}; oo::class create C {superclass B; method Public {} {}; export Public; method hidden {} {}; unexport hidden}; C create O; oo::objdefine O {method OPub {} {}; export OPub; method ohide {} {}; unexport ohide}";
  c=source(i,setup,(int)strlen(setup)); result(i,detail,c);
  if(c!=0) { printf("%s_NOT_ATTEMPTED|%d|OO-setup-failed\n",label,c); destroy(i); continue; }
#ifndef JIM_PROBE
#if TCL_MAJOR_VERSION >= 9
  const char *ps="oo::define C private method Secret {} {}; oo::objdefine O private method OSecret {} {}";
  c=source(i,ps,(int)strlen(ps)); snprintf(detail,sizeof(detail),"%s_PRIVATE_SETUP",label); result(i,detail,c);
  if(c!=0) { printf("%s_NOT_ATTEMPTED|%d|private-setup-failed\n",label,c); destroy(i); continue; }
#endif
#endif
  const char *target=item->target?item->target:(kind?"C":"O");
  if(mode==0) {
   Obj *v[8]; const char *heads[]={"info",kinds[kind],"methods",target};
   for(int k=0;k<4;k++) { v[k]=string(i,heads[k],(int)strlen(heads[k])); retain(v[k]); }
   for(int k=0;k<item->option_count;k++) { snprintf(detail,sizeof(detail),"%s_ARG%d",label,k); input(detail,item->options[k],item->lengths[k]); v[4+k]=string(i,item->options[k],item->lengths[k]); retain(v[4+k]); }
   c=invoke(i,4+item->option_count,v); result(i,label,c); options(i,label,c);
   for(int k=0;k<4+item->option_count;k++) release(i,v[k]);
  } else {
   int n=snprintf(script,sizeof(script),"info %s methods %s",kinds[kind],target);
   for(int k=0;k<item->option_count;k++) { script[n++]=' '; script[n++]='{'; memcpy(script+n,item->options[k],item->lengths[k]); n+=item->lengths[k]; script[n++]='}'; }
   input(label,script,n); c=source(i,script,n); result(i,label,c); options(i,label,c);
  }
  destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
