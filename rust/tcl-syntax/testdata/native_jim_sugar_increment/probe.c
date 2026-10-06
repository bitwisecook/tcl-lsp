#include <stdio.h>
#include "/tmp/2286-oracles/jimtcl/jim.c"
static const char *kind(Jim_Obj *o) {return o->typePtr?o->typePtr->name:"NULL";}
int main(void) {
 for(int c=0;c<7;c++) {
  Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
  Jim_Obj *key=Jim_NewStringObj(i,"k",1),*member=Jim_NewStringObj(i,(c==2||c==3)?"BAD":"4",-1);
  Jim_Obj *pair[]={key,member};Jim_Obj *root=Jim_NewDictObj(i,pair,2);
  Jim_SetVariableStr(i,"d",root);
  int shared=c==1||c==3||c==5||c==6;
  if(shared)Jim_IncrRefCount(root);
  if(c==4)Jim_IncrRefCount(member);
  Jim_Obj *argv[]={Jim_NewStringObj(i,"incr",4),Jim_NewStringObj(i,c==5?"d(missing)":"d(k)",-1),c==6?Jim_NewStringObj(i,"BAD",3):Jim_NewIntObj(i,1)};
  for(int a=0;a<3;a++)Jim_IncrRefCount(argv[a]);
  printf("BEFORE\t%d\t%d\t%s\t%d\t%d\t%s\t%d\n",c,root->refCount,kind(root),root->bytes!=NULL,member->refCount,kind(member),member->bytes!=NULL);
  int code=Jim_EvalObjVector(i,3,argv);
  Jim_Obj *newroot=Jim_GetVariableStr(i,"d",0),*current=NULL;
  Jim_DictKey(i,newroot,c==5?Jim_NewStringObj(i,"missing",7):key,&current,0);
  printf("AFTER\t%d\t%d\t%d\t%s\t%d\t%d\t%s\t%d\t%d\n",c,code,root==newroot,kind(newroot),newroot->refCount,current?current==member:-1,current?kind(current):"ABSENT",current?current->refCount:-1,current?current->bytes!=NULL:-1);
  if(shared)printf("OLD\t%d\t%d\t%s\t%d\n",c,member->refCount,kind(member),member->bytes!=NULL);
  for(int a=0;a<3;a++)Jim_DecrRefCount(i,argv[a]);
  if(c==4)Jim_DecrRefCount(i,member);
  if(shared)Jim_DecrRefCount(i,root);
  Jim_FreeInterp(i);
 }
}
