#include <tcl.h>
#include <stdio.h>
#include <string.h>
static const char *const table[] = {"provide", "present", NULL};
static void row(const char *stage, int code, int index, Tcl_Obj *obj, char *bytes) {
    printf("%s\t%d\t%d\t%s\t%d\t%d\n", stage, code, index,
        obj->typePtr ? obj->typePtr->name : "none", obj->bytes != NULL,
        obj->bytes == bytes);
}
int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]); Tcl_Interp *i=Tcl_CreateInterp(); int n=-1,code;
    Tcl_Obj *o=Tcl_NewStringObj("pro",3); Tcl_IncrRefCount(o); char *bytes=o->bytes;
    code=Tcl_GetIndexFromObj(i,o,table,"option",0,&n); row("abbreviated",code,n,o,bytes);
    n=-1; code=Tcl_GetIndexFromObj(i,o,table,"option",TCL_EXACT,&n); row("cached-exact",code,n,o,bytes);
    Tcl_InvalidateStringRep(o); n=-1;
    code=Tcl_GetIndexFromObj(i,o,table,"option",TCL_EXACT,&n); row("cached-absent",code,n,o,NULL);
    Tcl_Obj *dup=Tcl_DuplicateObj(o); Tcl_IncrRefCount(dup);
    n=-1; code=Tcl_GetIndexFromObj(i,dup,table,"option",TCL_EXACT,&n); row("duplicate-absent",code,n,dup,NULL);
    Tcl_DecrRefCount(dup); Tcl_DecrRefCount(o);
    static const char raw[]={'p','r','o','v','i','d','e',0,'j','u','n','k'};
    o=Tcl_NewStringObj(raw,sizeof(raw)); Tcl_IncrRefCount(o); bytes=o->bytes; n=-1;
    code=Tcl_GetIndexFromObj(i,o,table,"option",0,&n); row("counted-nul",code,n,o,bytes); Tcl_DecrRefCount(o);
    Tcl_DeleteInterp(i); Tcl_Finalize(); return 0;
}
