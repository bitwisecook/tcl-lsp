#include <stdio.h>
#include <string.h>
#include "jim.h"
static Jim_Obj *held;
static Jim_Cmd *nodes[128]; static unsigned node_count;
static unsigned identity(Jim_Cmd*p){if(!p)return 0;for(unsigned n=0;n<node_count;n++)if(nodes[n]==p)return n+1;nodes[node_count++]=p;return node_count;}
static void hex(const char*p,int len){for(int n=0;n<len;n++)printf("%02x",(unsigned char)p[n]);}
static Jim_Obj* obj(Jim_Interp*i,const char*s){Jim_Obj*o=Jim_NewStringObj(i,s,-1);Jim_IncrRefCount(o);return o;}
static void observation(Jim_Interp*i,const char*label,Jim_Obj*o,int reached){
 Jim_Cmd*selected=reached?Jim_GetCommand(i,o,JIM_NONE):NULL;
 printf("%s\tepoch=%lu\tframe=%lu\tlevel=%d\ttype=%s\trefs=%d\tselected=%u",label,i->procEpoch,i->framePtr->id,i->framePtr->level,o->typePtr?o->typePtr->name:"none",o->refCount,identity(selected));
 if(o->typePtr&&!strcmp(o->typePtr->name,"command")){
  printf("\tcacheepoch=%lu",o->internalRep.cmdValue.procEpoch);
  if(o->internalRep.cmdValue.procEpoch==i->procEpoch){Jim_Cmd*c=o->internalRep.cmdValue.cmdPtr;
   printf("\tcached=%u\tinuse=%d",identity(c),c->inUse);
   /* Retired negative nodes reuse prevCmd as a free-cache chain. It is
    * not an owning previous-command edge and must not be reported as one. */
   if(c->inUse){
    printf("\tprev=%u\tupcall=%d",identity(c->prevCmd),(c->flags&JIM_CMD_ISPROC)?c->u.proc.upcall:-1);
    if(c->prevCmd)printf("\tprevinuse=%d",c->prevCmd->inUse);
   }
  }
 }
 int len;const char *ns=Jim_GetString(i->framePtr->nsObj,&len);printf("\tnamespace=");hex(ns,len);
 Jim_Stack*s=i->framePtr->localCommands;printf("\tcleanup=%d",s?s->len:0);printf("\n");
 if(s)for(int n=0;n<s->len;n++){
  Jim_Obj*name=(Jim_Obj*)s->vector[n];const char*bytes=Jim_GetString(name,&len);
  printf("%s/cleanup-%d\tname=",label,n);hex(bytes,len);printf("\ttype=%s\trefs=%d\tcacheepoch=%lu\n",name->typePtr?name->typePtr->name:"none",name->refCount,(name->typePtr&&!strcmp(name->typePtr->name,"command"))?name->internalRep.cmdValue.procEpoch:0);
 }
}
static int observe(Jim_Interp*i,int ac,Jim_Obj*const*av){(void)ac;observation(i,Jim_String(av[1]),held,1);return JIM_OK;}
static int rename_probe(Jim_Interp*i,int ac,Jim_Obj*const*av){(void)ac;(void)av;Jim_Obj*v[3]={obj(i,"rename"),held,obj(i,"q")};unsigned long epoch=i->procEpoch;int code=Jim_EvalObjVector(i,3,v);int len;const char*bytes=Jim_GetString(Jim_GetResult(i),&len);printf("rename-local\tcode=%d\tbefore=%lu\tafter=%lu\tresult=",code,epoch,i->procEpoch);hex(bytes,len);puts("");for(int n=0;n<3;n++)if(n!=1)Jim_DecrRefCount(i,v[n]);return JIM_OK;}
static int remove_probe(Jim_Interp*i,int ac,Jim_Obj*const*av){(void)ac;(void)av;int code=Jim_DeleteCommand(i,held);printf("delete-local-active\tcode=%d\n",code);observation(i,"delete-local-active-original",held,1);Jim_Obj*fresh=obj(i,"p");observation(i,"delete-local-active-fresh",fresh,1);Jim_DecrRefCount(i,fresh);return JIM_OK;}
static int native_base(Jim_Interp*i,int ac,Jim_Obj*const*av){(void)ac;(void)av;observation(i,"native-base-active",held,1);Jim_SetResultString(i,"NATIVE",-1);return JIM_OK;}
static void run(const char*label,const char*setup,const char*body,int native){
 node_count=0;Jim_Interp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_InitStaticExtensions(i);held=obj(i,"p");Jim_CreateCommand(i,"observe",observe,NULL,NULL);Jim_CreateCommand(i,"renameprobe",rename_probe,NULL,NULL);Jim_CreateCommand(i,"removeprobe",remove_probe,NULL,NULL);if(native)Jim_CreateCommand(i,"p",native_base,NULL,NULL);
 if(setup&&*setup)Jim_Eval(i,setup);observation(i,label,held,1);int code=Jim_Eval(i,body);int len;const char*bytes=Jim_GetString(Jim_GetResult(i),&len);printf("%s/result\tcode=%d\tepoch=%lu\tresult=",label,code,i->procEpoch);hex(bytes,len);puts("");observation(i,"after-frame-cleanup",held,1);Jim_DecrRefCount(i,held);Jim_FreeInterp(i);
}
int main(void){
 run("ordinary-initial","proc p {} {return BASE}","proc driver {} {local proc p {} {return LOCAL}; observe ordinary-local; set got [p]; observe ordinary-aftercall; return $got}; driver",0);
 run("nested-initial","proc p {} {observe base-active; return BASE}","proc driver {} {local proc p {} {observe one-active; return [upcall p]}; observe nested-one; local proc p {} {observe two-active; set got [upcall p]; observe two-afterupcall; return $got}; observe nested-two; set got [p]; observe nested-aftercall; return $got}; driver",0);
 run("rename-initial","proc p {} {return BASE}","proc driver {} {local proc p {} {return LOCAL}; observe rename-before; renameprobe; observe rename-after; return DONE}; driver",0);
 run("fresh-initial","","proc driver {} {local proc p {} {return FRESH}; observe fresh-local; return [p]}; driver",0);
 run("native-initial","","proc driver {} {local proc p {} {return [upcall p]}; observe native-wrapper; return [p]}; driver",1);
 run("delete-initial","proc p {} {observe base-during-delete; removeprobe; observe base-after-delete; return BASE}","proc driver {} {local proc p {} {return [upcall p]}; observe delete-wrapper; return [p]}; driver",0);
 return 0;
}
