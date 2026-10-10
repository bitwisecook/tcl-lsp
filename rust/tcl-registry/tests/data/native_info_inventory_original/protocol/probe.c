#include <stdio.h>
#include <stdlib.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
typedef int Size;
static Interp *create(void) {
    Interp *i = Jim_CreateInterp();
    Jim_RegisterCoreCommands(i);
    if (Jim_InitStaticExtensions(i) != JIM_OK) { Jim_FreeInterp(i); return NULL; }
    return i;
}
static void destroy(Interp *i) { Jim_FreeInterp(i); }
static Obj *word(Interp *i, const char *p) { return Jim_NewStringObj(i, p, -1); }
static void hold(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { Jim_DecrRefCount(i, o); }
static int invoke(Interp *i, int n, Obj **v) { return Jim_EvalObjVector(i, n, v); }
static Obj *result(Interp *i) { return Jim_GetResult(i); }
static const char *bytes(Obj *o, Size *n) { return Jim_GetString(o, n); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static Interp *create(void) {
    Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) { Tcl_DeleteInterp(i); return NULL; }
    return i;
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
static Obj *word(Interp *i, const char *p) { (void)i; return Tcl_NewStringObj(p, -1); }
static void hold(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i, int n, Obj **v) { return Tcl_EvalObjv(i, n, v, TCL_EVAL_DIRECT); }
static Obj *result(Interp *i) { return Tcl_GetObjResult(i); }
static const char *bytes(Obj *o, Size *n) { return Tcl_GetStringFromObj(o, n); }
#endif
struct Case { int argc; const char *argv[12]; };
static const struct Case cases[] = {
    {2, {"info", "__r2286_absent__"}},
    {2, {"info", "-commands"}},
    {5, {"namespace", "ensemble", "configure", "info", "-map"}},
    {3, {"info", "exists", "__r2286_inventory_missing__"}},
    {12, {"info", "alias", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "aliases", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "args", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "body", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "channels", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "class", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "cmdcount", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "cmdtype", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "commands", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "complete", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "constant", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "consts", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "coroutine", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "default", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "errorstack", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "exists", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "frame", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "functions", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "globals", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "help", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "hostname", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "level", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "library", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "loaded", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "locals", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "nameofexecutable", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "object", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "patchlevel", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "procs", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "references", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "returncodes", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "script", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "sharedlibextension", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "source", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "stacktrace", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "statics", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "tainted", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "tclversion", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "usage", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "vars", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "version", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "co", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "cor", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "cl", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "cm", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "sta", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "vers", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}},
    {12, {"info", "", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__", "__r2286_operand__"}}
};
static void row(Interp *i, const char *tag, int n, const char *const *words) {
    Obj *v[12];
    for (int k = 0; k < n; ++k) { v[k] = word(i, words[k]); hold(v[k]); }
    int code = invoke(i, n, v);
    Obj *r = result(i); hold(r);
    Size size; const char *p = bytes(r, &size);
    printf("%s|%d|%lld|", tag, code, (long long)size);
    for (Size k = 0; k < size; ++k) printf("%02x", (unsigned char)p[k]);
    puts("");
    release(i, r);
    for (int k = 0; k < n; ++k) release(i, v[k]);
}
int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc != 2) return 2;
    char *end; long chosen = strtol(argv[1], &end, 10);
    if (*end || chosen < 0 || chosen >= (long)(sizeof(cases) / sizeof(cases[0]))) return 3;
#ifndef JIM_PROBE
    Tcl_FindExecutable("info-inventory-object-vector");
#endif
    Interp *i = create(); if (!i) return 4;
    const char *version[] = {"info", "patchlevel"}; row(i, "VERSION", 2, version);
    row(i, "INFO", cases[chosen].argc, cases[chosen].argv);
    destroy(i);
#ifndef JIM_PROBE
    Tcl_Finalize();
#endif
    return 0;
}
