#include <tcl.h>
#include <stdio.h>
#include <string.h>
int main(int argc,char **argv){
 const char *sources[]={"variable _tmm_operator_re \"\\\\m([join $_gen_all_operators |])\\\\M\"","list \"[set value]\\\\\"","list \"[set value]\\n\"","list prefix-[set value]","list \"\""};
 Tcl_Interp *interp; int i; (void)argc; Tcl_FindExecutable(argv[0]); interp=Tcl_CreateInterp();
 for(i=0;i<5;i++){Tcl_Parse parse;int j=0,last=0,words=0;int code=Tcl_ParseCommand(interp,sources[i],(int)strlen(sources[i]),0,&parse);if(code!=TCL_OK){fprintf(stderr,"parse %d: %s\n",i,Tcl_GetStringResult(interp));return 1;}while(j<parse.numTokens){Tcl_Token *token=parse.tokenPtr+j;last=(int)(token->start-sources[i])+token->size;words++;j+=token->numComponents+1;}printf("%d\t%d\t%d\t%d\n",i,words,last,(int)strlen(sources[i]));if(last!=(int)strlen(sources[i]))return 2;Tcl_FreeParse(&parse);}
 Tcl_DeleteInterp(interp);Tcl_Finalize();return 0;
}
