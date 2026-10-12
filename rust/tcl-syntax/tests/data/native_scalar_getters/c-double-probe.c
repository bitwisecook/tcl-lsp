/* Direct pinned C scalar GetDouble grammar, result bits, and retained cache. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "tcl.h"
int main(int argc, char **argv) {
    const char *cases[] = {
        "0755", "0x10", "0x1p4", "0x1.8p2", "0b10", "0o10", "0d10",
        "nan(foo)", "NaN(0x1)", "1e400", "1e-400", "18446744073709551615",
        "18446744073709551616", "-18446744073709551615", "-18446744073709551616",
        "9223372036854775808", " 1 ", "1_0", "true", "0x1.fffffffffffffp1023",
        "0x1p-1074", "0x1p-1075", "0x1.8p-1075", "-0", "-0.0", "0x1."
    };
    unsigned i;
    Tcl_FindExecutable(argv[0]);
    for (i=0;i<sizeof(cases)/sizeof(cases[0]);i++) {
        Tcl_Interp *interp=Tcl_CreateInterp();
        Tcl_Obj *input=Tcl_NewStringObj(cases[i],-1);
        double result=0;
        union { double d; uint64_t bits; } value;
        int code;
        Tcl_IncrRefCount(input);
        code=Tcl_GetDoubleFromObj(interp,input,&result);
        value.d=result;
        printf("%s\tcode=%d\ttype=%s\tbits=%016" PRIx64 "\n", cases[i],code,input->typePtr?input->typePtr->name:"string",value.bits);
        Tcl_DecrRefCount(input);
        Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();
    return 0;
}
