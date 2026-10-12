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
static void hex(const char *bytes, long length) {
    long i;
    for (i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
}
static int nibble(int ch) {
    if (ch >= '0' && ch <= '9') return ch - '0';
    if (ch >= 'a' && ch <= 'f') return ch - 'a' + 10;
    return -1;
}
static Object *snapshot_variable(Interp *interp, const char *name) {
#ifdef JIM_PROBE
    return Jim_GetGlobalVariableStr(interp, name, JIM_NONE);
#else
    return Tcl_GetVar2Ex(interp, name, NULL, TCL_GLOBAL_ONLY);
#endif
}
static void snapshot(Interp *interp, const char *name, const char *label) {
    Object *value = snapshot_variable(interp, name);
    printf("|%s_present=%d|%s_string=%d", label, value != NULL, label,
        value != NULL && value->bytes != NULL);
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
    /* Resident global names are read through the actual public variable API.
     * No value String getter, eval or result setter is used by this observer. */
    snapshot(lazy->interp, "::target", "target");
#ifdef JIM_PROBE
    /* A dictionary-sugar lookup could materialise another original key.
     * Keep that unavailable callback axis separate from public poststate. */
    printf("|arr_k_present=NA-no-nonmaterialising-array-observer|arr_k_string=NA"
           "|arr_m_present=NA-no-nonmaterialising-array-observer|arr_m_string=NA");
#else
    snapshot(lazy->interp, "::arr(k)", "arr_k");
    snapshot(lazy->interp, "::arr(m)", "arr_m");
#endif
    snapshot(lazy->interp, "::survivor", "survivor");
    printf("|payload=");
    hex((const char *)lazy->payload, lazy->length);
    puts("");
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
static const Jim_ObjType lazy_type = { "originalVariableOrder", free_lazy,
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
static const Tcl_ObjType lazy_type = { "originalVariableOrder", free_lazy,
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
    if (argc < 2) return JIM_ERR;
    return Jim_EvalObjVector(interp, argc - 1, argv + 1);
}
#else
static int direct_command(ClientData data, Tcl_Interp *interp, Count argc,
        Tcl_Obj *const *argv) {
    (void)data;
    if (argc < 2) return TCL_ERROR;
    return Tcl_EvalObjv(interp, argc - 1, argv + 1, 0);
}
#endif
static int observe(Interp *interp, const char *phase, const char *source) {
    int code = EVAL(interp, source);
    Count length;
    const char *bytes = BYTES(RESULT(interp), &length);
    printf("ROW|%d|%s|%d|", active_case, phase, code);
    hex(bytes, (long)length);
    printf("|events=%d\n", events);
    return code;
}
static const char *setups[] = {
    "proc ::p {{x VALUE}} {}; set ::a [lazy A 3a3a6d697373696e67]; set ::b [lazy B 78]; set ::c [lazy C 3a3a746172676574]; set ::setup_marker READY",
    "proc ::p {{x VALUE}} {}; set ::a [lazy A 3a3a70]; set ::b [lazy B 626164]; set ::c [lazy C 3a3a746172676574]; set ::setup_marker READY",
    "proc ::p {{x VALUE}} {}; set ::a [lazy A 3a3a70]; set ::b [lazy B 78]; set ::c [lazy C 3a3a746172676574]; set ::setup_marker READY",
    "proc ::p {x} {}; set ::a [lazy A 3a3a70]; set ::b [lazy B 78]; set ::c [lazy C 3a3a746172676574]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a617272]; set ::b [lazy B 6b]; set ::c [lazy C 56414c5545]; set ::pairs [list $::b $::c]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a617272]; set ::b [lazy B 6b]; set ::c [lazy C 56414c5545]; set ::pairs [list $::b $::c extra]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a617272]; set ::b [lazy B 6b]; set ::c [lazy C 6d]; set ::pairs [list $::b FIRST $::c SECOND]; set ::setup_marker READY",
    "array set ::arr {key VALUE}; set ::a [lazy A 3a3a6b]; set ::b [lazy B 3a3a76]; set ::c [lazy C 3a3a617272]; set ::vars [list $::a $::b]; set ::setup_marker READY",
    "array set ::arr {key VALUE}; set ::a [lazy A 3a3a6b]; set ::b [lazy B 3a3a617272]; set ::vars [list $::a]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a6b]; set ::b [lazy B 3a3a76]; set ::c [lazy C 3a3a6d697373696e67]; set ::vars [list $::a $::b]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::target OLD; set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 56414c5545]; set ::setup_marker READY",
    "set ::target 1; set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 32]; set ::setup_marker READY",
    "set ::a [lazy A 3a3a6d697373696e673a3a746172676574]; set ::b [lazy B 626164]; set ::setup_marker READY",
    "set ::target VALUE; set ::a [lazy A 3a3a6d697373696e67]; set ::b [lazy B 3a3a746172676574]; set ::setup_marker READY",
    "set ::target VALUE; set ::survivor STILL; set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 3a3a6d697373696e67]; set ::c [lazy C 3a3a7375727669766f72]; set ::setup_marker READY",
    "set ::target VALUE; set ::survivor STILL; set ::a [lazy A 3a3a746172676574]; set ::b [lazy B 3a3a6d697373696e67]; set ::c [lazy C 3a3a7375727669766f72]; set ::setup_marker READY",
    "set ::target VALUE; set ::a [lazy A 3a3a746172676574]; set ::setup_marker READY"
};
static const char *workers[] = {
    "direct info default $::a $::b $::c",
    "direct info default $::a $::b $::c",
    "direct info default $::a $::b $::c",
    "direct info default $::a $::b $::c",
    "direct array set $::a $::pairs",
    "direct array set $::a $::pairs",
    "direct array set $::a $::pairs",
    "direct array for $::vars $::c {}",
    "direct array for $::vars $::b {}",
    "direct array for $::vars $::c {}",
    "direct const $::a $::b",
    "direct const $::a $::b",
    "direct incr $::a $::b",
    "direct incr $::a $::b",
    "direct unset $::a $::b",
    "direct unset $::a $::b $::c",
    "direct unset -nocomplain $::a $::b $::c",
    "direct info exists $::a"
};
static const char *poststates[] = {
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "list [array exists ::arr] [array get ::arr]",
    "list [array exists ::arr] [array get ::arr]",
    "list [info exists ::arr(k)] [info exists ::arr(m)] [array get ::arr]",
    "list [info exists ::k] [info exists ::v]",
    "list [info exists ::k] [array exists ::arr]",
    "list [info exists ::k] [info exists ::v]",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "info exists ::missing::target",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}",
    "list [info exists ::target] [info exists ::survivor]",
    "list [info exists ::target] [info exists ::survivor]",
    "if {[info exists ::target]} {list 1 [set ::target]} else {list 0 ABSENT}"
};
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
    for (row = 0; row < 18; row++) {
        Interp *interp;
        int initialise;
        active_case = row; events = 0;
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
        if (observe(interp, "setup", setups[row]) == 0) {
            observe(interp, "worker", workers[row]);
            observe(interp, "poststate", poststates[row]);
        }
#ifdef JIM_PROBE
        Jim_FreeInterp(interp);
#else
        Tcl_DeleteInterp(interp);
#endif
        printf("CLOSED_CASE|%d\n", row);
    }
    puts("CLOSED_ORIGINAL_VARIABLE_INTROSPECTION_ORDER");
    return 0;
}
