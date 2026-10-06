// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tcl.h>
#include <stdio.h>
static Tcl_Interp *active;
static Tcl_Size method_calls;
static int method_status = TCL_OK;
static Tcl_Size MutatingLength(Tcl_Obj *obj) {
    (void)obj;
    method_calls++;
    Tcl_SetVar(active, "world", "after", TCL_GLOBAL_ONLY);
    return 2;
}
static Tcl_Obj *elements[2];
static int Elements(Tcl_Interp *interp, Tcl_Obj *obj, Tcl_Size *length, Tcl_Obj ***values) {
    (void)interp; (void)obj;
    Tcl_SetVar(active, "elementsSeen", "yes", TCL_GLOBAL_ONLY);
    *length = 2; *values = elements; return TCL_OK;
}
static void StringRep(Tcl_Obj *obj) {
    Tcl_InitStringRep(obj, "a b", 3);
}
static int MutatingIndex(Tcl_Interp *interp, Tcl_Obj *obj, Tcl_Size index, Tcl_Obj **result) {
    (void)obj; (void)index;
    Tcl_SetVar(interp, "world", "index", TCL_GLOBAL_ONLY);
    if (method_status != TCL_OK) {
        Tcl_SetObjResult(interp, Tcl_NewStringObj("INDEX_STATUS", -1));
        *result = NULL;
        return method_status;
    }
    *result = Tcl_NewStringObj("element", -1);
    return TCL_OK;
}
static int MutatingSlice(Tcl_Interp *interp, Tcl_Obj *obj, Tcl_Size first, Tcl_Size last, Tcl_Obj **result) {
    (void)obj; (void)first; (void)last;
    Tcl_SetVar(interp, "world", "slice", TCL_GLOBAL_ONLY);
    if (method_status != TCL_OK) {
        Tcl_SetObjResult(interp, Tcl_NewStringObj("SLICE_STATUS", -1));
        *result = NULL;
        return method_status;
    }
    *result = Tcl_NewStringObj("sliced", -1);
    return TCL_OK;
}
static void Duplicate(Tcl_Obj *src, Tcl_Obj *copy) {
    copy->typePtr = src->typePtr;
    Tcl_SetVar(active, "duplicateSeen", "yes", TCL_GLOBAL_ONLY);
}
static const Tcl_ObjType custom = {
    "length_effect_probe", NULL, Duplicate, StringRep, NULL,
    TCL_OBJTYPE_V2(MutatingLength, MutatingIndex, MutatingSlice, NULL, Elements, NULL, NULL, NULL)
};
static int MakeObject(void *unused, Tcl_Interp *interp, Tcl_Size objc, Tcl_Obj *const objv[]) {
    (void)unused; (void)objv;
    method_status = TCL_OK;
    if (objc == 2) { if (Tcl_GetIntFromObj(interp, objv[1], &method_status) != TCL_OK) return TCL_ERROR; }
    else if (objc != 1) return TCL_ERROR;
    active = interp;
    Tcl_Obj *obj = Tcl_NewObj();
    Tcl_InvalidateStringRep(obj);
    obj->typePtr = &custom;
    Tcl_SetObjResult(interp, obj);
    return TCL_OK;
}
static void Probe(Tcl_Interp *interp, const char *label, const char *script) {
    int status = Tcl_Eval(interp, script);
    printf("%s|%d|%s|%lld\n", label, status, Tcl_GetStringResult(interp), (long long)method_calls);
}
int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    Tcl_CreateObjCommand2(interp, "nativeobj", MakeObject, NULL, NULL);
    elements[0] = Tcl_NewIntObj(0); elements[1] = Tcl_NewIntObj(1);
    Tcl_IncrRefCount(elements[0]); Tcl_IncrRefCount(elements[1]);
    Probe(interp, "generic_value_list_duplicate", "set duplicateSeen no; set elementsSeen no; set world before; set object [nativeobj]; set command foreach; $command x $object {}; list $duplicateSeen $elementsSeen $world");
    Probe(interp, "generic_variable_list_methods", "set duplicateSeen no; set elementsSeen no; set world before; set object [nativeobj]; set command foreach; $command $object {a b} {}; list $duplicateSeen $elementsSeen $world");
    Probe(interp, "grouped_index_list_methods", "set duplicateSeen no; set elementsSeen no; set world before; set object [nativeobj]; set code [catch {lindex {{a b} {c d}} $object} result]; list $code $result $duplicateSeen $elementsSeen $world");
    Probe(interp, "compiled_iteration_methods", "set duplicateSeen no; set elementsSeen no; set world before; set object [nativeobj]; foreach x $object {}; list $duplicateSeen $elementsSeen $world");
    Tcl_DecrRefCount(elements[0]); Tcl_DecrRefCount(elements[1]);
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
