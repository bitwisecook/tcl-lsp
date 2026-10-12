/* Original public Tcl_SetObjErrorCode: selected header/storage and reached API phases. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tclInt.h"

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
    int updates;
    int frees;
    int duplicates;
    int caller_hold;
} HeaderRecord;

static Tcl_Interp *owner;
static HeaderRecord headers[2];
static const char *phase = "setup";
static int active_case;
static int sequence;
static int observe;
static int trace_calls;
static Tcl_Obj *prior_result;
static Tcl_ObjType lazy_type;
static const unsigned char payload[] = {'O','R','I','G','I','N','A','L',0,255,' ','C','O','D','E'};

/* Read actual already-existing storage only. The C84 hash lookup performs no
 * variable-worker, interpreter call, String getter or absent-shell creation.
 * Modern storage is the original private Interp field from its selected header. */
static Tcl_Obj *selected_header(void) {
    Interp *interp = (Interp *) owner;
#if SELECTED_C84
    Tcl_HashEntry *entry = Tcl_FindHashEntry(&interp->globalNsPtr->varTable, "errorCode");
    Var *variable;
    if (entry == NULL) return NULL;
    variable = (Var *) Tcl_GetHashValue(entry);
    if (!TclIsVarScalar(variable) || TclIsVarUndefined(variable)) return NULL;
    return variable->value.objPtr;
#else
    return interp->errorCode;
#endif
}

static const char *identity(Tcl_Obj *original) {
    if (original == NULL) return "NONE";
    if (original == headers[0].original) return "A";
    if (original == headers[1].original) return "B";
    return "OTHER";
}

static int explicit_flag(void) {
#if SELECTED_C84
    return (((Interp *) owner)->flags & ERROR_CODE_SET) != 0;
#else
    return -1;
#endif
}

static void event(const char *kind, HeaderRecord *record, Tcl_Obj *original) {
    if (!observe) return;
    printf("EVENT|%d|%d|phase=%s|kind=%s|record=%s|selected=%s|flag=%d|header=%p|refs=%ld\n",
        active_case, ++sequence, phase, kind, record ? record->label : "NA",
        identity(selected_header()), explicit_flag(), (void *) original,
        original ? (long) original->refCount : -1L);
}

static void free_rep(Tcl_Obj *original) {
    HeaderRecord *record = (HeaderRecord *) original->internalRep.otherValuePtr;
    record->frees++;
    event("free_int_rep", record, original);
    original->internalRep.otherValuePtr = NULL;
}

static void duplicate_rep(Tcl_Obj *original, Tcl_Obj *copy) {
    HeaderRecord *record = (HeaderRecord *) original->internalRep.otherValuePtr;
    record->duplicates++;
    copy->internalRep.otherValuePtr = record;
    copy->typePtr = &lazy_type;
    event("duplicate_int_rep", record, original);
}

static void update_string(Tcl_Obj *original) {
    HeaderRecord *record = (HeaderRecord *) original->internalRep.otherValuePtr;
    record->updates++;
    event("update_string", record, original);
    original->bytes = Tcl_Alloc((unsigned int) sizeof(payload) + 1);
    memcpy(original->bytes, payload, sizeof(payload));
    original->bytes[sizeof(payload)] = 0;
    original->length = (Count) sizeof(payload);
}

static Tcl_ObjType lazy_type = {
    "originalExplicitErrorCodeHeader", free_rep, duplicate_rep, update_string, NULL
};

static Tcl_Obj *fresh_header(int index) {
    Tcl_Obj *original = Tcl_NewObj();
    HeaderRecord *record = &headers[index];
    Tcl_IncrRefCount(original);
    Tcl_InvalidateStringRep(original);
    original->internalRep.otherValuePtr = record;
    original->typePtr = &lazy_type;
    record->original = original;
    record->caller_hold = 1;
    return original;
}

static void row(const char *stage, int public_code) {
    Tcl_Obj *selected = selected_header();
    Tcl_Obj *result = Tcl_GetObjResult(owner);
    printf("ROW|%d|%d|stage=%s|code=%d|selected=%s|selected_header=%p|flag=%d|trace=%d|result_same=%d|result_header=%p|prior_result_header=%p",
        active_case, ++sequence, stage, public_code, identity(selected), (void *) selected,
        explicit_flag(), trace_calls, result == prior_result, (void *) result, (void *) prior_result);
    for (int index = 0; index < 2; index++) {
        HeaderRecord *record = &headers[index];
        printf("|%s_update=%d|%s_free=%d|%s_dup=%d|%s_caller=%d|%s_header=%p",
            record->label, record->updates, record->label, record->frees,
            record->label, record->duplicates, record->label, record->caller_hold,
            record->label, (void *) record->original);
        /* A final free hook is not permission to dereference its retired header. */
        if (record->original != NULL && record->frees == 0) {
            printf("|%s_string=%d|%s_refs=%ld", record->label,
                record->original->bytes != NULL, record->label,
                (long) record->original->refCount);
        } else {
            printf("|%s_string=NA|%s_refs=NA", record->label, record->label);
        }
    }
    puts("");
}

static char *write_trace(ClientData data, Tcl_Interp *interp,
        const char *first, const char *second, int flags) {
    (void) data; (void) interp; (void) first; (void) second; (void) flags;
    trace_calls++;
    event("global_write_trace", NULL, NULL);
    return NULL;
}

