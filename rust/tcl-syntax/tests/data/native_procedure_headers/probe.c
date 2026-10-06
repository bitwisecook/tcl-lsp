#include "tclInt.h"
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
int main(void) {
    const char *parameters[] = {"args", "args\0X", " args \0X", "args", "args", "args", "args", "args"};
    int parameterLengths[] = {4,6,8,4,4,4,4,4};
    const char *bodies[] = {"", "", "", " \0X", "\xff", "\\\n", "\v\f\r\t\n", "\0"};
    int bodyLengths[] = {0,0,0,3,1,2,5,1};
    Tcl_Interp *interp = Tcl_CreateInterp();
    for (int caseIndex=0; caseIndex<8;caseIndex++) {
        Tcl_Obj *words[] = {Tcl_NewStringObj("proc",4),Tcl_NewStringObj("p",1),Tcl_NewStringObj(parameters[caseIndex],parameterLengths[caseIndex]),Tcl_NewStringObj(bodies[caseIndex],bodyLengths[caseIndex])};
        for(int n=0;n<4;n++)Tcl_IncrRefCount(words[n]);
        int code=Tcl_EvalObjv(interp,4,words,TCL_EVAL_DIRECT);
        Command *command=(Command *)Tcl_FindCommand(interp,"p",NULL,TCL_GLOBAL_ONLY);
        printf("case=%d code=%d header=%d\n",caseIndex,code,command!=NULL && command->compileProc!=NULL);
        for(int n=0;n<4;n++)Tcl_DecrRefCount(words[n]);
        Tcl_ResetResult(interp);
    }
    Tcl_DeleteInterp(interp);
    return 0;
}
