#include "tcl.h"
#include <stdio.h>
#include <stdlib.h>
int main(int argc,char **argv) {
    FILE *f;long n;char *source;Tcl_Interp *i;int code;
    if(argc!=2) return 2;
    f=fopen(argv[1],"rb");if(!f)return 3;
    fseek(f,0,SEEK_END);n=ftell(f);rewind(f);source=malloc(n+1);
    if(fread(source,1,n,f)!=(size_t)n)return 4;fclose(f);source[n]=0;
    Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();code=Tcl_EvalEx(i,source,n,0);
    if(code!=TCL_OK)fprintf(stderr,"%d|%s\n",code,Tcl_GetStringResult(i));
    Tcl_DeleteInterp(i);Tcl_Finalize();free(source);return code;
}
