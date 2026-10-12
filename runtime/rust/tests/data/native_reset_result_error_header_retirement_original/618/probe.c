/* Original public Tcl_ResetResult: exact prepared header roles and pure observation. */
#include "tclInt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
#define SELECTED_C84 1
#else
#define SELECTED_C84 0
#endif

typedef struct HeaderRecord {
    const char *label;
    Tcl_Obj *original;
    int updates, frees, duplicates, caller_hold;
} HeaderRecord;
static HeaderRecord records[3];
static Tcl_Interp *owner;
static int active_case, sequence, observe, code_traces, info_traces;
static const char *phase;
static Tcl_ObjType lazy_type;
static const unsigned char payload[] = {'R','E','S','E','T',0,255,' ','H','E','A','D','E','R'};

/* This observer performs no variable worker, String getter or missing-shell creation. */
static Tcl_Obj *selected_code(void) {
#if SELECTED_C84
    Interp *interp = (Interp *) owner;
    Tcl_HashEntry *entry = Tcl_FindHashEntry(&interp->globalNsPtr->varTable, "errorCode");
    Var *var;
    if (entry == NULL) return NULL;
    var = (Var *) Tcl_GetHashValue(entry);
    return TclIsVarScalar(var) && !TclIsVarUndefined(var) ? var->value.objPtr : NULL;
#else
    return ((Interp *) owner)->errorCode;
#endif
}
static Tcl_Obj *selected_info(void) {
#if SELECTED_C84
    Interp *interp = (Interp *) owner;
    Tcl_HashEntry *entry = Tcl_FindHashEntry(&interp->globalNsPtr->varTable, "errorInfo");
    Var *var;
    if (entry == NULL) return NULL;
    var = (Var *) Tcl_GetHashValue(entry);
    return TclIsVarScalar(var) && !TclIsVarUndefined(var) ? var->value.objPtr : NULL;
#else
    return ((Interp *) owner)->errorInfo;
#endif
}
static const char *identity(Tcl_Obj *p) {
    if (p == NULL) return "NONE";
    for (int n = 0; n < 3; n++) if (p == records[n].original) return records[n].label;
    return "OTHER";
}
static int copy_flag(void) {
#if SELECTED_C84
    return -1;
#else
    return ((((Interp *) owner)->flags & ERR_LEGACY_COPY) != 0);
#endif
}
static int explicit_flag(void) {
#if SELECTED_C84
    return ((((Interp *) owner)->flags & ERROR_CODE_SET) != 0);
#else
    return -1;
#endif
}
static void event(const char *kind, HeaderRecord *r, Tcl_Obj *passed) {
    if (!observe) return;
    printf("EVENT|%d|%d|phase=%s|kind=%s|record=%s|passed_header=%p|passed_refs=%ld|code=%s|code_header=%p|info=%s|info_header=%p|result=%s|result_header=%p|copy=%d|explicit=%d\n",
        active_case, ++sequence, phase, kind, r ? r->label : "NA", (void *) passed,
        passed ? (long) passed->refCount : -1L,
        identity(selected_code()), (void *) selected_code(), identity(selected_info()),
        (void *) selected_info(), identity(((Interp *) owner)->objResultPtr),
        (void *) ((Interp *) owner)->objResultPtr, copy_flag(), explicit_flag());
}
static void free_rep(Tcl_Obj *passed) {
    HeaderRecord *r = (HeaderRecord *) passed->internalRep.otherValuePtr;
    r->frees++;
    event("free_int_rep", r, passed);
    passed->internalRep.otherValuePtr = NULL;
}
static void duplicate_rep(Tcl_Obj *passed, Tcl_Obj *copy) {
    HeaderRecord *r = (HeaderRecord *) passed->internalRep.otherValuePtr;
    r->duplicates++;
    copy->internalRep.otherValuePtr = r;
    copy->typePtr = &lazy_type;
    event("duplicate_int_rep", r, passed);
}
static void update_string(Tcl_Obj *passed) {
    HeaderRecord *r = (HeaderRecord *) passed->internalRep.otherValuePtr;
    r->updates++;
    event("update_string", r, passed);
    passed->bytes = Tcl_Alloc((unsigned int) sizeof(payload) + 1);
    memcpy(passed->bytes, payload, sizeof(payload));
    passed->bytes[sizeof(payload)] = 0;
    passed->length = (Count) sizeof(payload);
}
static Tcl_ObjType lazy_type = {
    "originalResetErrorHeader", free_rep, duplicate_rep, update_string, NULL
};
static Tcl_Obj *fresh(int n) {
    Tcl_Obj *p = Tcl_NewObj();
    Tcl_IncrRefCount(p);
    Tcl_InvalidateStringRep(p);
    p->internalRep.otherValuePtr = &records[n];
    p->typePtr = &lazy_type;
    records[n].original = p;
    records[n].caller_hold = 1;
    return p;
}
static void release_caller(int n) {
    if (!records[n].caller_hold) return;
    records[n].caller_hold = 0;
    Tcl_DecrRefCount(records[n].original);
}
static char *write_trace(ClientData data, Tcl_Interp *interp,
        const char *first, const char *second, int flags) {
    (void) interp; (void) first; (void) second; (void) flags;
    if (!observe) return NULL;
    if (data == &records[1]) code_traces++; else info_traces++;
    event(data == &records[1] ? "global_code_write_trace" : "global_info_write_trace", NULL, NULL);
    return NULL;
}
static void row(const char *stage) {
    Tcl_Obj *result = ((Interp *) owner)->objResultPtr;
    printf("ROW|%d|%d|stage=%s|code=%s|code_header=%p|info=%s|info_header=%p|result=%s|result_header=%p|result_length=%ld|result_string=%d|result_type_none=%d|copy=%d|explicit=%d|code_trace=%d|info_trace=%d",
        active_case, ++sequence, stage, identity(selected_code()), (void *) selected_code(),
        identity(selected_info()), (void *) selected_info(), identity(result), (void *) result,
        (long) result->length, result->bytes != NULL, result->typePtr == NULL,
        copy_flag(), explicit_flag(), code_traces, info_traces);
    for (int n = 0; n < 3; n++) {
        HeaderRecord *r = &records[n];
        printf("|%s_header=%p|%s_updates=%d|%s_free_int_rep=%d|%s_duplicates=%d|%s_caller=%d",
            r->label, (void *) r->original, r->label, r->updates, r->label, r->frees,
            r->label, r->duplicates, r->label, r->caller_hold);
        /* A freeIntRep callback does not prove its object survives or dies.
         * Never dereference that original after the callback; current result is independent. */
        if (r->original != NULL && r->frees == 0) {
            printf("|%s_refs=%ld|%s_string=%d", r->label, (long) r->original->refCount,
                   r->label, r->original->bytes != NULL);
        } else printf("|%s_refs=NA|%s_string=NA", r->label, r->label);
    }
    puts("");
}
static void getter(int n) {
    Count length;
    const char *bytes;
    HeaderRecord *r = &records[n];
    /* The explicit caller +1 is the genuine getter hold; no storage loan is inferred. */
    if (!r->caller_hold || r->frees) return;
    phase = "explicit_caller_header_string_getter";
    printf("BOUNDARY|%d|GETTER_ENTRY|%s\n", active_case, r->label);
    bytes = Tcl_GetStringFromObj(r->original, &length);
    printf("BOUNDARY|%d|GETTER_RETURN|%s\n", active_case, r->label);
    printf("GETTER|%d|header=%s|count=%ld|hex=", active_case, r->label, (long) length);
    for (Count k = 0; k < length; k++) printf("%02x", (unsigned char) bytes[k]);
    puts("");
    row("after_explicit_caller_header_getter");
}
static int run_case(int n) {
    Tcl_Obj *code = NULL, *info = NULL;
    int init, code_trace_setup = -1, info_trace_setup = -1;
    active_case = n; sequence = code_traces = info_traces = observe = 0;
    phase = "setup"; memset(records, 0, sizeof(records));
    records[0].label = "R"; records[1].label = "C"; records[2].label = "I";
    owner = Tcl_CreateInterp(); init = Tcl_Init(owner);
    printf("CASE|%d|init_code=%d|selected_c84=%d|private_info_fixture=%d|shared_code_info_fixture=%d\n",
        n, init, SELECTED_C84, !SELECTED_C84 && n != 0, n == 4);
    if (init != TCL_OK) { Tcl_DeleteInterp(owner); owner = NULL; return 1; }
    Tcl_ResetResult(owner);
    (void) Tcl_UnsetVar2(owner, "errorCode", NULL, TCL_GLOBAL_ONLY);
    (void) Tcl_UnsetVar2(owner, "errorInfo", NULL, TCL_GLOBAL_ONLY);
    Tcl_ResetResult(owner);
    if (n != 0) {
        code = fresh(1); info = n == 4 ? code : fresh(2);
        Tcl_SetObjErrorCode(owner, code);
#if SELECTED_C84
        if (Tcl_SetVar2Ex(owner, "errorInfo", NULL, info, TCL_GLOBAL_ONLY) != info) {
            puts("SETUP_REFUSED|global_info_original_header");
            observe = 0; Tcl_DeleteInterp(owner); release_caller(1); release_caller(2); owner = NULL; return 1;
        }
#else
        /* Explicit input fixture: authentic private slot and one declared native +1.
         * This is not a public lazy-errorInfo producer or an entered-script receipt. */
        if (((Interp *) owner)->errorInfo != NULL) {
            puts("SETUP_REFUSED|private_info_not_empty");
            observe = 0; Tcl_DeleteInterp(owner); release_caller(1); release_caller(2); owner = NULL; return 1;
        }
        Tcl_IncrRefCount(info);
        ((Interp *) owner)->errorInfo = info;
#endif
        if (n == 1) { release_caller(1); release_caller(2); }
        if (n == 3) {
            code_trace_setup = Tcl_TraceVar2(owner, "errorCode", NULL, TCL_GLOBAL_ONLY | TCL_TRACE_WRITES, write_trace, &records[1]);
            info_trace_setup = Tcl_TraceVar2(owner, "errorInfo", NULL, TCL_GLOBAL_ONLY | TCL_TRACE_WRITES, write_trace, &records[2]);
            if (code_trace_setup != TCL_OK || info_trace_setup != TCL_OK) {
                puts("SETUP_REFUSED|global_trace_registration");
                observe = 0; Tcl_DeleteInterp(owner); release_caller(1); release_caller(2); owner = NULL; return 1;
            }
#if !SELECTED_C84
            /* Deliberately selected compatibility-flag input, not a claimed public producer. */
            ((Interp *) owner)->flags |= ERR_LEGACY_COPY;
#endif
        }
    }
    Tcl_SetObjResult(owner, fresh(0)); release_caller(0);
    printf("SETUP|%d|code_trace=%d|info_trace=%d|caller_code=%d|caller_info=%d|copy=%d\n",
        n, code_trace_setup, info_trace_setup, records[1].caller_hold, records[2].caller_hold, copy_flag());
    observe = 1; phase = "before_reset"; row("before_reset");
    puts("BOUNDARY|RESET_RESULT_ENTRY"); phase = "actual_reset_result";
    Tcl_ResetResult(owner);
    puts("BOUNDARY|RESET_RESULT_RETURN"); phase = "after_reset_before_any_getter";
    row("after_reset_before_any_getter");
    getter(1); getter(2);
    phase = "explicit_caller_release_after_reset";
    release_caller(1); release_caller(2); row("after_explicit_caller_releases");
    observe = 0; phase = "teardown_unobserved";
    Tcl_DeleteInterp(owner); owner = NULL;
    printf("CLOSED_CASE|%d\n", n);
    return 0;
}
int main(int argc, char **argv) {
    int major, minor, patch, type, failed = 0;
    (void) argc; setvbuf(stdout, NULL, _IONBF, 0);
    Tcl_FindExecutable(argv[0]); Tcl_GetVersion(&major, &minor, &patch, &type);
    printf("VERSION|%d.%d.%d|release_type=%d|header_major=%d|header_minor=%d\n", major, minor, patch, type, TCL_MAJOR_VERSION, TCL_MINOR_VERSION);
    for (int n = 0; n < 5; n++) failed |= run_case(n);
    puts("CLOSED_ORIGINAL_RESET_RESULT_ERROR_HEADER_RETIREMENT");
    return failed;
}
