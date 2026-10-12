#include <tcl.h>
#include <stdio.h>

#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif

static const char *primary(Tcl_Obj *value) {
    return value->typePtr == NULL ? "NULL" : value->typePtr->name;
}

static void observe(const char *label, Tcl_Obj *value) {
    const char *before;
    const char *after;
    const unsigned char *bytes;
    Count length;
    Count index;
    Tcl_IncrRefCount(value);
    before = primary(value);
    printf("B|%s|%s|%d\n", label, before, value->bytes != NULL);
    Tcl_AppendToObj(value, "", 0);
    after = primary(value);
    printf("A|%s|%s|%d|", label, after, value->bytes != NULL);
    bytes = (const unsigned char *)Tcl_GetStringFromObj(value, &length);
    printf("%lld|", (long long)length);
    for (index = 0; index < length; index++) {
        printf("%02x", bytes[index]);
    }
    printf("\n");
    Tcl_DecrRefCount(value);
}

int main(int argc, char **argv) {
    Tcl_Interp *interp;
    Tcl_Obj *prepared;
    Tcl_UniChar units[] = {0xd800};
    const char opaque[] = {(char)0xff, 0, ':', ':', 't'};
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    interp = Tcl_CreateInterp();
    if (Tcl_Init(interp) != TCL_OK) {
        fprintf(stderr, "init: %s\n", Tcl_GetStringResult(interp));
        return 1;
    }
    printf("V|%s\n", Tcl_GetVar(interp, "tcl_patchLevel", TCL_GLOBAL_ONLY));
    observe("fresh-empty", Tcl_NewStringObj(NULL, 0));
    observe("fresh-tail", Tcl_NewStringObj("a::", 3));
    observe("fresh-counted", Tcl_NewStringObj(opaque, 5));
    observe("integer", Tcl_NewIntObj(5));
    observe("unicode", Tcl_NewUnicodeObj(units, 1));
    prepared = Tcl_NewStringObj("existing", 8);
    (void)Tcl_GetCharLength(prepared);
    observe("prepared-string", prepared);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
