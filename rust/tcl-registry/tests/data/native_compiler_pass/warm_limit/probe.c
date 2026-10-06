#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
static int calls;
extern int __real_TclCompileArraySetCmd(Tcl_Interp *,Tcl_Parse *,Command *,CompileEnv *);
int __wrap_TclCompileArraySetCmd(Tcl_Interp *i,Tcl_Parse *p,Command *c,CompileEnv *e) {
    calls++;
    return __real_TclCompileArraySetCmd(i,p,c,e);
}
int main(void) {
    Tcl_FindExecutable("probe");
    Tcl_Interp *i=Tcl_CreateInterp();
    int code=Tcl_Eval(i,"proc p {} {array set a {k V}}; p");
    Command *cmd=(Command *)Tcl_FindCommand(i,"p",NULL,TCL_GLOBAL_ONLY);
    if(code!=TCL_OK || cmd==NULL) return 2;
#if TCL_MAJOR_VERSION >= 9 && TCL_MINOR_VERSION >= 1
    Proc *proc=(Proc *)cmd->objClientData2;
#else
    Proc *proc=(Proc *)cmd->objClientData;
#endif
    int first=calls, locals=proc->numCompiledLocals;
    void *bytecode=proc->bodyPtr->internalRep.twoPtrValue.ptr1;
    Tcl_LimitSetCommands(i,1000000);
    Tcl_LimitTypeSet(i,TCL_LIMIT_COMMANDS);
    code=Tcl_Eval(i,"p");
    printf("%d\t%d\t%d\t%d\t%d\t%d\n",code,first,calls,locals,proc->numCompiledLocals,bytecode==proc->bodyPtr->internalRep.twoPtrValue.ptr1);
    Tcl_DeleteInterp(i);Tcl_Finalize();return code==TCL_OK?0:3;
}
