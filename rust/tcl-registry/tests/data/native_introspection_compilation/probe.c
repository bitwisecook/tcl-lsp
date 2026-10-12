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
    Tcl_EvalEx(ip, "namespace eval ::N {}; namespace eval ::source {proc target {} {}; namespace export target}; namespace eval ::N {namespace import ::source::target}", -1, 0);
    const char *cases[] = {
        "namespace current",
        "namespace current extra",
        "namespace origin $left",
        "namespace origin missing",
        "namespace code {puts X}",
        "namespace code $left",
        "namespace code {::namespace inscope }",
        "namespace code {::namespace inscope ::N X}",
        "info level",
        "info level 0",
        "info level $left",
        "info level 2147483648",
        "info level -2147483649",
        "info level invalid",
        "info level 0 extra",
        "array exists a",
        "array exists $left",
        "array exists a(k)",
        "array set a {}",
        "array set ::N::a {}",
        "array set a {k V j W}",
        "array set a {k}",
        "array set a $right",
        "array set $left {}",
        "array unset a",
        "array unset $left",
        "array unset a k",
        "set a scalar; array set a {}",
        "set a scalar; array unset a; set a",
        "array set a {k V}; array unset a; array exists a",
        "array set a {k V}; array unset a k; array names a"
    };
    for (int n = 0; n < 31; ++n) {
        char source[1024];
        snprintf(source, sizeof(source), "proc ::N::p {left right} {%s}", cases[n]);
        int setup = Tcl_EvalEx(ip, source, -1, 0);
        if (setup != TCL_OK) return 2;
        Tcl_Obj *words[3] = {Tcl_NewStringObj("::N::p", -1),
            Tcl_NewStringObj(n == 2 ? "target" : (n == 10 ? "0" : "a"), -1),
            Tcl_NewStringObj("k V j W", -1)};
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
        Proc *proc = TclFindProc((Interp *)ip, "::N::p");
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
        printf("\t%d\t", proc->numCompiledLocals);
        for (CompiledLocal *local = proc->firstLocalPtr; local; local = local->nextPtr) {
            if (local != proc->firstLocalPtr) printf(",");
            hex(local->name, local->nameLength);
        }
        printf("\n");
        for (int i = 0; i < 3; ++i) Tcl_DecrRefCount(words[i]);
    }
    Tcl_DeleteInterp(ip);
    Tcl_Finalize();
    return 0;
}
