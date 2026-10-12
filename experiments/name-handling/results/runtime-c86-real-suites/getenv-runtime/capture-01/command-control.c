#include "tclInt.h"
#include <stdio.h>
#include <string.h>

#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif

static void call(Tcl_Interp *i, const char *label, const char *name, int length) {
    Tcl_Obj *words[2] = {Tcl_NewStringObj("::tcl::clock::getenv", -1), Tcl_NewStringObj(name, length)};
    Tcl_IncrRefCount(words[0]);
    Tcl_IncrRefCount(words[1]);
    int code = Tcl_EvalObjv(i, 2, words, TCL_EVAL_GLOBAL);
    ProbeSize size;
    const unsigned char *bytes = (const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i), &size);
    printf("%s code %d length %lld hex ", label, code, (long long)size);
    for (ProbeSize n = 0; n < size; ++n) printf("%02x", bytes[n]);
    puts("");
    Tcl_DecrRefCount(words[0]);
    Tcl_DecrRefCount(words[1]);
}

int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) {
        fprintf(stderr, "%s\n", Tcl_GetStringResult(i));
        return 1;
    }
    Command *worker = (Command *)Tcl_FindCommand(i, "::tcl::clock::getenv", NULL, TCL_GLOBAL_ONLY);
    printf("present %d null-hook %d\n", worker != NULL, worker && !worker->compileProc);
    if (worker) {
        call(i, "plain", "NH_CLOCK_GETENV_TEST", -1);
        call(i, "missing", "NH_CLOCK_GETENV_MISSING", -1);
        const char zeroName[] = "NH_CLOCK_GETENV_TEST\0suffix";
        call(i, "counted-nul-name", zeroName, sizeof(zeroName) - 1);
        const char rawName[] = "NH_CLOCK_GETENV_\xff";
        call(i, "raw-name", rawName, sizeof(rawName) - 1);
        call(i, "raw-value", "NH_CLOCK_GETENV_RAW_VALUE", -1);
    }
    Tcl_DeleteInterp(i);
    Tcl_Finalize();
    return 0;
}
