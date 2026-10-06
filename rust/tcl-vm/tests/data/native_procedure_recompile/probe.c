#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tclInt.h"

#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ObjSize;
#else
typedef int ObjSize;
#endif

typedef struct State {
    int mode;
    int trigger;
    int nested;
    Proc *initial_proc;
    Tcl_Obj *initial_body;
    Tcl_Obj *initial_default;
    Tcl_CmdInfo original_set;
    Tcl_Obj *key;
} State;

static Proc *current_proc(Tcl_Interp *interp) {
    Proc *proc = TclFindProc((Interp *)interp, "p");
    if (!proc) exit(30);
    return proc;
}

static void sample(Tcl_Interp *interp, State *state, const char *window) {
    Proc *proc = current_proc(interp);
    Tcl_Obj *body = proc->bodyPtr;
    Tcl_Obj *def = proc->firstLocalPtr->defValuePtr;
    CallFrame *frame_ptr = ((Interp *)interp)->varFramePtr;
    Proc *frame = frame_ptr ? frame_ptr->procPtr : NULL;
    int proc_refs = proc->refCount;
    int frame_refs = frame ? frame->refCount : -1;
    int body_refs = body->refCount;
    int def_refs = def->refCount;
    int epoch = ((Interp *)interp)->compileEpoch;
    const char *body_type = body->typePtr ? body->typePtr->name : "none";
    const char *def_type = def->typePtr ? def->typePtr->name : "none";
    const char *key_type = state->key && state->key->typePtr ? state->key->typePtr->name : "none";
    int key_refs = state->key ? state->key->refCount : -1;
    int body_resident = body->bytes != NULL;
    int def_resident = def->bytes != NULL;
    int same_proc = proc == state->initial_proc;
    int same_body = body == state->initial_body;
    int same_default = def == state->initial_default;
    int same_frame = frame == proc;
    int locals = proc->numCompiledLocals;
    /* Every field above is captured before any result/string observer. */
    printf("S|%d|%s|%d|%d|%d|%d|%d|%s|%d|%s|%d|%d|%s|%d|%d|%d|%d|%d\n",
        state->mode, window, same_proc, same_body, same_default, proc_refs,
        body_refs, body_type, body_resident, def_type, def_refs, def_resident,
        key_type, key_refs, same_frame, frame_refs, locals, epoch);
    fflush(stdout);
}

static int forward_set(ClientData data, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    State *state = (State *)data;
#if TCL_MAJOR_VERSION >= 9
    if (state->original_set.objProc2) {
        return state->original_set.objProc2(state->original_set.objClientData2, interp, objc, objv);
    }
#endif
    return state->original_set.objProc(state->original_set.objClientData, interp, objc, objv);
}

static void invalidate(Tcl_Interp *interp, State *state) {
    /* Keep the original native handler/compatibility client data alive. */
    if (Tcl_EvalEx(interp, "rename set __original_set", -1, 0) != TCL_OK) exit(31);
    if (!Tcl_GetCommandInfo(interp, "__original_set", &state->original_set)) exit(31);
    if (!Tcl_CreateObjCommand(interp, "set", forward_set, state, NULL)) exit(32);
}

static int invoke(Tcl_Interp *interp) {
    Tcl_Obj *word = Tcl_NewStringObj("p", 1);
    int code;
    Tcl_IncrRefCount(word);
    code = Tcl_EvalObjv(interp, 1, &word, 0);
    Tcl_DecrRefCount(word);
    return code;
}

static int refresh(ClientData data, Tcl_Interp *interp, int objc, Tcl_Obj *const objv[]) {
    State *state = (State *)data;
    int code;
    (void)objc; (void)objv;
    if (state->mode != 2 || !state->trigger || state->nested) return TCL_OK;
    sample(interp, state, "active-before-invalidation");
    invalidate(interp, state);
    sample(interp, state, "active-after-invalidation");
    state->nested = 1;
    code = invoke(interp);
    state->nested = 0;
    sample(interp, state, "active-after-recursive-call");
    return code;
}

static void completion(Tcl_Interp *interp, int mode, const char *window, int code) {
    ObjSize length, j;
    const unsigned char *bytes = (const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
    printf("R|%d|%s|%d|", mode, window, code);
    for (j = 0; j < length; ++j) printf("%02x", bytes[j]);
    putchar('\n'); fflush(stdout);
}

int main(int argc, char **argv) {
    Tcl_Interp *interp;
    Tcl_Obj *proc_body = NULL;
    State state;
    int code;
    if (argc != 2) return 2;
    memset(&state, 0, sizeof(state));
    state.mode = atoi(argv[1]);
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    Tcl_CreateObjCommand(interp, "refresh", refresh, &state, NULL);
    if (state.mode == 3) {
        state.key = Tcl_NewStringObj("x", 1);
        Tcl_IncrRefCount(state.key);
        if (!Tcl_SetVar2Ex(interp, "key", NULL, state.key, TCL_GLOBAL_ONLY)) return 3;
    }
    code = Tcl_EvalEx(interp, state.mode == 3
        ? "proc p {{x DEFAULT}} {set $::key; return $x}"
        : "proc p {{x DEFAULT}} {refresh; return $x}", -1, 0);
    if (code != TCL_OK) return 4;
    state.initial_proc = current_proc(interp);
    state.initial_body = state.initial_proc->bodyPtr;
    state.initial_default = state.initial_proc->firstLocalPtr->defValuePtr;
    sample(interp, &state, "defined");
    code = invoke(interp);
    sample(interp, &state, "warm-return");
    completion(interp, state.mode, "warm-return", code);
    if (code != TCL_OK) return 5;
    if (state.mode == 1) {
        proc_body = TclNewProcBodyObj(current_proc(interp));
        if (!proc_body) return 6;
        Tcl_IncrRefCount(proc_body);
        sample(interp, &state, "procbody-held");
    }
    if (state.mode != 2) {
        invalidate(interp, &state);
        sample(interp, &state, "invalidated");
    } else {
        state.trigger = 1;
    }
    code = invoke(interp);
    sample(interp, &state, "recompiled-return");
    completion(interp, state.mode, "recompiled-return", code);
    if (proc_body) {
        Tcl_DecrRefCount(proc_body);
        sample(interp, &state, "procbody-released");
    }
    if (state.key) Tcl_DecrRefCount(state.key);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return code == TCL_OK ? 0 : 7;
}
