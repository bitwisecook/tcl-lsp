#include <jim.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static Jim_Obj *query_name;
static Jim_Obj *incoming;
static uintptr_t retiring_address;
static int calls;
static int saw_old;
static int saw_new;
static int old_references;
static int incoming_references;

static void retiring_value(Jim_Interp *interp, Jim_Obj *original)
{
    Jim_Obj *selected = Jim_GetVariable(interp, query_name, JIM_NONE);
    calls++;
    saw_old = (uintptr_t)selected == retiring_address;
    saw_new = selected == incoming;
    /* These headers are both live during the free-internal-rep callback. */
    old_references = original->refCount;
    incoming_references = incoming->refCount;
}

static const Jim_ObjType type = {
    "originalAssignmentRetirement", retiring_value, NULL, NULL, JIM_TYPE_NONE
};

static void check(int condition, const char *label)
{
    if (!condition) {
        fprintf(stderr, "failed: %s\n", label);
        exit(1);
    }
}

int main(void)
{
    for (int fresh = 0; fresh <= 1; fresh++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        Jim_Obj *name = Jim_NewStringObj(interp, "target", -1);
        Jim_IncrRefCount(name);
        Jim_Obj *old = Jim_NewStringObj(interp, "OLD", -1);
        old->typePtr = &type;
        retiring_address = (uintptr_t)old;
        Jim_IncrRefCount(old);
        check(Jim_SetVariable(interp, name, old) == JIM_OK, "initial assignment");
        Jim_DecrRefCount(interp, old);
        query_name = fresh ? Jim_NewStringObj(interp, "target", -1) : name;
        Jim_IncrRefCount(query_name);
        incoming = Jim_NewStringObj(interp, "NEXT", -1);
        Jim_IncrRefCount(incoming);
        calls = saw_old = saw_new = old_references = incoming_references = -1;
        calls = 0;
        int code = Jim_SetVariable(interp, query_name, incoming);
        /* The retired pointer is never dereferenced after Jim_SetVariable. */
        Jim_Obj *after = Jim_GetVariable(interp, query_name, JIM_NONE);
        int after_is_incoming = after == incoming;
        printf("ASSIGN|fresh=%d|code=%d|callbacks=%d|during_old=%d|during_new=%d|old_refs_during=%d|incoming_refs_during=%d|after_new=%d|incoming_refs_after=%d\n",
            fresh, code, calls, saw_old, saw_new, old_references,
            incoming_references, after_is_incoming, incoming->refCount);
        check(code == JIM_OK && calls == 1 && saw_old == 1 && saw_new == 0,
            "old pointer remains selected during retirement");
        check(old_references == 0 && incoming_references == 2,
            "incoming reference precedes old retirement");
        check(after_is_incoming && incoming->refCount == 2, "future assignment");
        Jim_DecrRefCount(interp, incoming);
        Jim_DecrRefCount(interp, query_name);
        Jim_DecrRefCount(interp, name);
        Jim_FreeInterp(interp);
    }
    return 0;
}
