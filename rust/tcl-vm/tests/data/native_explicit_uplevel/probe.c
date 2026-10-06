#include "tclInt.h"
#include <stdio.h>
#include <string.h>

#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif

static int restored;
static int explicit_eval(ClientData ignored, Tcl_Interp *interp,
        ProbeSize objc, Tcl_Obj *const objv[]) {
    Interp *ip = (Interp *)interp;
    CallFrame *saved = ip->varFramePtr, *target;
    int selected, code;
    (void)ignored;
    (void)objc;
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    selected = TclGetFrame(interp, Tcl_GetString(objv[1]), &target);
#else
    selected = TclObjGetFrame(interp, objv[1], &target);
#endif
    if (selected == -1) {
        restored = ip->varFramePtr == saved;
        return TCL_ERROR;
    }
    ip->varFramePtr = target;
    code = Tcl_EvalObjEx(interp, objv[2], 0);
    ip->varFramePtr = saved;
    restored = ip->varFramePtr == saved;
    return code;
}

static void hex(const char *bytes, ProbeSize length) {
    ProbeSize k;
    for (k = 0; k < length; k++) printf("%02x", (unsigned char)bytes[k]);
}

int main(void) {
    const char *levels[] = {"#0", "0", "1", "bad", "+0", "99", "-1",
        "1.0", "1\0X", "#0\0X", "#0", "#0", "#0", "#0"};
    const int lengths[] = {2,1,1,3,2,2,2,3,3,4,2,2,2,2};
    int k;
    for (k = 0; k < 14; k++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        Tcl_Obj *level = Tcl_NewStringObj(levels[k], lengths[k]);
        Tcl_Obj *script;
        Tcl_Obj *call[3];
        const char *before, *bytes;
        int resident, code;
        ProbeSize length;
#if TCL_MAJOR_VERSION >= 9
        Tcl_CreateObjCommand2(interp, "explicit_eval", explicit_eval, NULL, NULL);
#else
        Tcl_CreateObjCommand(interp, "explicit_eval", explicit_eval, NULL, NULL);
#endif
        if (Tcl_Eval(interp, "proc p {level script} {set local LOCAL; explicit_eval $level $script}") != TCL_OK)
            return 2;
        if (k == 10) script = Tcl_NewStringObj("error BODY", -1);
        else if (k == 11) script = Tcl_NewStringObj("set prefix BEFORE; set x {", -1);
        else if (k == 12) {
            Tcl_Obj *members[3] = {Tcl_NewStringObj("set",-1),
                Tcl_NewStringObj("marker",-1), Tcl_NewStringObj("VALUE",-1)};
            script = Tcl_NewListObj(3, members);
        } else if (k == 13) script = Tcl_NewStringObj("break", -1);
        else script = Tcl_NewStringObj("list [info level] [info exists local]", -1);
        call[0] = Tcl_NewStringObj("p", -1);
        call[1] = level;
        call[2] = script;
        Tcl_IncrRefCount(call[0]);
        Tcl_IncrRefCount(level);
        Tcl_IncrRefCount(script);
        before = level->typePtr ? level->typePtr->name : "none";
        resident = level->bytes != NULL;
        restored = 0;
        code = Tcl_EvalObjv(interp, 3, call, 0);
        printf("%d\t%s\t%d\t%d\t", k, before, resident, code);
        bytes = Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
        hex(bytes, length);
        printf("\t%s\t%d\t%d\t%d\n",
            level->typePtr ? level->typePtr->name : "none",
            level->bytes != NULL, script->bytes != NULL, restored);
        Tcl_DecrRefCount(script);
        Tcl_DecrRefCount(level);
        Tcl_DecrRefCount(call[0]);
        Tcl_DeleteInterp(interp);
    }
    return 0;
}
