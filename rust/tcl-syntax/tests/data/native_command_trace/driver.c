#include <tcl.h>
#include <stdio.h>
#include <stdlib.h>
int main(int argc, char **argv) {
 if (argc != 3) return 2;
 Tcl_FindExecutable(argv[1]);
 Tcl_Interp *i = Tcl_CreateInterp();
 if (Tcl_Init(i) != TCL_OK) {fprintf(stderr,"init:%s\n",Tcl_GetStringResult(i)); return 3;}
 FILE *f = fopen(argv[2],"rb"); if (!f) return 4;
 fseek(f,0,SEEK_END); long size=ftell(f); rewind(f);
 char *bytes=malloc((size_t)size+1); if (!bytes) return 5;
 if (fread(bytes,1,(size_t)size,f)!=(size_t)size) return 6;
 fclose(f); bytes[size]=0;
 int code=Tcl_EvalEx(i,bytes,(int)size,0);
 if (code!=TCL_OK) fprintf(stderr,"eval:%d:%s\n",code,Tcl_GetStringResult(i));
 free(bytes); Tcl_DeleteInterp(i); Tcl_Finalize(); return code;
}
