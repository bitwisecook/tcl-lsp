/* Jim native container conversion callbacks, independent of abstract methods. */
#include <stdio.h>
#include <string.h>
#include "jim.h"
static int destructor_enabled;
static void update_scalar(Jim_Obj *value) {
    Jim_Interp *interp=(Jim_Interp *)value->internalRep.ptr;
    Jim_SetVariableStr(interp,"keep",Jim_NewStringObj(interp,"MUTATED",-1));
    value->bytes=Jim_Alloc(1);value->bytes[0]=0;value->length=0;
}
static void free_scalar(Jim_Interp *interp,Jim_Obj *value) {
    (void)value;
    if(destructor_enabled)Jim_SetVariableStr(interp,"keep",Jim_NewStringObj(interp,"FREED",-1));
}
static const Jim_ObjType custom_scalar={"custom-scalar",free_scalar,NULL,update_scalar,JIM_TYPE_NONE};
static int make_input(Jim_Interp *interp,int argc,Jim_Obj *const *argv) {
    Jim_Obj *value=Jim_NewObj(interp), *result;const char *kind=Jim_String(argv[1]);
    (void)argc;value->bytes=NULL;value->length=0;value->typePtr=&custom_scalar;value->internalRep.ptr=interp;
    if(strcmp(kind,"list")==0)result=Jim_NewListObj(interp,&value,1);
    else if(strcmp(kind,"dict")==0) { Jim_Obj *pairs[]={Jim_NewStringObj(interp,"key",-1),value}; result=Jim_NewDictObj(interp,pairs,2); }
    else result=value;
    Jim_SetResult(interp,result);return JIM_OK;
}
int main(void) {
    const char *kinds[]={"scalar"};int kind,route,free_hook;
    for(free_hook=0;free_hook<2;free_hook++)for(route=0;route<2;route++)for(kind=0;kind<1;kind++) {
        Jim_Interp *interp=Jim_CreateInterp();char script[512];int code;
        destructor_enabled=free_hook;Jim_RegisterCoreCommands(interp);
        Jim_CreateCommand(interp,"make_input",make_input,NULL,NULL);
        Jim_Eval(interp,"alias runtime_iteration foreach");
        snprintf(script,sizeof(script),"proc f {} {set input [make_input %s]; set keep SAFE; %s item $input {}; list $keep}; f",kinds[kind],route?"runtime_iteration":"foreach");
        code=Jim_Eval(interp,script);
        printf("free=%d route=%s kind=%s code=%d result=%s\n",free_hook,route?"alias":"direct",kinds[kind],code,Jim_String(Jim_GetResult(interp)));
        fflush(stdout);destructor_enabled=0;Jim_FreeInterp(interp);
    }
    return 0;
}
