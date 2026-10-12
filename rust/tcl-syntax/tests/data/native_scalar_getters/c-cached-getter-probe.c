/* Direct existing-cache native scalar getter discriminator. */
#include <stdint.h>
#include <inttypes.h>
#include <math.h>
#include <stdio.h>
#include "tcl.h"
int main(int argc,char **argv) {
 int kind,getter;
 Tcl_FindExecutable(argv[0]);
 for(kind=0;kind<7;kind++)for(getter=0;getter<3;getter++) {
  Tcl_Interp *interp=Tcl_CreateInterp(); Tcl_Obj *input; int code,b=0;
  Tcl_WideInt wide=0;double d=0;union{double d;uint64_t bits;}v;
  if(kind==0)input=Tcl_NewWideIntObj(INT64_MAX);
  else if(kind==1)input=Tcl_NewWideIntObj(2);
  else if(kind<5)input=Tcl_NewDoubleObj(kind==2?1.0:kind==3?1.5:NAN);
  else if(kind==5){input=Tcl_NewStringObj("true",-1);Tcl_GetBooleanFromObj(interp,input,&b);}
  else input=Tcl_NewBooleanObj(1);
  Tcl_IncrRefCount(input);
  printf("kind=%d getter=%d before=%s",kind,getter,input->typePtr?input->typePtr->name:"string");
  if(getter==0){code=Tcl_GetWideIntFromObj(interp,input,&wide);v.bits=(uint64_t)wide;}
  else if(getter==1){code=Tcl_GetDoubleFromObj(interp,input,&d);v.d=d;}
  else {code=Tcl_GetBooleanFromObj(interp,input,&b);v.bits=(uint64_t)b;}
  printf(" code=%d after=%s bits=%016" PRIx64 "\n",code,input->typePtr?input->typePtr->name:"string",v.bits);
  Tcl_DecrRefCount(input);Tcl_DeleteInterp(interp);
 }
 Tcl_Finalize();return 0;
}
