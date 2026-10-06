/* Direct pinned Jim Wide boundary: ambient errno and cached bypass. */
#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include "jim.h"
int main(void) {
    const char *cases[]={"9223372036854775807","-9223372036854775808"};
    int kind, state, at;
    for(kind=0;kind<2;kind++)for(state=0;state<2;state++)for(at=0;at<2;at++) {
        Jim_Interp *interp=Jim_CreateInterp();
        Jim_Obj *input=kind?Jim_NewIntObj(interp,at?INT64_MIN:INT64_MAX):Jim_NewStringObj(interp,cases[at],-1);
        jim_wide value=0;int code;
        Jim_IncrRefCount(input);
        /* These two explicit native inputs discriminate the real dependency. */
        errno=state?ERANGE:0;
        code=Jim_GetWide(interp,input,&value);
        printf("kind=%d state=%d case=%d code=%d type=%s bits=%016" PRIx64 "\n",kind,state,at,code,input->typePtr?input->typePtr->name:"string",(uint64_t)value);
        Jim_DecrRefCount(interp,input);Jim_FreeInterp(interp);
    }
    return 0;
}
