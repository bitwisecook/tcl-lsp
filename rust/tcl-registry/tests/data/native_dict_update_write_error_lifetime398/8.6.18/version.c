#include <stdio.h>
#include "tcl.h"
int main(void){Tcl_FindExecutable("lifetime398");Tcl_Interp *i=Tcl_CreateInterp();int c=Tcl_Eval(i,"info patchlevel");printf("%d\t%s\n",c,Tcl_GetStringResult(i));Tcl_DeleteInterp(i);return c;}
