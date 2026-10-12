#include <tcl.h>
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
#define SIZE Tcl_Size
#else
#define SIZE int
#endif
int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *ip = Tcl_CreateInterp();
    const char bytes[] = {'v', 0, (char)255};
    Tcl_Obj *child = Tcl_NewStringObj(bytes, 3);
    Tcl_IncrRefCount(child);
    Tcl_Obj *members[] = {child, child};
    Tcl_Obj *list = Tcl_NewListObj(2, members);
    Tcl_IncrRefCount(list);
    SIZE count;
    Tcl_Obj **items;
    int list_code = Tcl_ListObjGetElements(NULL, list, &count, &items);
    Tcl_SetObjResult(ip, list);
    int result_same = Tcl_GetObjResult(ip) == list;
    int members_same = list_code == TCL_OK && count == 2 && items[0] == child && items[1] == child;
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    Tcl_Obj *options = Tcl_GetReturnOptions(ip, TCL_ERROR);
    Tcl_IncrRefCount(options);
    Tcl_Obj *key = Tcl_NewStringObj("-errorinfo", -1), *info = NULL;
    Tcl_IncrRefCount(key);
    int lookup = Tcl_DictObjGet(NULL, options, key, &info);
    printf("{\"result_same\":%d,\"members_same\":%d,\"errorinfo_same\":%d}\n",
        result_same, members_same, lookup == TCL_OK && info == list);
    Tcl_DecrRefCount(key);
    Tcl_DecrRefCount(options);
#else
    printf("{\"result_same\":%d,\"members_same\":%d,\"errorinfo_same\":null}\n",
        result_same, members_same);
#endif
    Tcl_DecrRefCount(list);
    Tcl_DecrRefCount(child);
    Tcl_DeleteInterp(ip);
    Tcl_Finalize();
    return 0;
}
