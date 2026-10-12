#include <stdio.h>
#include <string.h>
#include <tcl.h>

static char visits[32];
static int visitCount;
static const char *rootText;
static void updateRoot(Tcl_Obj *obj) {
    size_t n = strlen(rootText);
    visits[visitCount++] = 'R';
    obj->bytes = Tcl_Alloc((unsigned)n + 1);
    memcpy(obj->bytes, rootText, n + 1);
    obj->length = (int)n;
}
static void updateIndex(Tcl_Obj *obj) {
    visits[visitCount++] = 'I';
    obj->bytes = Tcl_Alloc(2);
    memcpy(obj->bytes, "k", 2);
    obj->length = 1;
}
static Tcl_ObjType rootType = {"originalRootGetterProbe", NULL, NULL, updateRoot, NULL};
static Tcl_ObjType indexType = {"originalIndexGetterProbe", NULL, NULL, updateIndex, NULL};

int main(int argc, char **argv) {
    int shape, quiet;
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    for (shape = 0; shape != 5; ++shape) {
        for (quiet = 0; quiet != 2; ++quiet) {
            Tcl_Interp *interp = Tcl_CreateInterp();
            Tcl_Obj *root, *index, *value, *selected;
            const char *shapes[] = {"missing-namespace", "scalar-root", "array-root", "fresh-combined-root", "cached-combined-root"};
            if (shape == 0) rootText = "::missing::arr";
            else if (shape == 1) {rootText = "scalar"; Tcl_SetVar(interp, "scalar", "BASE", 0);}
            else {rootText = "arr"; Tcl_SetVar(interp, "arr(seed)", "BASE", 0);}
            if (shape < 3) {
                root = Tcl_NewObj();
                Tcl_InvalidateStringRep(root);
                root->typePtr = &rootType;
            } else {
                root = Tcl_NewStringObj("arr(inner)", -1);
            }
            Tcl_IncrRefCount(root);
            if (shape == 4) {
                Tcl_Obj *seed = Tcl_NewStringObj("BASE", -1);
                Tcl_IncrRefCount(seed);
                Tcl_ObjSetVar2(interp, root, NULL, seed, TCL_LEAVE_ERR_MSG);
                Tcl_DecrRefCount(seed);
            }
            index = Tcl_NewObj();
            Tcl_InvalidateStringRep(index);
            index->typePtr = &indexType;
            value = Tcl_NewStringObj("NEXT", -1);
            Tcl_IncrRefCount(index);
            Tcl_IncrRefCount(value);
            Tcl_SetObjResult(interp, Tcl_NewStringObj("SENTINEL", -1));
            visitCount = 0;
            visits[0] = 0;
            printf("before|shape=%s|purpose=%s|root-type=%s|root-refs=%d|root-string=%s|index-type=%s|index-refs=%d|index-string=%s\n", shapes[shape], quiet ? "QuietWrite" : "Write", root->typePtr ? root->typePtr->name : "NULL", root->refCount, root->bytes ? "yes" : "no", index->typePtr ? index->typePtr->name : "NULL", index->refCount, index->bytes ? "yes" : "no");
            selected = Tcl_ObjSetVar2(interp, root, index, value, quiet ? 0 : TCL_LEAVE_ERR_MSG);
            visits[visitCount] = 0;
            printf("shape=%s|purpose=%s|selected=%s|visits=%s|result=%s\n", shapes[shape], quiet ? "QuietWrite" : "Write", selected ? "some" : "none", visits, Tcl_GetStringResult(interp));
            printf("after|shape=%s|purpose=%s|root-type=%s|root-refs=%d|root-string=%s|index-type=%s|index-refs=%d|index-string=%s\n", shapes[shape], quiet ? "QuietWrite" : "Write", root->typePtr ? root->typePtr->name : "NULL", root->refCount, root->bytes ? "yes" : "no", index->typePtr ? index->typePtr->name : "NULL", index->refCount, index->bytes ? "yes" : "no");
            Tcl_DecrRefCount(value);
            Tcl_DecrRefCount(index);
            Tcl_DecrRefCount(root);
            Tcl_DeleteInterp(interp);
        }
    }
    Tcl_Finalize();
    return 0;
}
