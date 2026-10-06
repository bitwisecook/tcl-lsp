#include <stdio.h>
#include <string.h>
#include "jim.h"
static int hooks_enabled;
static int replacement(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
 (void)argc;(void)argv;Jim_SetResultString(i,"CHANGED",-1);return JIM_OK;
}
static void mutate(Jim_Interp *i,const char *value) {
 if(!hooks_enabled)return;
 Jim_SetVariableStr(i,"keep",Jim_NewStringObj(i,value,-1));
 Jim_Obj *name=Jim_NewStringObj(i,"worker",-1);Jim_IncrRefCount(name);Jim_DeleteCommand(i,name);Jim_DecrRefCount(i,name);Jim_CreateCommand(i,"worker",replacement,NULL,NULL);
}
static void update(Jim_Obj *o) {
 mutate((Jim_Interp *)o->internalRep.ptr,"UPDATED");
 o->bytes=Jim_Alloc(2);memcpy(o->bytes,"3",2);o->length=1;
}
static void free_rep(Jim_Interp *i,Jim_Obj *o) {(void)o;mutate(i,"FREED");}
static const Jim_ObjType updater={"incr-update",NULL,NULL,update,JIM_TYPE_NONE};
static const Jim_ObjType freer={"incr-free",free_rep,NULL,NULL,JIM_TYPE_NONE};
static int make_input(Jim_Interp *i,int argc,Jim_Obj *const *argv) {
 Jim_Obj *o; (void)argc;
 if(strcmp(Jim_String(argv[1]),"free")==0) {o=Jim_NewStringObj(i,"3",-1);o->typePtr=&freer;}
 else {o=Jim_NewObj(i);o->bytes=NULL;o->length=0;o->typePtr=&updater;}
 o->internalRep.ptr=i;Jim_SetResult(i,o);return JIM_OK;
}
int main(void) {
 const char *modes[]={"update","free"};int route,mode;
 for(route=0;route<2;route++)for(mode=0;mode<2;mode++) {
 Jim_Interp *i=Jim_CreateInterp();char script[600];int code;
 Jim_RegisterCoreCommands(i);Jim_CreateCommand(i,"make_input",make_input,NULL,NULL);
 Jim_Eval(i,"alias runtime_incr incr; proc worker {} {return OLD}");hooks_enabled=1;
 snprintf(script,sizeof(script),"proc f {} {set input [make_input %s]; set keep SAFE; set result [%s input]; list $result $keep [worker]}; f",modes[mode],route?"runtime_incr":"incr");
 code=Jim_Eval(i,script);printf("route=%s hook=%s code=%d result=%s\n",route?"alias":"direct",modes[mode],code,Jim_String(Jim_GetResult(i)));
 hooks_enabled=0;Jim_FreeInterp(i);
 }
 return 0;
}
