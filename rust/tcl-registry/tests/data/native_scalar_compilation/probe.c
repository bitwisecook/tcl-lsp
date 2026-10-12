#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>

static void hex(const char *bytes, int length) {
    for (int i = 0; i < length; ++i) printf("%02x", (unsigned char)bytes[i]);
}
int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *ip = Tcl_CreateInterp();
    const char *cases[] = {
        "string equal $left $right", "string e $left $right",
        "string {equal} $left $right", "string equal -nocase $left $right",
        "string length $left", "string len $left", "string \"length\" $left",
        "llength $left", "llength", "llength $left $right",
        "string {*}{length} $left", "string {*}{equal} $left $right",
        "llength {*}{a b}", "string length", "string equal $left"
    };
    for (int n = 0; n < 15; ++n) {
        char source[1024];
        snprintf(source, sizeof(source), "proc p {left right} {%s}", cases[n]);
        int setup = Tcl_EvalEx(ip, source, -1, 0);
        if (setup != TCL_OK) return 2;
        Tcl_Obj *words[3] = {Tcl_NewStringObj("p", -1),
            Tcl_NewStringObj(n >= 7 && n <= 9 ? "a b" : "A\0x", 3),
            Tcl_NewStringObj("A\0y", 3)};
        for (int i = 0; i < 3; ++i) Tcl_IncrRefCount(words[i]);
        int code = Tcl_EvalObjv(ip, 3, words, 0);
        Tcl_Obj *result = Tcl_GetObjResult(ip);
        printf("%d\t%d\t%s\t%d\t%d\t%s\t%d\t%d\t", n, code,
            result->typePtr ? result->typePtr->name : "none",
            result->bytes != NULL, result->refCount,
            words[1]->typePtr ? words[1]->typePtr->name : "none",
            words[1]->bytes != NULL, result == words[1]);
        int length = 0;
#if TCL_MAJOR_VERSION >= 9
        Tcl_Size size = 0;
        const char *bytes = Tcl_GetStringFromObj(result, &size);
        length = (int)size;
#else
        const char *bytes = Tcl_GetStringFromObj(result, &length);
#endif
        hex(bytes, length);
        printf("\t");
        Proc *proc = TclFindProc((Interp *)ip, "p");
        if (proc->bodyPtr->typePtr && !strcmp(proc->bodyPtr->typePtr->name, "bytecode")) {
            ByteCode *bytecode;
#ifdef ByteCodeGetInternalRep
            ByteCodeGetInternalRep(proc->bodyPtr, &tclByteCodeType, bytecode);
#else
            bytecode = (ByteCode *)proc->bodyPtr->internalRep.otherValuePtr;
#endif
            for (int offset = 0; offset < bytecode->numCodeBytes;) {
                const InstructionDesc *instruction = &tclInstructionTable[bytecode->codeStart[offset]];
                if (offset) printf(",");
                printf("%s", instruction->name);
                if (!instruction->numBytes) return 3;
                offset += instruction->numBytes;
            }
        }
        printf("\n");
        for (int i = 0; i < 3; ++i) Tcl_DecrRefCount(words[i]);
    }
    Tcl_DeleteInterp(ip);
    Tcl_Finalize();
    return 0;
}
