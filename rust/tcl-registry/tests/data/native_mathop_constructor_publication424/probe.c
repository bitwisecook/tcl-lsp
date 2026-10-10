#include <stdio.h>
#include <string.h>
static void hex(const char *s, long n) { for(long k=0;k<n;k++) printf("%02x",(unsigned char)s[k]); }
#ifdef JIM_PROBE
#include "jim.h"
static void query(Jim_Interp *interp,const char *stage,const char *purpose,const char *source) {
 int code=Jim_Eval(interp,source), n;const char *bytes=Jim_GetString(Jim_GetResult(interp),&n);
 printf("WINDOW|%s|%s|%d|",stage,purpose,code);hex(bytes,n);puts("");
}
static void observe(Jim_Interp *interp,const char *stage) {
 query(interp,stage,"operator-plus","info commands ::tcl::mathop::+");
 query(interp,stage,"operator-family","info commands ::tcl::mathop::*");
 query(interp,stage,"namespace-exists","namespace exists ::tcl::mathop");
 query(interp,stage,"namespace-exports","if {[namespace exists ::tcl::mathop]} {namespace eval ::tcl::mathop {namespace export}} else {set marker ABSENT}");
}
int main(void) { Jim_Interp *interp=Jim_CreateInterp();observe(interp,"create-only");Jim_RegisterCoreCommands(interp);observe(interp,"core-registered");int code=Jim_InitStaticExtensions(interp);printf("INITIALISE_STATIC_EXTENSIONS|%d\n",code);observe(interp,"extensions-initialised");Jim_FreeInterp(interp);return 0; }
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static void query(Tcl_Interp *interp,const char *stage,const char *purpose,const char *source) {
 int code=Tcl_EvalEx(interp,source,-1,0);Count n;const char *bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&n);
 printf("WINDOW|%s|%s|%d|",stage,purpose,code);hex(bytes,n);puts("");
}
static void observe(Tcl_Interp *interp,const char *stage) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
 printf("NATIVE_TABLE|%s|not_available_in_selected_public_header\n",stage);
#else
 Tcl_Namespace *ns=Tcl_FindNamespace(interp,"::tcl::mathop",NULL,TCL_GLOBAL_ONLY);
 Tcl_Command plus=Tcl_FindCommand(interp,"::tcl::mathop::+",NULL,TCL_GLOBAL_ONLY);
 printf("NATIVE_TABLE|%s|namespace=%d|plus=%d\n",stage,ns!=NULL,plus!=NULL);
#endif
 query(interp,stage,"operator-plus","info commands ::tcl::mathop::+");
 query(interp,stage,"operator-family","info commands ::tcl::mathop::*");
 query(interp,stage,"namespace-exists","namespace exists ::tcl::mathop");
 query(interp,stage,"namespace-exports","if {[namespace exists ::tcl::mathop]} {namespace eval ::tcl::mathop {namespace export}} else {set marker ABSENT}");
}
int main(int argc,char **argv) { (void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp *interp=Tcl_CreateInterp();observe(interp,"create-only");int code=Tcl_Init(interp);printf("INITIALISE_DISTRIBUTION|%d\n",code);observe(interp,"distribution-initialised");Tcl_DeleteInterp(interp);return 0; }
#endif
