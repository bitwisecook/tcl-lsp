#include "tclInt.h"
#include "tclCompile.h"
#include <stddef.h>
#include <stdio.h>
#include <string.h>

static Tcl_Obj *error_info_cell(Interp *ip) {
    Tcl_Obj *key = Tcl_NewStringObj("errorInfo", -1);
    Tcl_IncrRefCount(key);
    Tcl_HashEntry *entry = Tcl_FindHashEntry(&ip->globalNsPtr->varTable.table, key);
    Tcl_DecrRefCount(key);
    if (!entry) return NULL;
    Var *variable = (Var *)((char *)entry - offsetof(VarInHash, entry));
    return TclIsVarScalar(variable) ? variable->value.objPtr : NULL;
}
static void observe(Tcl_Interp *interp, ByteCode *code, Tcl_Obj *message,
        int which, const char *window) {
    Interp *ip = (Interp *)interp;
    int global = 0;
    for (int bucket = 0; bucket < ip->literalTable.numBuckets; bucket++) {
        for (LiteralEntry *entry = ip->literalTable.buckets[bucket]; entry;
                entry = entry->nextPtr) {
            global += entry->objPtr == message;
        }
    }
    Tcl_Obj *key = Tcl_NewStringObj("-errorinfo", -1), *member = NULL;
    Tcl_IncrRefCount(key);
    Tcl_DictObjGet(NULL, code->objArrayPtr[1], key, &member);
    Tcl_DecrRefCount(key);
    printf("%d\t%s\t%d\t%d\t%d\t%d\t%d\t%d\n", which, window,
            message->refCount, global, member == message,
            error_info_cell(ip) == message, ip->errorInfo == message,
            Tcl_GetObjResult(interp) == message);
}
int main(void) {
    const char *source[] = {
        "expr {$x ? (1/0) : (1+2)}",
        "catch {expr {1/0}} result; return $result"
    };
    for (int which = 0; which < 2; which++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        Tcl_Obj *definition[] = {Tcl_NewStringObj("proc", -1),
            Tcl_NewStringObj("p", -1), Tcl_NewStringObj("x", -1),
            Tcl_NewStringObj(source[which], -1)};
        for (int n = 0; n < 4; n++) Tcl_IncrRefCount(definition[n]);
        if (Tcl_EvalObjv(interp, 4, definition, 0) != TCL_OK) return 1;
        Proc *procedure = TclFindProc((Interp *)interp, "p");
        if (TclProcCompileProc(interp, procedure, procedure->bodyPtr,
                ((Interp *)interp)->globalNsPtr, "body of procedure", "p") != TCL_OK) return 2;
        ByteCode *code = (ByteCode *)procedure->bodyPtr->internalRep.twoPtrValue.ptr1;
        Tcl_Obj *message = code->objArrayPtr[0];
        observe(interp, code, message, which, "compiled");
        Tcl_Obj *call[] = {Tcl_NewStringObj("p", -1), Tcl_NewStringObj("3", -1)};
        for (int n = 0; n < 2; n++) Tcl_IncrRefCount(call[n]);
        int completion = Tcl_EvalObjv(interp, 2, call, 0);
        if (completion != (which ? TCL_OK : TCL_ERROR)) return 3;
        observe(interp, code, message, which, "executed");
        Tcl_ResetResult(interp);
        observe(interp, code, message, which, "reset");
        for (int n = 0; n < 2; n++) Tcl_DecrRefCount(call[n]);
        for (int n = 0; n < 4; n++) Tcl_DecrRefCount(definition[n]);
        Tcl_DeleteInterp(interp);
    }
    return 0;
}
