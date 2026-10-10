#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if (Jim_InitStaticExtensions(i) != JIM_OK) return NULL; return i; }
static Obj *string(Interp *i, const char *p, int n) { return Jim_NewStringObj(i, p, n); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { Jim_DecrRefCount(i, o); }
static int invoke(Interp *i, Obj **v) { return Jim_EvalObjVector(i, 3, v); }
static int source(Interp *i, const char *p, int n) { Obj *o = string(i, p, n); retain(o); int c = Jim_EvalObj(i, o); release(i, o); return c; }
static void result(Interp *i, const char *label, int c) { int n; const char *p = Jim_GetString(Jim_GetResult(i), &n); printf("%s|%d|", label, c); for (int k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); puts(""); }
static void options(Interp *i, const char *label, int c) { (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-C-return-options-api\n", label, c); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i = Tcl_CreateInterp(); if (Tcl_Init(i) != TCL_OK) return NULL; return i; }
static Obj *string(Interp *i, const char *p, int n) { (void)i; return Tcl_NewStringObj(p, n); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i, Obj **v) { return Tcl_EvalObjv(i, 3, v, 0); }
static int source(Interp *i, const char *p, int n) { return Tcl_EvalEx(i, p, n, 0); }
static void bytes(const char *p, Count n) { for (Count k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); }
static void result(Interp *i, const char *label, int c) { Count n; const char *p = Tcl_GetStringFromObj(Tcl_GetObjResult(i), &n); printf("%s|%d|", label, c); bytes(p, n); puts(""); }
static void options(Interp *i, const char *label, int c) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-Tcl_GetReturnOptions\n", label, c);
#else
    Obj *o = Tcl_GetReturnOptions(i, c); retain(o); Count n; const char *p = Tcl_GetStringFromObj(o, &n); printf("%s_OPTIONS|%d|", label, c); bytes(p, n); puts(""); release(i, o);
#endif
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif

static void input(const char *label, const char *p, int n) { printf("%s_INPUT|0|", label); for (int k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); puts(""); }

int main(int argc, char **argv) {
    (void)argc;
#ifndef JIM_PROBE
    Tcl_FindExecutable(argv[0]);
#else
    (void)argv;
#endif
    Interp *i = fresh(); if (!i) return 2;
    int c = source(i, "info patchlevel", 15); result(i, "VERSION", c); destroy(i);
    const char *types[] = {"var", "com", "exec"};
    const char *members[] = {"info", "add"};
    const char *modes[] = {"DIRECT_ASCII", "DIRECT_RAW_ZERO", "DIRECT_ENCODED_ZERO", "ASCII_SOURCE", "COUNTED_SOURCE_RAW_ZERO"};
    for (int t = 0; t < 3; t++) for (int m = 0; m < 2; m++) for (int mode = 0; mode < 5; mode++) {
        char label[100], operand[20], script[60]; int n = (int)strlen(types[t]);
        memcpy(operand, types[t], n);
        if (mode == 1 || mode == 4) { operand[n++] = 0; operand[n++] = (char)0xff; }
        if (mode == 2) { operand[n++] = (char)0xc0; operand[n++] = (char)0x80; operand[n++] = (char)0xff; }
        snprintf(label, sizeof(label), "%s_%s_%s", modes[mode], types[t], members[m]);
        i = fresh(); if (!i) return 2;
        if (mode < 3) {
            input(label, operand, n);
            Obj *v[] = {string(i, "trace", 5), string(i, members[m], (int)strlen(members[m])), string(i, operand, n)};
            for (int k = 0; k < 3; k++) retain(v[k]);
            c = invoke(i, v); result(i, label, c); options(i, label, c);
            for (int k = 0; k < 3; k++) release(i, v[k]);
        } else {
            int prefix = snprintf(script, sizeof(script), "trace %s ", members[m]);
            memcpy(script + prefix, operand, n); script[prefix + n] = 0;
            input(label, script, prefix + n);
            c = source(i, script, prefix + n); result(i, label, c); options(i, label, c);
        }
        destroy(i);
    }
#ifndef JIM_PROBE
    Tcl_Finalize();
#endif
    return 0;
}
