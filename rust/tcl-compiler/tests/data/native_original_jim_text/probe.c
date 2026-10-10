#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp; typedef Jim_Obj Obj;
static Interp *fresh(void){Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return Jim_InitStaticExtensions(i)==JIM_OK?i:NULL;}
static int source(Interp *i,const char *p,int direct){(void)direct;return Jim_Eval(i,p);}
static Obj *result(Interp *i){return Jim_GetResult(i);}
static const char *bytes(Obj *o,long *n){int size;const char *p=Jim_GetString(o,&size);*n=size;return p;}
static void destroy(Interp *i){Jim_FreeInterp(i);}
#else
#include "tcl.h"
typedef Tcl_Interp Interp; typedef Tcl_Obj Obj;
static Interp *fresh(void){Interp *i=Tcl_CreateInterp();return Tcl_Init(i)==TCL_OK?i:NULL;}
static int source(Interp *i,const char *p,int direct){return Tcl_EvalEx(i,p,(int)strlen(p),direct?TCL_EVAL_DIRECT:0);}
static Obj *result(Interp *i){return Tcl_GetObjResult(i);}
static const char *bytes(Obj *o,long *n){
#if TCL_MAJOR_VERSION >= 9
Tcl_Size size;
#else
int size;
#endif
const char *p=Tcl_GetStringFromObj(o,&size);*n=(long)size;return p;}
static void destroy(Interp *i){Tcl_DeleteInterp(i);}
#endif
static void hex(const char *p,long n){for(long k=0;k<n;k++)printf("%02x",(unsigned char)p[k]);}
static void row(Interp *i,const char *label,int code){long n;const char *p=bytes(result(i),&n);printf("%s|%d|",label,code);hex(p,n);puts("");}
struct Case{const char *name;const char *source;};
static const struct Case cases[]={
{"UNQUOTED_ESCAPED","set value p\\uD800"},
{"QUOTED_ESCAPED","set value \"p\\uD800\""},
{"EMPTY_QUOTED","set value \"\""},
{"OPAQUE_PROC_HEAD","proc p\\uD800 {argument} {return $argument}; p\\uD800 VALUE"},
{"UNKNOWN_VARIABLE","set value \"$missing\""},
{"UNKNOWN_COMMAND_THEN_LITERAL","missing_command; set value p\\uD800"},
};
int main(int argc,char **argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
Interp *i=fresh();if(!i)return 2;row(i,"VERSION",source(i,"info patchlevel",0));destroy(i);
#ifdef JIM_PROBE
const int modes=1;const char *mode_names[]={"JIM_SOURCE"};
printf("DIRECT_MODE_UNAVAILABLE|0|4a696d20686173206e6f2054434c5f4556414c5f44495245435420726563697065\n");
#else
const int modes=2;const char *mode_names[]={"SCRIPT_CODE_FLAGS_ZERO","DIRECT_FLAG"};
#endif
for(int mode=0;mode<modes;mode++)for(unsigned k=0;k<sizeof(cases)/sizeof(cases[0]);k++){
const struct Case *item=&cases[k];char label[160];i=fresh();if(!i)return 2;
printf("INPUT_%s_%s|0|",mode_names[mode],item->name);hex(item->source,(long)strlen(item->source));puts("");
snprintf(label,sizeof(label),"%s_%s_RESULT",mode_names[mode],item->name);row(i,label,source(i,item->source,mode));
destroy(i);
}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
