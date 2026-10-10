#include <stdint.h>
#include <stdio.h>
#include "tcl.h"
#include "tclInt.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size L;
#else
typedef int L;
#endif

static Tcl_Interp *active;
static int hook_calls, old_b_deleted, new_b_deleted;
static int measuring, ordinal, old_known, new_known;
static uint64_t old_id, new_id;

static void event(const char *stage, const char *callback_kind, int created) {
    Tcl_Namespace *b;
    uint64_t current_id = 0;
    if (!measuring) return;
    b = Tcl_FindNamespace(active, "::B", NULL, 0);
    if (b) current_id = (uint64_t)((Namespace *)b)->nsId;
    fprintf(stderr,
        "{\"ordinal\":%d,\"stage\":\"%s\",\"callbackKind\":\"%s\","
        "\"B\":%d,\"oldIncarnation\":%d,\"newIncarnation\":%d,"
        "\"oldKnown\":%d,\"newKnown\":%d,\"createReturnedNonNull\":%d,"
        "\"hook\":%d,\"oldBdeleted\":%d,\"newBdeleted\":%d}\n",
        ++ordinal, stage, callback_kind, b != NULL,
        b && old_known && current_id == old_id,
        b && new_known && current_id == new_id,
        old_known, new_known, created,
        hook_calls, old_b_deleted, new_b_deleted);
}

static void old_b(void *p) {
    (void)p;
    old_b_deleted++;
    event("old-b-delete-proc", "namespace-DeleteProc", -1);
}
static void new_b(void *p) {
    (void)p;
    new_b_deleted++;
    event("new-b-delete-proc", "namespace-DeleteProc", -1);
}
static int hook(void *p, Tcl_Interp *i, int n, Tcl_Obj *const *v) {
    (void)p; (void)i; (void)n; (void)v;
    return TCL_OK;
}
static void retire(void *p) {
    Tcl_Namespace *b = Tcl_FindNamespace(active, "::B", NULL, 0);
    hook_calls++;
    event("retire-entry", "command-DeleteProc", -1);
    event("retire-before-delete-B", "command-DeleteProc", -1);
    if (b) Tcl_DeleteNamespace(b);
    event("retire-after-delete-B", "command-DeleteProc", -1);
    if (p) {
        Tcl_Namespace *created;
        event("retire-before-create-B", "command-DeleteProc", -1);
        created = Tcl_CreateNamespace(active, "::B", NULL, new_b);
        if (created) {
            new_id = (uint64_t)((Namespace *)created)->nsId;
            new_known = 1;
        }
        event("retire-after-create-B", "command-DeleteProc", created != NULL);
    }
}

int main(void) {
    Tcl_Interp *i = Tcl_CreateInterp();
    Tcl_Namespace *b;
    Tcl_Obj *v[4];
    int code;
    L len;
    const unsigned char *bytes;
    active = i;
    Tcl_CreateNamespace(i, "::A", NULL, NULL);
    Tcl_CreateNamespace(i, "::A::q", NULL, NULL);
    Tcl_CreateNamespace(i, "::B", NULL, old_b);
    Tcl_CreateObjCommand(i, "::A::hook", hook, NULL, retire);
    /* Preserve the original scenario-3 setup, including its unmeasured deletion. */
    Tcl_DeleteCommand(i, "::A::hook");
    hook_calls = old_b_deleted = new_b_deleted = 0;
    b = Tcl_CreateNamespace(i, "::B", NULL, old_b);
    if (!b) {
        fprintf(stderr, "setup failed to recreate ::B\n");
        Tcl_DeleteInterp(i);
        return 2;
    }
    old_id = (uint64_t)((Namespace *)b)->nsId;
    old_known = 1;
    Tcl_CreateObjCommand(i, "::A::hook", hook, (void *)1, retire);
    v[0] = Tcl_NewStringObj("namespace", -1);
    v[1] = Tcl_NewStringObj("delete", -1);
    v[2] = Tcl_NewStringObj("::A", -1);
    v[3] = Tcl_NewStringObj("::B", -1);
    for (int k = 0; k < 4; k++) Tcl_IncrRefCount(v[k]);
    measuring = 1;
    event("before-eval", "none", -1);
    code = Tcl_EvalObjv(i, 4, v, 0);
    event("after-eval", "none", -1);
    bytes = (const unsigned char *)Tcl_GetStringFromObj(Tcl_GetObjResult(i), &len);
    printf("{\"case\":\"callback-recreates-later\",\"code\":%d,\"result\":\"", code);
    for (L k = 0; k < len; k++) printf("%02x", bytes[k]);
    printf("\",\"A\":%d,\"child\":%d,\"B\":%d,\"hook\":%d,\"oldBdeleted\":%d,\"newBdeleted\":%d}\n",
        Tcl_FindNamespace(i, "::A", NULL, 0) != NULL,
        Tcl_FindNamespace(i, "::A::q", NULL, 0) != NULL,
        Tcl_FindNamespace(i, "::B", NULL, 0) != NULL,
        hook_calls, old_b_deleted, new_b_deleted);
    event("after-result-publication", "none", -1);
    measuring = 0;
    for (int k = 0; k < 4; k++) Tcl_DecrRefCount(v[k]);
    Tcl_DeleteInterp(i);
    return 0;
}
