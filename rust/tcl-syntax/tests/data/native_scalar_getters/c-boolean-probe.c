/* Boolean word extent and cached boolean type are independent numeric doors. */
#include <stdio.h>
#include <tcl.h>
int main(int argc,char **argv) {
    const char *cases[]={"1\0X","true\0X","false\0X","true","false","2","1.5"};
    const int lengths[]={3,6,7,4,5,1,3};int i,kind;
    (void)argc;Tcl_FindExecutable(argv[0]);
    for(kind=0;kind<2;kind++)for(i=0;i<7;i++) {
        Tcl_Interp *interp=Tcl_CreateInterp();Tcl_Obj *input=kind?Tcl_NewByteArrayObj((const unsigned char *)cases[i],lengths[i]):Tcl_NewStringObj(cases[i],lengths[i]);
        int boolean=0,code,widecode;Tcl_WideInt wide=0;const char *type;
        Tcl_IncrRefCount(input);code=Tcl_GetBooleanFromObj(interp,input,&boolean);
        type=input->typePtr?input->typePtr->name:"string";Tcl_ResetResult(interp);
        widecode=Tcl_GetWideIntFromObj(interp,input,&wide);
        printf("kind=%s case=%d boolcode=%d bool=%d booltype=%s widecode=%d wide=%lld\n",kind?"bytearray":"rawstring",i,code,boolean,type,widecode,(long long)wide);
        Tcl_DecrRefCount(input);Tcl_DeleteInterp(interp);
    }
    Tcl_Finalize();return 0;
}
