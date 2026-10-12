#include "tclInt.h"
#include <stdio.h>
#include <stdlib.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
int main(int argc, char **argv) {
    if (argc != 2) return 64;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *i = Tcl_CreateInterp();
    if (Tcl_Init(i) != TCL_OK) {fprintf(stderr,"init: %s\n",Tcl_GetStringResult(i)); return 1;}
    Command *worker = (Command *)Tcl_FindCommand(i,"::tcl::clock::GetDateFields",NULL,TCL_GLOBAL_ONLY);
    Command *parser = (Command *)Tcl_FindCommand(i,"::tcl::clock::ParseFormatArgs",NULL,TCL_GLOBAL_ONLY);
    Command *getter = (Command *)Tcl_FindCommand(i,"::tcl::clock::getenv",NULL,TCL_GLOBAL_ONLY);
    printf("C_WORKER present %d null_hook %d shared_parser_pool %d shared_getter_pool %d\n",worker!=NULL,worker&&!worker->compileProc,worker&&parser&&worker->objClientData==parser->objClientData,worker&&getter&&worker->objClientData==getter->objClientData);
    FILE *f=fopen(argv[1],"rb"); if(!f)return 66;
    fseek(f,0,SEEK_END);long length=ftell(f);rewind(f);
    char *source=malloc((size_t)length+1);
    if(!source||fread(source,1,(size_t)length,f)!=(size_t)length)return 74;
    fclose(f);source[length]=0;
    int code=Tcl_EvalEx(i,source,(Size)length,TCL_EVAL_GLOBAL);
    printf("C_COMPLETION %d\n",code);
    if(code!=TCL_OK)fprintf(stderr,"guest: %s\n",Tcl_GetStringResult(i));
    free(source);Tcl_DeleteInterp(i);Tcl_Finalize();return code==TCL_OK?0:1;
}
