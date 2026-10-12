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
}
static const Tcl_ObjType custom = {
    "length_effect_probe", NULL, Duplicate, StringRep, NULL,
    TCL_OBJTYPE_V2(MutatingLength, MutatingIndex, MutatingSlice, NULL, NULL, NULL, NULL, NULL)
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
    Probe(interp, "custom_length", "set world before; set object [nativeobj]; set count [llength $object]; list $count $world");
    Probe(interp, "stock_series", "set world before; set object [lseq 1 to 3]; set count [llength $object]; list $count $world");
    Probe(interp, "ordinary_list", "set world before; set object [list a b]; set count [llength $object]; list $count $world");
    Probe(interp, "custom_index", "set world before; set object [nativeobj]; set value [lindex $object 0]; list $value $world");
    Probe(interp, "custom_slice", "set world before; set object [nativeobj]; set value [lrange $object 0 1]; list $value $world");
    Probe(interp, "custom_iteration", "set world before; set object [nativeobj]; set visits {}; foreach x $object {lappend visits $x}; list $visits $world");
    Probe(interp, "index_return_status", "set world before; set object [nativeobj 2]; set code [catch {lindex $object 0} result]; list $code $result $world");
    Probe(interp, "slice_return_status", "set world before; set object [nativeobj 2]; set code [catch {lrange $object 0 1} result]; list $code $result $world");
    Probe(interp, "generic_slice_return_status", "set world before; set object [nativeobj 2]; set command lrange; set code [catch {$command $object 0 1} result]; list $code $result $world");
    Probe(interp, "iteration_return_status", "set world before; set object [nativeobj 2]; set code [catch {foreach x $object {set visit $x}} result]; list $code $result $world");
    Probe(interp, "replaced_creator", "rename lseq original_lseq; proc lseq args {nativeobj}; set world before; set object [lseq 1 to 3]; set count [llength $object]; list $count $world");
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
