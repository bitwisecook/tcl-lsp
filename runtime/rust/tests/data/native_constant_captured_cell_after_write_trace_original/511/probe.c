#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Object;
typedef int Count;
#define BYTES(o,n) Jim_GetString((o),(n))
#define RESULT(i) Jim_GetResult(i)
#define EVAL(i,s) Jim_Eval((i),(s))
#define ALLOC(n) Jim_Alloc(n)
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Object;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
#define BYTES(o,n) Tcl_GetStringFromObj((o),(n))
#define RESULT(i) Tcl_GetObjResult(i)
#define EVAL(i,s) Tcl_EvalEx((i),(s),-1,0)
#define ALLOC(n) Tcl_Alloc(n)
#endif

typedef struct Lazy {
    Interp *interp;
    char label[64];
    unsigned char payload[512];
    int length;
} Lazy;
static int active_case;
static int events;
static const char *active_phase = "bootstrap";
static Object *original_a;
static Object *original_b;
static int trace_calls;
static void headers(const char *stage);
static void hex(const char *bytes, long length) {
    long i;
    for (i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
}
static int nibble(int ch) {
    if (ch >= '0' && ch <= '9') return ch - '0';
    if (ch >= 'a' && ch <= 'f') return ch - 'a' + 10;
    return -1;
}
static void update_string(Object *original) {
#ifdef JIM_PROBE
    Lazy *lazy = original->internalRep.ptr;
#else
    Lazy *lazy = original->internalRep.otherValuePtr;
#endif
    events++;
    printf("EVENT|%d|%d|%s|string_before=%d|refs=%ld|original=%p", active_case,
        events, lazy->label, original->bytes != NULL, (long)original->refCount,
        (void *)original);
    /* Pure header observer: no interpreter operation is called here. */
    printf("|payload=");
    hex((const char *)lazy->payload, lazy->length);
    printf("|phase=%s\n", active_phase);
    original->bytes = ALLOC((size_t)lazy->length + 1);
    memcpy(original->bytes, lazy->payload, (size_t)lazy->length);
    original->bytes[lazy->length] = '\0';
    original->length = lazy->length;
}
#ifdef JIM_PROBE
static void free_lazy(Jim_Interp *interp, Jim_Obj *original) {
    (void)interp;
    free(original->internalRep.ptr);
}
static void duplicate_lazy(Jim_Interp *interp, Jim_Obj *source, Jim_Obj *target) {
    Lazy *copy = malloc(sizeof(*copy));
    (void)interp;
    memcpy(copy, source->internalRep.ptr, sizeof(*copy));
    target->internalRep.ptr = copy;
    target->typePtr = source->typePtr;
}
static const Jim_ObjType lazy_type = { "originalConstantCapturedCell", free_lazy,
    duplicate_lazy, update_string, JIM_TYPE_NONE };
static int lazy_command(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
#else
static void free_lazy(Tcl_Obj *original) {
    free(original->internalRep.otherValuePtr);
}
static void duplicate_lazy(Tcl_Obj *source, Tcl_Obj *target) {
    Lazy *copy = malloc(sizeof(*copy));
    memcpy(copy, source->internalRep.otherValuePtr, sizeof(*copy));
    target->internalRep.otherValuePtr = copy;
    target->typePtr = source->typePtr;
}
static const Tcl_ObjType lazy_type = { "originalConstantCapturedCell", free_lazy,
    duplicate_lazy, update_string, NULL };
static int lazy_command(ClientData data, Tcl_Interp *interp, Count argc,
        Tcl_Obj *const *argv) {
    (void)data;
#endif
    Count label_length, payload_length;
    const char *label, *payload;
    Lazy *lazy;
    Object *original;
    int i;
    if (argc != 3) return 1;
    label = BYTES(argv[1], &label_length);
    payload = BYTES(argv[2], &payload_length);
    if (label_length > 63 || payload_length > 1024 || payload_length % 2) return 1;
    lazy = calloc(1, sizeof(*lazy));
    lazy->interp = interp;
    memcpy(lazy->label, label, (size_t)label_length);
    lazy->length = (int)payload_length / 2;
    for (i = 0; i < lazy->length; i++) {
        int high = nibble(payload[2*i]), low = nibble(payload[2*i+1]);
        if (high < 0 || low < 0) { free(lazy); return 1; }
        lazy->payload[i] = (unsigned char)(16*high + low);
    }
#ifdef JIM_PROBE
    original = Jim_NewObj(interp);
    original->bytes = NULL;
    original->length = 0;
    original->typePtr = &lazy_type;
    original->internalRep.ptr = lazy;
    Jim_SetResult(interp, original);
#else
    original = Tcl_NewObj();
    Tcl_InvalidateStringRep(original);
    original->typePtr = &lazy_type;
    original->internalRep.otherValuePtr = lazy;
    Tcl_SetObjResult(interp, original);
#endif
    printf("BIRTH|%d|%s|string=%d|refs=%ld|original=%p|payload=", active_case, lazy->label,
        original->bytes != NULL, (long)original->refCount, (void *)original);
    hex((const char *)lazy->payload, lazy->length);
    puts("");
    return 0;
}
#ifdef JIM_PROBE
static int direct_command(Jim_Interp *interp, int argc, Jim_Obj *const *argv) {
    int code;
    if (argc < 2) return JIM_ERR;
    printf("VECTOR_ENTRY|%d|%d|events=%d\n", active_case, argc - 1, events);
    headers("before-vector");
    code = Jim_EvalObjVector(interp, argc - 1, argv + 1);
    headers("after-vector-before-outer-eval");
    printf("VECTOR_RETURN|%d|%d|events=%d\n", active_case, code, events);
    return code;
}
#else
static int direct_command(ClientData data, Tcl_Interp *interp, Count argc,
        Tcl_Obj *const *argv) {
    (void)data;
    int code;
    if (argc < 2) return TCL_ERROR;
    printf("VECTOR_ENTRY|%d|%ld|events=%d\n", active_case, (long)argc - 1, events);
    headers("before-vector");
    code = Tcl_EvalObjv(interp, argc - 1, argv + 1, 0);
    headers("after-vector-before-outer-eval");
    printf("VECTOR_RETURN|%d|%d|events=%d\n", active_case, code, events);
    return code;
}
#endif
static int observe(Interp *interp, const char *phase, const char *source) {
    int code;
    Count length;
    const char *bytes;
    active_phase = phase;
    code = EVAL(interp, source);
    printf("EVAL_RETURN|%d|%s|%d|events=%d\n", active_case, phase, code, events);
    if (original_a != NULL && original_b != NULL) headers("after-eval-before-public-result");
    active_phase = "public-result-getter";
    bytes = BYTES(RESULT(interp), &length);
    printf("ROW|%d|%s|%d|", active_case, phase, code);
    hex(bytes, (long)length);
    printf("|events=%d\n", events);
    return code;
}

static void headers(const char *stage) {
    printf("HEADERS|%d|%s|events=%d|trace_calls=%d|a=%p|a_string=%d|a_refs=%ld|b=%p|b_string=%d|b_refs=%ld\n",
        active_case, stage, events, trace_calls, (void *)original_a,
        original_a->bytes != NULL, (long)original_a->refCount,
        (void *)original_b, original_b->bytes != NULL, (long)original_b->refCount);
}
#ifndef JIM_PROBE
#if TCL_MAJOR_VERSION >= 9
static char *write_trace(void *data, Tcl_Interp *interp, const char *part1,
        const char *part2, int flags) {
    const char *previous_phase = active_phase;
    int outcome = 0;
    (void)data;
    printf("TRACE|%d|entry|flags=%d|part1=", active_case, flags);
    hex(part1, (long)strlen(part1));
    printf("|part2=");
    if (part2 != NULL) hex(part2, (long)strlen(part2));
    puts("");
    if (!(flags & TCL_TRACE_WRITES)) return NULL;
    trace_calls++;
    headers("write-trace-before-mutation");
    active_phase = "declared-write-trace-mutation";
    switch (active_case) {
    case 0: break;
    case 1:
        headers("write-trace-before-guest-rejection");
        active_phase = previous_phase;
        return "original constant trace rejected";
    case 2:
        outcome = Tcl_UnsetVar(interp, part1, TCL_GLOBAL_ONLY);
        break;
    case 3:
        outcome = Tcl_UnsetVar(interp, part1, TCL_GLOBAL_ONLY);
        if (outcome == TCL_OK)
            outcome = Tcl_SetVar(interp, part1, "RECREATED", TCL_GLOBAL_ONLY) != NULL ? 0 : 1;
        break;
    case 4:
        outcome = Tcl_UnsetVar(interp, part1, TCL_GLOBAL_ONLY);
        if (outcome == TCL_OK)
            outcome = Tcl_SetVar2(interp, part1, "k", "RECREATED", TCL_GLOBAL_ONLY) != NULL ? 0 : 1;
        break;
    case 5:
        outcome = EVAL(interp, "namespace delete ::holder; namespace eval ::holder {}");
        if (outcome == TCL_OK)
            outcome = Tcl_SetVar(interp, "::holder::target", "RECREATED", TCL_GLOBAL_ONLY) != NULL ? 0 : 1;
        break;
    case 6:
        outcome = Tcl_SetVar(interp, part1, "TRACE_VALUE", TCL_GLOBAL_ONLY) != NULL ? 0 : 1;
        break;
    case 7:
        outcome = Tcl_UnsetVar(interp, part1, TCL_GLOBAL_ONLY);
        if (outcome == TCL_OK) outcome = EVAL(interp, "const ::target INNER");
        break;
    }
    active_phase = previous_phase;
    headers("write-trace-after-mutation");
    printf("TRACE|%d|return|mutation_code=%d\n", active_case, outcome);
    return outcome == 0 ? NULL : "original constant declared trace mutation failed";
}
#endif
#endif
static const char *setups[] = {
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "namespace eval ::holder {}; set ::a [lazy A 3a3a686f6c6465723a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY"
};
static void poststates(Interp *interp, int row) {
    const char *target = row == 5 ? "::holder::target" : "::target";
    char source[512];
    snprintf(source, sizeof(source), "list [info exists %s] [array exists %s]", target, target);
    observe(interp, "post-exists-and-array", source);
    snprintf(source, sizeof(source), "set %s", target);
    observe(interp, "post-scalar-read", source);
    snprintf(source, sizeof(source), "array get %s", target);
    observe(interp, "post-array-read", source);
    snprintf(source, sizeof(source), "info constant %s", target);
    observe(interp, "post-constant-query", source);
    snprintf(source, sizeof(source), "set %s MUTABILITY_ATTEMPT", target);
    observe(interp, "post-mutable-set", source);
    snprintf(source, sizeof(source), "info constant %s", target);
    observe(interp, "post-constant-after-set", source);
}
int main(int argc, char **argv) {
    int row;
    (void)argc;
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#else
    (void)argv;
#endif
    printf("ABI|int=%lu|long=%lu|refcount=%lu\n", (unsigned long)sizeof(int),
        (unsigned long)sizeof(long), (unsigned long)sizeof(((Object *)0)->refCount));
    for (row = 0; row < 8; row++) {
        Interp *interp;
        int initialise;
        active_case = row; events = 0; trace_calls = 0;
#ifdef JIM_PROBE
        interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        initialise = Jim_InitStaticExtensions(interp);
        Jim_CreateCommand(interp, "lazy", lazy_command, NULL, NULL);
        Jim_CreateCommand(interp, "direct", direct_command, NULL, NULL);
#else
        interp = Tcl_CreateInterp();
        initialise = Tcl_Init(interp);
#if TCL_MAJOR_VERSION >= 9
        Tcl_CreateObjCommand2(interp, "lazy", lazy_command, NULL, NULL);
        Tcl_CreateObjCommand2(interp, "direct", direct_command, NULL, NULL);
#else
        Tcl_CreateObjCommand(interp, "lazy", lazy_command, NULL, NULL);
        Tcl_CreateObjCommand(interp, "direct", direct_command, NULL, NULL);
#endif
#endif
        printf("INITIALISE|%d|%d\n", row, initialise);
        if (initialise != 0) return 2;
        observe(interp, "version", "info patchlevel");
        observe(interp, "availability", "info commands const");
        if (observe(interp, "setup", setups[row]) != 0) return 3;
#ifdef JIM_PROBE
        original_a = Jim_GetGlobalVariableStr(interp, "::a", JIM_ERRMSG);
        original_b = Jim_GetGlobalVariableStr(interp, "::b", JIM_ERRMSG);
        printf("OBSERVER_AVAILABILITY|%d|write-trace=NA|reason=selected-const-worker-and-C-public-trace-purpose-unavailable\n", row);
#else
        original_a = Tcl_GetVar2Ex(interp, "::a", NULL, TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG);
        original_b = Tcl_GetVar2Ex(interp, "::b", NULL, TCL_GLOBAL_ONLY | TCL_LEAVE_ERR_MSG);
#if TCL_MAJOR_VERSION >= 9
        printf("OBSERVER_AVAILABILITY|%d|write-trace=PUBLIC_C9\n", row);
        if (Tcl_TraceVar2(interp, row == 5 ? "::holder::target" : "::target", NULL,
                TCL_GLOBAL_ONLY | TCL_TRACE_WRITES | TCL_TRACE_UNSETS, write_trace, NULL) != TCL_OK) return 4;
#else
        printf("OBSERVER_AVAILABILITY|%d|write-trace=NA|reason=selected-const-worker-unavailable\n", row);
#endif
#endif
        if (original_a == NULL || original_b == NULL) return 5;
        headers("before-worker-eval");
        observe(interp, "worker", "direct const $::a $::b");
        headers("after-worker-result-presentation");
        poststates(interp, row);
        original_a = NULL; original_b = NULL;
#ifdef JIM_PROBE
        Jim_FreeInterp(interp);
#else
        Tcl_DeleteInterp(interp);
#endif
        printf("CLOSED_CASE|%d\n", row);
    }
    puts("CLOSED_ORIGINAL_CONSTANT_CAPTURED_CELL_ORDER");
    return 0;
}
