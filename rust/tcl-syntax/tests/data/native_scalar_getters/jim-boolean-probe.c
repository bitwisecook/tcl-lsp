/* Jim boolean getter cache and its distinct integer roundtrip. */
#include <stdio.h>
#include "jim.h"
int main(void) {
    const char *cases[]={"1\0X","true\0X","false\0X","true","false","2","1.5"};
    const int lengths[]={3,6,7,4,5,1,3};int i;
    for(i=0;i<7;i++) {
        Jim_Interp *interp=Jim_CreateInterp();Jim_Obj *input=Jim_NewStringObj(interp,cases[i],lengths[i]);
        int boolean=0,code,widecode;jim_wide wide=0;const char *type;
        Jim_IncrRefCount(input);code=Jim_GetBoolean(interp,input,&boolean);
        type=input->typePtr?input->typePtr->name:"string";widecode=Jim_GetWide(interp,input,&wide);
        printf("case=%d boolcode=%d bool=%d booltype=%s widecode=%d wide=%lld\n",i,code,boolean,type,widecode,(long long)wide);
        Jim_DecrRefCount(interp,input);Jim_FreeInterp(interp);
    }
    return 0;
}
