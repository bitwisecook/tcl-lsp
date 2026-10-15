#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Obj *observed_namespace;
static const char *label, *name_bytes;
static void update_name(Jim_Obj *object) {
 printf("NAMESPACE_AT_NAME_GETTER|%s|%d|%s\n",label,observed_namespace->bytes != NULL,observed_namespace->typePtr ? observed_namespace->typePtr->name : "none");
 int length=strlen(name_bytes); object->bytes=Jim_Alloc(length+1);memcpy(object->bytes,name_bytes,length+1);object->length=length;
}
static const Jim_ObjType name_type={"which-name-probe",NULL,NULL,update_name,JIM_TYPE_NONE};
static Jim_Interp *fresh(void) { Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return Jim_InitStaticExtensions(i)==JIM_OK ? i:NULL; }
static void sample(const char *kind,Jim_Obj *object) { printf("%s|%s|%d|%s\n",kind,label,object->bytes != NULL,object->typePtr ? object->typePtr->name : "none"); }
static void row(const char *kind,int code,Jim_Obj *object) { int length;const char *bytes=Jim_GetString(object,&length);printf("%s|%s|%d|",kind,label,code);for(int n=0;n<length;n++)printf("%02x",(unsigned char)bytes[n]);puts(""); }
int main(void) {
 Jim_Interp *version=fresh();if(!version)return 2;label="VERSION";if(Jim_Eval(version,"info patchlevel")!=JIM_OK)return 3;row("VERSION",0,Jim_GetResult(version));Jim_FreeInterp(version);
 const struct {const char *label,*option,*name;int direct;} cases[]={
  {"PUBLIC_VARIABLE_ABSOLUTE","-variable","::absent",0},{"PUBLIC_VARIABLE_RELATIVE","-variable","absent",0},
  {"PUBLIC_COMMAND_ABSOLUTE","-command","::return",0},{"PUBLIC_COMMAND_RELATIVE","-command","return",0},
  {"DIRECT_VARIABLE_ABSOLUTE","-variable","::absent",1},{"DIRECT_VARIABLE_RELATIVE","-variable","absent",1},
  {"DIRECT_COMMAND_ABSOLUTE","-command","::return",1},{"DIRECT_COMMAND_RELATIVE","-command","return",1}
 };
 for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
  Jim_Interp *interp=fresh();if(!interp)return 4;
  Jim_Obj *namespace=Jim_NewIntObj(interp,42);Jim_IncrRefCount(namespace);observed_namespace=namespace;name_bytes=cases[n].name;
  Jim_Obj *name=Jim_NewStringObj(interp,"",0);Jim_IncrRefCount(name);Jim_InvalidateStringRep(name);name->typePtr=&name_type;
  Jim_Obj *elements[]={Jim_NewStringObj(interp,cases[n].direct ? "namespace which":"namespace",-1),Jim_NewStringObj(interp,"which",-1),Jim_NewStringObj(interp,cases[n].option,-1),name};
  if(cases[n].direct){elements[1]=elements[2];elements[2]=elements[3];}
  Jim_Obj *body=Jim_NewListObj(interp,elements,cases[n].direct ? 3:4);Jim_IncrRefCount(body);
  for(int hot=0;hot<2;hot++) {
   char phase[128];snprintf(phase,sizeof(phase),"%s_%s",cases[n].label,hot ? "HOT":"COLD");label=phase;
   if(hot){Jim_InvalidateStringRep(namespace);Jim_InvalidateStringRep(name);}
   sample("NAMESPACE_BEFORE",namespace);sample("NAME_BEFORE",name);
   int code=Jim_EvalNamespace(interp,body,namespace);
   sample("NAMESPACE_AFTER",namespace);sample("NAME_AFTER",name);row("RESULT",code,Jim_GetResult(interp));
  }
  Jim_DecrRefCount(interp,body);Jim_DecrRefCount(interp,name);Jim_DecrRefCount(interp,namespace);Jim_FreeInterp(interp);
 }
 return 0;
}
