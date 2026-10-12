#include <stdio.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
static Jim_Obj *parent;
static ScriptObj *prepared;
static int control, calls, case_id;
static int inspect(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
    calls++;
    printf("CALL\t%d\t%d\t%d\t%d\t%d\n",case_id,calls,argc,prepared->inUse,prepared->linenr);
    for(int j=0;j<argc;j++) {
        int token=-1;
        for(int k=0;k<prepared->len;k++)if(prepared->token[k].objPtr==argv[j])token=k;
        printf("ARGV\t%d\t%d\t%d\t%d\t%s\t%d\t%d\n",case_id,calls,j,token,argv[j]->typePtr?argv[j]->typePtr->name:"NULL",argv[j]->bytes!=NULL,argv[j]->refCount);
    }
    if(control) {
        (void)Jim_ListLength(i,parent);
        printf("SHIMMER\t%d\t%s\t%d\n",case_id,parent->typePtr?parent->typePtr->name:"NULL",prepared->inUse);
    }
    Jim_SetResult(i,argc>1?argv[1]:i->emptyObj);
    return control==2?JIM_ERR:control==3?JIM_RETURN:JIM_OK;
}
int main(void) {
    const char *sources[]={"inspect X", "inspect X; inspect Y", "inspect X; set x {bad", "inspect X; inspect [error FAIL]", "", "inspect X", "inspect X", "inspect X", "inspect {*} {X Y}", "inspect pre${v}post"};
    for(int k=0;k<10;k++) {
        Jim_Interp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); Jim_CreateCommand(i,"inspect",inspect,NULL,NULL);
        Jim_SetVariableStr(i,"v",Jim_NewStringObj(i,"V",1));
        Jim_Obj *f=Jim_NewStringObj(i,"FILE",4); Jim_IncrRefCount(f);
        parent=Jim_NewStringObj(i,sources[k],-1); Jim_IncrRefCount(parent); Jim_SetSourceInfo(i,parent,f,7);
        JimSetScriptFromAny(i,parent); prepared=Jim_GetIntRepPtr(parent);
        printf("BEFORE\t%d\t%d\t%d\t%d\t%d\n",k,prepared->len,prepared->missing,prepared->linenr,prepared->inUse);
        case_id=k; calls=0; control=k==5?1:k==6?2:k==7?3:0;
        int code=Jim_EvalObj(i,parent);
        printf("AFTER\t%d\t%d\t%d\t%s\t%d\t%d\t%d\t%d\n",k,code,calls,parent->typePtr?parent->typePtr->name:"NULL",parent->typePtr==&scriptObjType&&Jim_GetIntRepPtr(parent)==prepared,prepared->inUse,prepared->linenr,Jim_GetResult(i)==i->emptyObj);
        Jim_DecrRefCount(i,parent); Jim_DecrRefCount(i,f); Jim_FreeInterp(i);
    }
    return 0;
}
