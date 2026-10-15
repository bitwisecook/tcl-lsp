#include <stdio.h>
#include "jim.h"
static const char *label;
static int deleted, invoked;
static Jim_Interp *fresh(void){Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return Jim_InitStaticExtensions(i)==JIM_OK ? i:NULL;}
static void sample(const char *kind,Jim_Obj *object){printf("%s|%s|%d|%s\n",kind,label,object->bytes != NULL,object->typePtr ? object->typePtr->name:"none");}
static void row(const char *kind,int code,Jim_Obj *object){int length;const char *bytes=Jim_GetString(object,&length);printf("%s|%s|%d|",kind,label,code);for(int n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts("");}
static void destroy(Jim_Interp *interp,void *private_data){(void)interp;(void)private_data;deleted++;printf("DELETE|%s|%d|%d\n",label,deleted,invoked);}
static int selected(Jim_Interp *interp,int argc,Jim_Obj *const *argv){invoked++;printf("INVOKE|%s|%d|%d\n",label,deleted,invoked);if(argc!=2)return JIM_ERR;Jim_SetResult(interp,argv[1]);return JIM_OK;}
static void execute(const char *kind,const char *script,const char *target,int hot,int deletion){
 Jim_Interp *interp=fresh();if(!interp)return;deleted=invoked=0;label=kind;
 if(deletion)Jim_CreateCommand(interp,"selected",selected,NULL,destroy);
 Jim_Obj *namespace=Jim_NewIntObj(interp,42);Jim_IncrRefCount(namespace);
 Jim_Obj *elements[]={Jim_NewStringObj(interp,"target",-1),Jim_NewStringObj(interp,script,-1),namespace};
 Jim_Obj *lambda=Jim_NewListObj(interp,elements,3);Jim_IncrRefCount(lambda);
 Jim_Obj *argument=Jim_NewStringObj(interp,target,-1);Jim_IncrRefCount(argument);
 Jim_Obj *argv[]={Jim_NewStringObj(interp,"apply",-1),lambda,argument};Jim_IncrRefCount(argv[0]);
 for(int phase=0;phase<(hot ? 2:1);phase++){
  label=phase ? "HOT_CACHED":"COLD_CACHED";if(!hot)label=kind;
  if(phase)Jim_InvalidateStringRep(namespace);
  sample("NAMESPACE_BEFORE",namespace);int code=Jim_EvalObjVector(interp,3,argv);sample("NAMESPACE_AFTER",namespace);
  printf("COUNTS|%s|%d|%d\n",label,deleted,invoked);row("RESULT",code,Jim_GetResult(interp));
 }
 Jim_DecrRefCount(interp,argv[0]);Jim_DecrRefCount(interp,argument);Jim_DecrRefCount(interp,lambda);Jim_DecrRefCount(interp,namespace);Jim_FreeInterp(interp);
}
int main(void){Jim_Interp *version=fresh();if(!version)return 2;label="VERSION";if(Jim_Eval(version,"info patchlevel")!=JIM_OK)return 3;row("VERSION",0,Jim_GetResult(version));Jim_FreeInterp(version);
 execute("CACHED","tailcall $target VALUE","::return",1,0);
 execute("DEFERRED_DELETE","defer {rename ::selected {}}; tailcall $target VALUE","::selected",0,1);
 execute("MISSING_TARGET","tailcall $target VALUE","::missing",0,0);
 return 0;}
