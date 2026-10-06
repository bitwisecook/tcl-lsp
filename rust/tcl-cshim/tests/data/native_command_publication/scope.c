#include <tcl.h>
#include <stdio.h>
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
/* Exact declarations from pinned C8.4 tclIntDecls.h (native exported entries). */
extern Tcl_Namespace *Tcl_GetCurrentNamespace(Tcl_Interp *);
extern void Tcl_GetCommandFullName(Tcl_Interp *, Tcl_Command, Tcl_Obj *);
#endif
static int answer(ClientData data, Tcl_Interp *interp, int count, Tcl_Obj *const values[]) {
    (void)count; (void)values; Tcl_SetObjResult(interp, Tcl_NewStringObj((const char *)data,-1)); return TCL_OK;
}
static void report(Tcl_Interp *interp, const char *label, const char *script) {
    int status=Tcl_Eval(interp,script);
    printf("%s status=%d result=%s\n",label,status,Tcl_GetStringResult(interp));
}
static int publish(ClientData data, Tcl_Interp *interp, int count, Tcl_Obj *const values[]) {
    (void)data;(void)count;(void)values;
    Tcl_Namespace *current=Tcl_GetCurrentNamespace(interp);
    printf("current=%s\n",current->fullName);
    Tcl_Command simple=Tcl_CreateObjCommand(interp,"p",answer,(ClientData)"GLOBAL",NULL);
    Tcl_Command qualified=Tcl_CreateObjCommand(interp,"q::p",answer,(ClientData)"QUALIFIED",NULL);
    Tcl_Obj *name=Tcl_NewObj(); Tcl_IncrRefCount(name);
    Tcl_GetCommandFullName(interp,simple,name); printf("unqualified-full=%s\n",Tcl_GetString(name));
    Tcl_SetObjLength(name,0); Tcl_GetCommandFullName(interp,qualified,name); printf("qualified-full=%s\n",Tcl_GetString(name));
    Tcl_DecrRefCount(name);
    Tcl_CmdInfo info; int found=Tcl_GetCommandInfo(interp,"q::p",&info);
    printf("qualified-local-token=%d\n",found && info.objClientData==(ClientData)"QUALIFIED");
    report(interp,"local-simple","p"); report(interp,"local-qualified","q::p");
    Tcl_ResetResult(interp); return TCL_OK;
}
int main(void) {
    Tcl_FindExecutable("c-api-terminal-colon"); Tcl_Interp *interp=Tcl_CreateInterp();
    Tcl_CreateObjCommand(interp,"publish",publish,NULL,NULL);
    report(interp,"publication","namespace eval {::a:} {::publish}");
    report(interp,"global-simple","::p");
    report(interp,"global-a-q","::a::q::p");
    report(interp,"global-a-colon-q","::a:::q::p");
    Tcl_DeleteInterp(interp); Tcl_Finalize(); return 0;
}