static int callback(ClientData data, Tcl_Interp *interp, int count, Tcl_Obj *const *argv) {
    (void) data; (void) count; (void) argv;
    phase = "callback_before_setter";
    Tcl_SetObjResult(interp, Tcl_NewStringObj("ORIGINAL CALLBACK ERROR", -1));
    row("callback_before_setter", -1);
    phase = "callback_setter";
    Tcl_SetObjErrorCode(interp, headers[1].original);
    phase = "callback_after_setter_before_error_return";
    row("callback_after_setter_before_error_return", -1);
    return TCL_ERROR;
}

static void getter(Tcl_Obj *original) {
    Count count;
    const char *bytes;
    phase = "explicit_header_string_getter";
    puts("BOUNDARY|EXPLICIT_HEADER_STRING_GETTER_ENTRY");
    bytes = Tcl_GetStringFromObj(original, &count);
    puts("BOUNDARY|EXPLICIT_HEADER_STRING_GETTER_RETURN");
    printf("GETTER|%d|count=%ld|hex=", active_case, (long) count);
    for (Count index = 0; index < count; index++) printf("%02x", (unsigned char) bytes[index]);
    puts("");
    row("after_explicit_header_string_getter", -1);
}

static int run_case(int number) {
    Tcl_Obj *incoming;
    int code = TCL_OK;
    active_case = number; sequence = trace_calls = observe = 0;
    memset(headers, 0, sizeof(headers));
    headers[0].label = "A"; headers[1].label = "B";
    phase = "setup";
    owner = Tcl_CreateInterp();
    code = Tcl_Init(owner);
    printf("SETUP|%d|Tcl_Init=%d\n", number, code);
    if (code != TCL_OK) {
        Tcl_DeleteInterp(owner); owner = NULL; return 1;
    }
    incoming = fresh_header(1);
    if (number == 1 || number == 2) {
        Tcl_Obj *old = fresh_header(0);
        Tcl_SetObjErrorCode(owner, old);
        headers[0].caller_hold = 0;
        Tcl_DecrRefCount(old);
    }
    if (number == 2) {
        code = Tcl_TraceVar2(owner, "errorCode", NULL, TCL_GLOBAL_ONLY | TCL_TRACE_WRITES,
            write_trace, NULL);
        printf("SETUP|%d|Tcl_TraceVar2=%d\n", number, code);
        if (code != TCL_OK) {
            Tcl_DeleteInterp(owner); Tcl_DecrRefCount(incoming); owner = NULL; return 1;
        }
#if SELECTED_C84
        /* Actual public reset clears the original explicit-code flag, retaining
         * the C84 global cell. No synthetic flag or Var mutation is performed. */
        Tcl_ResetResult(owner);
#endif
    }
    Tcl_SetObjResult(owner, Tcl_NewStringObj("PRESERVED RESULT", -1));
    prior_result = Tcl_GetObjResult(owner);
    Tcl_IncrRefCount(prior_result);
    observe = 1;
    phase = "before_public_setter";
    row("before_public_setter", -1);
    if (number == 3) {
        Tcl_Obj *word = Tcl_NewStringObj("::original_explicit_callback", -1);
        Tcl_IncrRefCount(word);
        Tcl_CreateObjCommand(owner, "::original_explicit_callback", callback, NULL, NULL);
        phase = "public_eval_entry";
        puts("BOUNDARY|PUBLIC_EVAL_OBJV_ENTRY");
        code = Tcl_EvalObjv(owner, 1, &word, TCL_EVAL_DIRECT);
        puts("BOUNDARY|PUBLIC_EVAL_OBJV_RETURN");
        phase = "after_public_eval_before_presentation";
        row("after_public_eval_before_presentation", code);
        Tcl_DecrRefCount(word);
    } else {
        phase = "public_setter";
        puts("BOUNDARY|PUBLIC_SETTER_ENTRY");
        Tcl_SetObjErrorCode(owner, incoming);
        puts("BOUNDARY|PUBLIC_SETTER_RETURN");
        phase = "after_public_setter_before_getter";
        row("after_public_setter_before_getter", -1);
    }
    getter(incoming);
    /* No storage queries occur from final teardown free hooks. */
    observe = 0; phase = "teardown_unobserved";
    Tcl_DecrRefCount(prior_result);
    Tcl_DeleteInterp(owner);
    headers[1].caller_hold = 0;
    Tcl_DecrRefCount(incoming);
    owner = NULL;
    printf("CLOSED_CASE|%d\n", number);
    return 0;
}

int main(int argc, char **argv) {
    int major, minor, patch, type;
    (void) argc;
    setvbuf(stdout, NULL, _IONBF, 0);
    Tcl_FindExecutable(argv[0]);
    Tcl_GetVersion(&major, &minor, &patch, &type);
    printf("VERSION|%d.%d.%d|release_type=%d|selected_c84=%d\n",
        major, minor, patch, type, SELECTED_C84);
    for (int number = 0; number < 4; number++) if (run_case(number)) return 2;
    Tcl_Finalize();
    puts("CLOSED_ORIGINAL_EXPLICIT_ERROR_CODE_HEADER_SETTER");
    return 0;
}
