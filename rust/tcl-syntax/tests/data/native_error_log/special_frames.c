#include "tcl.h"
#include "tclInt.h"
#include <stdio.h>
static void run(Tcl_Interp *i, const char *label, const char *script, int special) {
 Tcl_CallFrame frame;
 if (special && Tcl_PushCallFrame(i,&frame,Tcl_GetGlobalNamespace(i),0)!=TCL_OK) return;
 int code=Tcl_EvalEx(i,script,-1,0);
 printf("%s|%d|%s\n",label,code,Tcl_GetStringResult(i));
 if(special) Tcl_PopCallFrame(i);
}
int main(void) {
 Tcl_Interp *i=Tcl_CreateInterp();
 run(i,"root","catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",0);
 run(i,"special-shift","catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",1);
 run(i,"special-same","catch {uplevel 0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]",1);
 run(i,"nested-shift","proc p {} {catch {uplevel #0 {error BODY}} m o; list [dict get $o -errorstack] [dict keys $o]}; namespace eval N {p}",0);
 Tcl_DeleteInterp(i);return 0;
}
