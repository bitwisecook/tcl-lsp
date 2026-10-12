#include "tcl.h"
#include <stdio.h>
#include <stdlib.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size ProbeSize;
#else
typedef int ProbeSize;
#endif
int main(int argc, char **argv) {
    if (argc != 2) return 64;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) {
        fprintf(stderr, "init: %s\n", Tcl_GetStringResult(i)); return 1;
    }
    FILE *f = fopen(argv[1], "rb");
    if (!f) return 66;
    fseek(f, 0, SEEK_END); long length = ftell(f); rewind(f);
    char *source = malloc((size_t)length + 1);
    if (!source || fread(source, 1, (size_t)length, f) != (size_t)length) return 74;
    fclose(f); source[length] = 0;
    int code = Tcl_EvalEx(i, source, (ProbeSize)length, TCL_EVAL_GLOBAL);
    printf("C_COMPLETION %d\n", code);
    if (code == TCL_OK) {
        Tcl_Obj **parts; ProbeSize count;
        if (Tcl_ListObjGetElements(i, Tcl_GetObjResult(i), &count, &parts) != TCL_OK) return 1;
        for (ProbeSize j = 0; j < count; ++j) {
            ProbeSize n; const unsigned char *p = (const unsigned char *)Tcl_GetStringFromObj(parts[j], &n);
            printf("C_RAW_ELEMENT %lld ", (long long)n);
            for (ProbeSize k = 0; k < n; ++k) printf("%02x", p[k]);
            puts("");
        }
        if (count >= 3) printf("C_ELEMENT_IDENTITY_0_2 %d\n", parts[0] == parts[2]);
    } else {
        ProbeSize n; const char *p = Tcl_GetStringFromObj(Tcl_GetObjResult(i), &n);
        fputs("guest: ", stderr); fwrite(p, 1, (size_t)n, stderr); fputc('\n', stderr);
    }
    free(source); Tcl_DeleteInterp(i); Tcl_Finalize();
    return code == TCL_OK ? 0 : 1;
}
