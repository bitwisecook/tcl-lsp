// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tcl.h>
#include <stdio.h>
static Tcl_Interp *active;
static int lengths, elements, duplicates;
static int length_only;
static Tcl_Size Length(Tcl_Obj *obj) {
    (void)obj; lengths++;
    Tcl_SetVar(active, "r", "LENGTH", 0);
    return 0;
}
static int Elements(Tcl_Interp *interp, Tcl_Obj *obj, Tcl_Size *count, Tcl_Obj ***values) {
    (void)obj; elements++;
    if (!length_only) Tcl_SetVar(interp, "r", "ELEMENTS", 0);
    *count = 0; *values = NULL; return TCL_OK;
}
static void Duplicate(Tcl_Obj *src, Tcl_Obj *copy) {
    duplicates++; copy->typePtr = src->typePtr;
    if (!length_only) Tcl_SetVar(active, "r", "DUPLICATE", 0);
}
static void StringRep(Tcl_Obj *obj) {Tcl_InitStringRep(obj, "", 0);}
static const Tcl_ObjType custom = {
    "empty_mutating_list", NULL, Duplicate, StringRep, NULL,
    TCL_OBJTYPE_V2(Length, NULL, NULL, NULL, Elements, NULL, NULL, NULL)
};
static int MakeObject(void *unused, Tcl_Interp *interp, Tcl_Size argc, Tcl_Obj *const argv[]) {
    (void)unused; if (argc > 2) return TCL_ERROR;
    length_only = argc == 2;
    active = interp;
    Tcl_Obj *obj = Tcl_NewObj(); Tcl_InvalidateStringRep(obj);
    obj->typePtr = &custom; Tcl_SetObjResult(interp, obj); return TCL_OK;
}
static void Probe(Tcl_Interp *interp, const char *label, const char *source) {
    lengths = elements = duplicates = 0;
    int code = Tcl_Eval(interp, source);
    printf("%s|%d|%s|%d|%d|%d\n", label, code, Tcl_GetStringResult(interp), lengths, elements, duplicates);
}
int main(int argc, char **argv) {
    (void)argc; Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    Tcl_CreateObjCommand2(interp, "nativeobj", MakeObject, NULL, NULL);
    Tcl_Eval(interp, "proc f {items} {set r SAFE; set ran NO; foreach i $items {set ran YES}; list $r $ran}; proc g {items} {set r SAFE; set ran NO; set iterate foreach; $iterate i $items {set ran YES}; list $r $ran}");
    Probe(interp, "compiled_empty_custom", "f [nativeobj]");
    Probe(interp, "generic_empty_custom", "g [nativeobj]");
    Probe(interp, "compiled_empty_length_only", "f [nativeobj lengthOnly]");
    Probe(interp, "generic_empty_length_only", "g [nativeobj lengthOnly]");
    Probe(interp, "compiled_empty_plain", "f {}");
    Probe(interp, "generic_empty_plain", "g {}");
    Tcl_DeleteInterp(interp); Tcl_Finalize(); return 0;
}
