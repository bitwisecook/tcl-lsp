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
static int command_exists(Interp *interp, const char *name) {
#ifdef JIM_PROBE
    Jim_Obj *original = Jim_NewStringObj(interp, name, -1);
    int present;
    Jim_IncrRefCount(original);
    present = Jim_GetCommand(interp, original, JIM_NONE) != NULL;
    Jim_DecrRefCount(interp, original);
    return present;
#else
    Tcl_CmdInfo info;
    return Tcl_GetCommandInfo(interp, name, &info);
#endif
}
static void update_string(Object *original) {
#ifdef JIM_PROBE
    Lazy *lazy = original->internalRep.ptr;
#else
    Lazy *lazy = original->internalRep.otherValuePtr;
#endif
    Count length;
    events++;
    printf("EVENT|%d|%d|%s|string_before=%d|exports=", active_case, events,
        lazy->label, original->bytes != NULL);
#ifdef JIM_PROBE
    /* No C export-list API is asserted for the Jim namespace helper. */
    printf("NA-no-C-export-list");
    (void)length;
#elif TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    printf("NA-selected-public-header-no-export-list-api");
    (void)length;
#else
    {
        Tcl_Obj *exports = Tcl_NewObj();
        int code;
        Tcl_IncrRefCount(exports);
        code = Tcl_AppendExportList(lazy->interp, NULL, exports);
        printf("%d:", code);
        if (code == TCL_OK) {
            const char *bytes = Tcl_GetStringFromObj(exports, &length);
            hex(bytes, (long)length);
        }
        Tcl_DecrRefCount(exports);
    }
#endif
    printf("|a=%d|b=%d|p=%d|refs=%ld|payload=",
        command_exists(lazy->interp, "::d::a"),
        command_exists(lazy->interp, "::d::b"),
        command_exists(lazy->interp, "::d::p"), (long)original->refCount);
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
static const Jim_ObjType lazy_type = { "originalNamingOrder", free_lazy,
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
static const Tcl_ObjType lazy_type = { "originalNamingOrder", free_lazy,
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
    printf("BIRTH|%d|%s|string=%d|refs=%ld|payload=", active_case, lazy->label,
        original->bytes != NULL, (long)original->refCount);
    hex((const char *)lazy->payload, lazy->length);
    puts("");
    return 0;
}
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
    "namespace eval ::s {proc a {} {}; proc b {} {}; namespace export *}; namespace eval ::d {namespace export old}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A 3a3a626164]; set b [lazy B 7461696c2a]}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A 6f6e652a]; set b [lazy B 3a3a626164]; set c [lazy C 6c6173742a]}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A 3a3a626164]; set b [lazy B 2a]}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A 6f6e65]; set b [lazy B 2d636c656172]; set c [lazy C 6c617374]}; set setup_marker READY",
    "namespace eval ::s {proc a {} {}; proc b {} {}; namespace export *}; namespace eval ::d {set a [lazy A 3a3a733a3a61]; set b [lazy B 3a3a6d697373696e673a3a2a]; set c [lazy C 3a3a733a3a62]}; set setup_marker READY",
    "namespace eval ::s {proc a {} {}; proc b {} {}; namespace export *}; namespace eval ::d {set a [lazy A 3a3a733a3a61]; set b [lazy B 3a3a6d697373696e673a3a2a]; set c [lazy C 3a3a733a3a62]}; set setup_marker READY",
    "namespace eval ::s {proc a {} {}; namespace export *}; namespace eval ::d {set a [lazy A 756e7175616c6966696564]; set b [lazy B 3a3a733a3a61]}; set setup_marker READY",
    "namespace eval ::s {proc a {} {}; proc b {} {}; namespace export *}; namespace eval ::d {namespace import ::s::*; set a [lazy A 3a3a733a3a61]; set b [lazy B 3a3a6d697373696e673a3a2a]; set c [lazy C 3a3a733a3a62]}; set setup_marker READY",
    "namespace eval ::d {set a [lazy A 3a3a6d697373696e673a3a70]; set b [lazy B 7b626164]; set c [lazy C 72657475726e204f4b]}; set setup_marker READY",
    "namespace eval ::d {set a [lazy A 3a3a643a3a70]; set b [lazy B 7b626164]; set c [lazy C 72657475726e204f4b]}; set setup_marker READY",
    "namespace eval ::d {set a [lazy A 3a3a643a3a70]; set b [lazy B {}]; set c [lazy C 72657475726e204f4b]}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A 6f6b003a3a626164]; set b [lazy B 6c617374]}; set setup_marker READY",
    "namespace eval ::s {proc a {} {}; namespace export *}; namespace eval ::d {set a [lazy A 3a3a733a3a61003a3a626164]; set b [lazy B 3a3a6d697373696e673a3a2a]}; set setup_marker READY",
    "namespace eval ::d {namespace export old; set a [lazy A c3a92a]; set b [lazy B 3a3a626164]; set c [lazy C 6c617374]}; set setup_marker READY"
};
static const char *workers[] = {
    "namespace eval ::d {namespace export}",
    "namespace eval ::d {namespace export -clear $a $b}",
    "namespace eval ::d {namespace export $a $b $c}",
    "namespace eval ::d {namespace export $a $b}",
    "namespace eval ::d {namespace export $a $b $c}",
    "namespace eval ::d {namespace import $a $b $c}",
    "namespace eval ::d {namespace import -force $a $b $c}",
    "namespace eval ::d {namespace import $a $b}",
    "namespace eval ::d {namespace forget $a $b $c}",
    "namespace eval ::d {proc $a $b $c}",
    "namespace eval ::d {proc $a $b $c}",
    "namespace eval ::d {proc $a $b $c}",
    "namespace eval ::d {namespace export $a $b}",
    "namespace eval ::d {namespace import $a $b}",
    "namespace eval ::d {namespace export $a $b $c}"
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
    for (row = 0; row < 15; row++) {
        Interp *interp;
        int initialise;
        active_case = row; events = 0;
#ifdef JIM_PROBE
        interp = Jim_CreateInterp();
        Jim_RegisterCoreCommands(interp);
        initialise = Jim_InitStaticExtensions(interp);
        Jim_CreateCommand(interp, "lazy", lazy_command, NULL, NULL);
#else
        interp = Tcl_CreateInterp();
        initialise = Tcl_Init(interp);
#if TCL_MAJOR_VERSION >= 9
        Tcl_CreateObjCommand2(interp, "lazy", lazy_command, NULL, NULL);
#else
        Tcl_CreateObjCommand(interp, "lazy", lazy_command, NULL, NULL);
#endif
#endif
        printf("INITIALISE|%d|%d\n", row, initialise);
        if (initialise != 0) return 2;
        observe(interp, "version", "info patchlevel");
        if (observe(interp, "setup", setups[row]) == 0) {
            observe(interp, "worker", workers[row]);
            observe(interp, "exports", "namespace eval ::d {namespace export}");
            observe(interp, "commands", "list [info commands ::d::a] [info commands ::d::b] [info commands ::d::p]");
            if (row == 11) observe(interp, "invoke", "::d::p");
        }
#ifdef JIM_PROBE
        Jim_FreeInterp(interp);
#else
        Tcl_DeleteInterp(interp);
#endif
        printf("CLOSED_CASE|%d\n", row);
    }
    puts("CLOSED_ORIGINAL_NAMING_ARGUMENT_ORDER");
    return 0;
}
