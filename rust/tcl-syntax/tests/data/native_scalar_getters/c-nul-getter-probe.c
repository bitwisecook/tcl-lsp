/* Distinguish raw host string bytes from guest bytearray string materialization. */
#include <stdio.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif
static void printhex(const char *bytes, ProbeSize len) { int i; for (i=0; i<len; i++) printf("%02x", (unsigned char)bytes[i]); }
int main(int argc, char **argv) {
    const char *cases[] = {"1\0X", "x\0after", "\xff\0after"};
    const int lengths[] = {3,7,7};
    int i,kind;
    Tcl_FindExecutable(argv[0]);
    for (kind=0;kind<2;kind++) for(i=0;i<3;i++) {
        Tcl_Interp *interp = Tcl_CreateInterp();
        Tcl_Obj *value = kind ? Tcl_NewByteArrayObj((const unsigned char *)cases[i],lengths[i]) : Tcl_NewStringObj(cases[i],lengths[i]);
        Tcl_WideInt wide=0; double number=0; int code; ProbeSize len; const char *bytes;
        Tcl_IncrRefCount(value);
        code = Tcl_GetWideIntFromObj(interp,value,&wide);
        bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&len);
        printf("kind=%s case=%d widecode=%d wide=%lld message=",kind?"bytearray":"rawstring",i,code,(long long)wide); printhex(bytes,len);
        Tcl_DecrRefCount(value);
        value = kind ? Tcl_NewByteArrayObj((const unsigned char *)cases[i],lengths[i]) : Tcl_NewStringObj(cases[i],lengths[i]);
        Tcl_IncrRefCount(value);
        Tcl_ResetResult(interp);
        code=Tcl_GetDoubleFromObj(interp,value,&number);
        bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&len);
        printf(" doublecode=%d double=%.17g message=",code,number);printhex(bytes,len);
        bytes=Tcl_GetStringFromObj(value,&len); printf(" input=");printhex(bytes,len);putchar('\n');
        Tcl_DecrRefCount(value);Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize(); return 0;
}
