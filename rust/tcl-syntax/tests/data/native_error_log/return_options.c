#include "tclInt.h"
#include <stdio.h>

#if TCL_MAJOR_VERSION > 8
typedef Tcl_Size ProbeLength;
#else
typedef int ProbeLength;
#endif

static void hex(const char *bytes, ProbeLength length) {
    ProbeLength index;
    for (index = 0; index < length; ++index) {
        printf("%02x", (unsigned char)bytes[index]);
    }
}

static void observation(Tcl_Interp *interp, const char *label,
        int count, const char *const arguments[]) {
    Tcl_Obj *objects[10];
    Interp *internal = (Interp *)interp;
    int index, code;
    ProbeLength length;
    const char *bytes;
    for (index = 0; index < count; ++index) {
        objects[index] = Tcl_NewStringObj(arguments[index], -1);
        Tcl_IncrRefCount(objects[index]);
    }
    Tcl_ResetResult(interp);
    code = Tcl_EvalObjv(interp, count, objects, TCL_EVAL_DIRECT);
    printf("%s|%d|", label, code);
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    if (internal->returnOpts != NULL) {
        bytes = Tcl_GetStringFromObj(internal->returnOpts, &length);
        hex(bytes, length);
    } else {
        printf("absent");
    }
    printf("|");
    if (internal->errorCode != NULL) {
        bytes = Tcl_GetStringFromObj(internal->errorCode, &length);
        hex(bytes, length);
    } else {
        printf("absent");
    }
#else
    (void)internal;
    (void)bytes;
    (void)length;
    printf("unavailable|unavailable");
#endif
    printf("\n");
    for (index = 0; index < count; ++index) {
        Tcl_DecrRefCount(objects[index]);
    }
}

int main(int argc, char *argv[]) {
    Tcl_Interp *interp;
    const char *error[] = {"error", "BODY"};
    const char *explicit_error[] = {"error", "BODY", "INFO", "CUSTOM"};
    const char *returned[] = {"return", "-code", "error", "-level", "0", "BODY"};
    const char *custom[] = {"return", "-code", "error", "-level", "0",
        "-options", "-custom kept -errorcode CUSTOM", "BODY"};
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    observation(interp, "error", 2, error);
    observation(interp, "explicit-error", 4, explicit_error);
    observation(interp, "return", 6, returned);
    observation(interp, "custom", 8, custom);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
