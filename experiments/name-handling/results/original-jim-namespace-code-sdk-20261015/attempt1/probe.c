#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Obj *observed_namespace;
static const char *label;
static const char *payload;
static int payload_length;
static void update_argument(Jim_Obj *object) {
    printf("NAMESPACE_AT_ARG_GETTER|%s|%d|%s\n", label, observed_namespace->bytes != NULL,
        observed_namespace->typePtr ? observed_namespace->typePtr->name : "none");
    object->bytes = Jim_Alloc(payload_length + 1);
    memcpy(object->bytes, payload, payload_length);
    object->bytes[payload_length] = 0;
    object->length = payload_length;
}
static const Jim_ObjType argument_type = {"code-argument-probe", NULL, NULL, update_argument, JIM_TYPE_NONE};
static Jim_Interp *fresh(void) {
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    return Jim_InitStaticExtensions(interp) == JIM_OK ? interp : NULL;
}
static void sample(const char *kind, Jim_Obj *object) {
    printf("%s|%s|%d|%s\n", kind, label, object->bytes != NULL,
        object->typePtr ? object->typePtr->name : "none");
}
static void result_bytes(int code, Jim_Obj *object) {
    int length;
    const char *bytes = Jim_GetString(object, &length);
    printf("RESULT|%s|%d|", label, code);
    for (int n = 0; n < length; n++) printf("%02x", (unsigned char)bytes[n]);
    puts("");
}
int main(void) {
    Jim_Interp *version = fresh();
    if (!version) return 2;
    label = "VERSION";
    if (Jim_Eval(version, "info patchlevel") != JIM_OK) return 3;
    result_bytes(0, Jim_GetResult(version));
    Jim_FreeInterp(version);
    static const char plain[] = "return VALUE";
    static const char scoped[] = "::namespace inscope 7 {return VALUE}";
    static const char plain_nul[] = "return VALUE\0TAIL";
    static const char scoped_nul[] = "::namespace inscope 7 {return VALUE}\0TAIL";
    const struct { const char *name, *bytes; int length, direct, argc, change; } cases[] = {
        {"PUBLIC_PLAIN", plain, sizeof(plain)-1, 0, 1, 0},
        {"PUBLIC_SCOPED", scoped, sizeof(scoped)-1, 0, 1, 0},
        {"PUBLIC_PLAIN_NUL", plain_nul, sizeof(plain_nul)-1, 0, 1, 0},
        {"PUBLIC_SCOPED_NUL", scoped_nul, sizeof(scoped_nul)-1, 0, 1, 0},
        {"DIRECT_PLAIN", plain, sizeof(plain)-1, 1, 1, 0},
        {"DIRECT_SCOPED", scoped, sizeof(scoped)-1, 1, 1, 0},
        {"DIRECT_PLAIN_NUL", plain_nul, sizeof(plain_nul)-1, 1, 1, 0},
        {"DIRECT_SCOPED_NUL", scoped_nul, sizeof(scoped_nul)-1, 1, 1, 0},
        {"PUBLIC_ARITY_ZERO", plain, sizeof(plain)-1, 0, 0, 0},
        {"PUBLIC_ARITY_TWO", plain, sizeof(plain)-1, 0, 2, 0},
        {"DIRECT_ARITY_ZERO", plain, sizeof(plain)-1, 1, 0, 0},
        {"DIRECT_ARITY_TWO", plain, sizeof(plain)-1, 1, 2, 0},
        {"PUBLIC_REPLACED", plain, sizeof(plain)-1, 0, 1, 1},
        {"DIRECT_REPLACED", plain, sizeof(plain)-1, 1, 1, 1},
        {"PUBLIC_DELETED", plain, sizeof(plain)-1, 0, 1, 2},
        {"DIRECT_DELETED", plain, sizeof(plain)-1, 1, 1, 2}
    };
    for (unsigned n = 0; n < sizeof(cases)/sizeof(cases[0]); n++) {
        Jim_Interp *interp = fresh();
        if (!interp) return 4;
        if (cases[n].change && Jim_Eval(interp, cases[n].change == 1
            ? "rename {namespace code} saved; proc {namespace code} arg {return REPLACED}"
            : "rename {namespace code} {}") != JIM_OK) return 5;
        Jim_Obj *ns = Jim_NewIntObj(interp, 42); Jim_IncrRefCount(ns);
        observed_namespace = ns; payload = cases[n].bytes; payload_length = cases[n].length;
        Jim_Obj *arg = Jim_NewStringObj(interp, "", 0); Jim_IncrRefCount(arg);
        Jim_InvalidateStringRep(arg); arg->typePtr = &argument_type;
        Jim_Obj *elements[4];
        int argc = 0;
        elements[argc++] = Jim_NewStringObj(interp, cases[n].direct ? "namespace code" : "namespace", -1);
        if (!cases[n].direct) elements[argc++] = Jim_NewStringObj(interp, "code", -1);
        for (int j = 0; j < cases[n].argc; j++) elements[argc++] = arg;
        Jim_Obj *body = Jim_NewListObj(interp, elements, argc); Jim_IncrRefCount(body);
        for (int hot = 0; hot < 2; hot++) {
            char phase[128]; snprintf(phase, sizeof(phase), "%s_%s", cases[n].name, hot ? "HOT" : "COLD"); label = phase;
            if (hot) { Jim_InvalidateStringRep(ns); Jim_InvalidateStringRep(arg); }
            sample("NAMESPACE_BEFORE", ns); sample("ARG_BEFORE", arg);
            int code = Jim_EvalNamespace(interp, body, ns);
            sample("NAMESPACE_AFTER", ns); sample("ARG_AFTER", arg);
            Jim_Obj *result = Jim_GetResult(interp);
            printf("RESULT_IS_ARG|%s|%d\n", label, result == arg);
            if (code == JIM_OK && result != arg && result->typePtr && !strcmp(result->typePtr->name, "list")) {
                int count; Jim_ListLength(interp, result, &count);
                Jim_Obj *child = count == 4 ? Jim_ListGetIndex(interp, result, 3) : NULL;
                printf("RESULT_LIST|%s|%d|%d\n", label, count, child == arg);
            }
            result_bytes(code, result);
        }
        Jim_DecrRefCount(interp, body); Jim_DecrRefCount(interp, arg); Jim_DecrRefCount(interp, ns);
        Jim_FreeInterp(interp);
    }
    return 0;
}
