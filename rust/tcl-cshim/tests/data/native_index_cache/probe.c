#include <tcl.h>
#include <stdio.h>
#if TCL_MAJOR_VERSION >= 9
#define INDEX Tcl_Size
#else
#define INDEX int
#endif
static const char *first[] = {"alpha", "beta", NULL};
static const char *other[] = {"alpha", "beta", NULL};
static void row(const char *name, int code, INDEX index, Tcl_Obj *value) {
    printf("{\"case\":\"%s\",\"code\":%d,\"index\":%ld,\"type\":\"%s\",\"resident\":%d,\"hex\":\"", name, code, (long)index,
        value->typePtr ? value->typePtr->name : "none", value->bytes != NULL);
    if (value->bytes) for (int i=0; i<value->length; ++i) printf("%02x", (unsigned char)value->bytes[i]);
    puts("\"}");
}
int main(int argc, char **argv) {
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *ip=Tcl_CreateInterp();
    Tcl_Obj *value=Tcl_NewStringObj("a",1); Tcl_IncrRefCount(value);
    INDEX index=-9; int code=Tcl_GetIndexFromObj(ip,value,first,"option",0,&index);
    row("prefix",code,index,value);
    index=-9; code=Tcl_GetIndexFromObj(ip,value,first,"option",TCL_EXACT,&index);
    row("cached-exact",code,index,value);
    index=-9; code=Tcl_GetIndexFromObj(ip,value,other,"option",TCL_EXACT,&index);
    row("different-table-exact",code,index,value);
    first[0]="changed"; Tcl_InvalidateStringRep(value); (void)Tcl_GetString(value);
    row("resident-from-retained-table",TCL_OK,0,value);
    Tcl_Obj *copy=Tcl_DuplicateObj(value); Tcl_IncrRefCount(copy);
    first[0]="again"; Tcl_InvalidateStringRep(copy); (void)Tcl_GetString(copy);
    row("duplicate-retains-table",TCL_OK,0,copy);
#if TCL_MAJOR_VERSION >= 9
    Tcl_Obj *temporary=Tcl_NewStringObj("a",1); Tcl_IncrRefCount(temporary);
    other[0]="alpha"; index=-9;
    code=Tcl_GetIndexFromObj(ip,temporary,other,"option",64,&index);
    row("temporary-table",code,index,temporary);
    Tcl_DecrRefCount(temporary);
#endif
    Tcl_DecrRefCount(copy); Tcl_DecrRefCount(value);
    Tcl_DeleteInterp(ip); Tcl_Finalize(); return 0;
}
