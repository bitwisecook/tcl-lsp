#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static unsigned calls;
static void hex(const char *value, Size length) { for (Size i = 0; i < length; i++) printf("%02x", (unsigned char)value[i]); }
static int watch(ClientData data, Tcl_Interp *interp, int argc, Tcl_Obj *const *argv) {
    (void)data; calls++;
    printf("callback\t%u\t%d", calls, argc);
    for (int i = 1; i < argc; i++) { Size length; const char *value = Tcl_GetStringFromObj(argv[i], &length); putchar('\t'); hex(value, length); }
    putchar('\n'); Tcl_ResetResult(interp); return TCL_OK;
}
static int command(Tcl_Interp *interp, Tcl_Obj *prefix, int add) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    const char *text[] = {"trace", add ? "variable" : "vdelete", "v", "w"};
#else
    const char *text[] = {"trace", add ? "add" : "remove", "variable", "v", "write"};
#endif
    Tcl_Obj *words[6]; int count = (int)(sizeof(text) / sizeof(text[0]));
    for (int i = 0; i < count; i++) { words[i] = Tcl_NewStringObj(text[i], -1); Tcl_IncrRefCount(words[i]); }
    words[count] = prefix; Tcl_IncrRefCount(prefix);
    int code = Tcl_EvalObjv(interp, count + 1, words, 0);
    for (int i = 0; i <= count; i++) Tcl_DecrRefCount(words[i]);
    return code;
}
static int info(Tcl_Interp *interp) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    return Tcl_Eval(interp, "trace vinfo v");
#else
    return Tcl_Eval(interp, "trace info variable v");
#endif
}
static void report(Tcl_Interp *interp, const char *label) {
    int code = info(interp); Size count = 0, length = 0; Tcl_Obj *entry = NULL, *prefix = NULL;
    if (code == TCL_OK && Tcl_ListObjLength(interp, Tcl_GetObjResult(interp), &count) != TCL_OK) code = TCL_ERROR;
    if (code == TCL_OK && count) {
        if (Tcl_ListObjIndex(interp, Tcl_GetObjResult(interp), 0, &entry) != TCL_OK || Tcl_ListObjIndex(interp, entry, 1, &prefix) != TCL_OK) code = TCL_ERROR;
    }
    printf("info\t%s\t%d\t%ld\t", label, code, (long)count);
    if (prefix) { const char *value = Tcl_GetStringFromObj(prefix, &length); hex(value, length); }
    putchar('\n');
}
#endif
int main(int argc, char **argv) {
    (void)argc;
#ifdef JIM_PROBE
    (void)argv;
    Jim_Interp *interp = Jim_CreateInterp(); Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    int code = Jim_Eval(interp, "trace add variable v write watch");
    int length; const char *value = Jim_GetString(Jim_GetResult(interp), &length);
    printf("availability\t%d\t", code); for (int i = 0; i < length; i++) printf("%02x", (unsigned char)value[i]); puts("");
    Jim_FreeInterp(interp);
#else
    Tcl_FindExecutable(argv[0]);
    static const char raw[] = {'w','a','t','c','h',' ','A',0,'X'};
    static const char same[] = {'w','a','t','c','h',' ','A',0,'Y'};
    static const char longer[] = {'w','a','t','c','h',' ','A',0,'Y','Y'};
    static const char encoded[] = {'w','a','t','c','h',' ','A',(char)0xc0,(char)0x80,'X'};
    const char *remove[] = {same, longer, encoded}; Size lengths[] = {sizeof(same), sizeof(longer), sizeof(encoded)};
    for (int test = 0; test < 3; test++) {
        calls = 0; Tcl_Interp *interp = Tcl_CreateInterp(); if (Tcl_Init(interp) != TCL_OK) return 2;
        Tcl_CreateObjCommand(interp, "watch", watch, NULL, NULL);
        Tcl_Obj *prefix = Tcl_NewStringObj(raw, sizeof(raw)); Tcl_IncrRefCount(prefix);
        int code = command(interp, prefix, 1); printf("add\t%d\t%d\tprefixRefs=%d\n", test, code, prefix->refCount);
        Tcl_SetStringObj(prefix, "watch MUTATED", -1); report(interp, "copied");
        code = Tcl_Eval(interp, "set v VALUE"); printf("fire\t%d\t%d\tcalls=%u\n", test, code, calls);
        Tcl_Obj *other = Tcl_NewStringObj(remove[test], lengths[test]); Tcl_IncrRefCount(other);
        code = command(interp, other, 0); printf("remove\t%d\t%d\n", test, code); report(interp, "remaining");
        Tcl_DecrRefCount(other); Tcl_DecrRefCount(prefix); Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();
#endif
    return 0;
}
