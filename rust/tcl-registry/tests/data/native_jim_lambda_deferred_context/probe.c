
#include <stdio.h>
#include "jim.h"
static Jim_Interp *fresh(void){Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return Jim_InitStaticExtensions(i)==JIM_OK?i:NULL;}
static void row(const char *kind,const char *label,int code,Jim_Obj *object){int length;const char *bytes=Jim_GetString(object,&length);printf("%s|%s|%d|",kind,label,code);for(int n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts("");}
static void sample(const char *kind,const char *label,Jim_Obj *object){printf("%s|%s|%d|%s\n",kind,label,object->bytes!=NULL,object->typePtr?object->typePtr->name:"none");}
int main(void){
 Jim_Interp *version=fresh();if(!version)return 2;int code=Jim_Eval(version,"info patchlevel");row("VERSION","VERSION",code,Jim_GetResult(version));Jim_FreeInterp(version);
 const char *labels[]={"EMPTY_BODY","RETURN_BODY","NAMESPACE_BODY","EMPTY_REQUIRED","NONEMPTY_REQUIRED","COMMENT_BODY","INVALID_BODY"};const char *bodies[]={"","return BODY","namespace current","","return BODY","# comment","{"};
 for(int n=0;n<7;n++){
  Jim_Interp *interp=fresh();if(!interp)return 3;
  Jim_Obj *namespace=Jim_NewIntObj(interp,42);Jim_IncrRefCount(namespace);
  Jim_Obj *parameters = (n < 3 || n >= 5) ? Jim_NewListObj(interp,NULL,0) : Jim_NewStringObj(interp,"required",-1);
  Jim_Obj *elements[]={parameters,Jim_NewStringObj(interp,bodies[n],-1),namespace};
  Jim_Obj *lambda=Jim_NewListObj(interp,elements,3);Jim_IncrRefCount(lambda);
  Jim_Obj *args[]={Jim_NewStringObj(interp,"apply",5),lambda};Jim_IncrRefCount(args[0]);
  sample("BEFORE",labels[n],namespace);
  code=Jim_EvalObjVector(interp,2,args);
  sample("AFTER",labels[n],namespace);row("RESULT",labels[n],code,Jim_GetResult(interp));row("GETTER",labels[n],0,namespace);
  Jim_DecrRefCount(interp,args[0]);Jim_DecrRefCount(interp,lambda);Jim_DecrRefCount(interp,namespace);Jim_FreeInterp(interp);
 }
 return 0;
}
