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
struct Case { const char *label; const char *option; int option_length; const char *target; int count; };
static const struct Case cases[]={
 {"BASE",NULL,0,NULL,4},
 {"PRIVATE_EXACT","-private",8,NULL,5},
 {"PRIVATE_RAW_ZERO","-private\0\xff",10,NULL,5},
 {"PRIVATE_ENCODED_ZERO","-private\xc0\x80",10,NULL,5},
 {"PRIVATE_PREFIX","-priv",5,NULL,5},
 {"BAD_RAW_ZERO","-bad\xff\0TAIL",10,NULL,5},
 {"MISSING_BAD","-bad\xff\0TAIL",10,"::missing",5},
 {"MISSING_PRIVATE","-private",8,"::missing",5},
 {"MISSING_EXTRA","-private",8,"::missing",6},
 {"MISSING_BASE",NULL,0,"::missing",4},
 {"NO_TARGET",NULL,0,NULL,3}
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
  char label[100],setup_label[110],script[200]; const struct Case *item=&cases[x];
  snprintf(label,sizeof(label),"%s_%s_%s",mode?"COUNTED_SOURCE":"DIRECT",kinds[kind],item->label);
  snprintf(setup_label,sizeof(setup_label),"%s_SETUP",label);
  i=fresh(); if(!i) return 2;
  const char *setup="oo::class create C {variable PUBLIC}; C create O; oo::objdefine O variable OPUBLIC";
  c=source(i,setup,(int)strlen(setup)); result(i,setup_label,c);
#ifndef JIM_PROBE
#if TCL_MAJOR_VERSION >= 9
  if(c==TCL_OK) { const char *private_setup="oo::define C private variable PRIVATE; oo::objdefine O private variable OPRIVATE"; c=source(i,private_setup,(int)strlen(private_setup)); snprintf(setup_label,sizeof(setup_label),"%s_PRIVATE_SETUP",label); result(i,setup_label,c); }
#endif
#endif
  const char *target=item->target?item->target:(kind?"C":"O");
  if(mode==0) {
   input(label,item->option?item->option:"",item->option_length);
   Obj *v[]={string(i,"info",4),string(i,kinds[kind],(int)strlen(kinds[kind])),string(i,"variables",9),string(i,target,(int)strlen(target)),string(i,item->option?item->option:"",item->option_length),string(i,"EXTRA",5)};
   for(int k=0;k<6;k++) retain(v[k]);
   c=invoke(i,item->count,v); result(i,label,c); options(i,label,c);
   for(int k=0;k<6;k++) release(i,v[k]);
  } else {
   int n=snprintf(script,sizeof(script),"info %s variables",kinds[kind]);
   if(item->count>=4) n+=snprintf(script+n,sizeof(script)-n," %s",target);
   if(item->count>=5) { script[n++]=' '; script[n++]='{'; memcpy(script+n,item->option,item->option_length); n+=item->option_length; script[n++]='}'; }
   if(item->count>=6) n+=snprintf(script+n,sizeof(script)-n," EXTRA");
   input(label,script,n); c=source(i,script,n); result(i,label,c); options(i,label,c);
  }
  destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
