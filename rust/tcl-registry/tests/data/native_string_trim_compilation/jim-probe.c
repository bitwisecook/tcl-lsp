#include "jim.h"
#include <stdio.h>
static void header(Jim_Obj *o) {printf("%s,%d,%d",o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount);}
int main(void) {Jim_Interp *ip=Jim_CreateInterp();Jim_RegisterCoreCommands(ip);
 const char *members[]={"trim","trimleft","trimright"};const char *input[]={" x ","x","","\0x\0","\xffx\xff","--x--"};int sizes[]={3,1,0,3,3,5};
 for(int mode=0;mode<2;mode++)for(int member=0;member<3;member++)for(int n=0;n<6;n++) {
  char source[256];snprintf(source,sizeof(source),"proc p {s c} {string %s $s%s}",members[member],n==5?" $c":"");if(Jim_Eval(ip,source)!=JIM_OK)return 2;
  Jim_Obj *subject=Jim_NewStringObj(ip,input[n],sizes[n]),*chars=Jim_NewStringObj(ip,"-",1);
  Jim_Obj *v[4]={Jim_NewStringObj(ip,mode?"p":"string",-1),mode?subject:Jim_NewStringObj(ip,members[member],-1),mode?chars:subject,chars};int count=mode?3:(n==5?4:3);for(int j=0;j<count;j++)Jim_IncrRefCount(v[j]);
  int rc=Jim_EvalObjVector(ip,count,v);Jim_Obj *result=ip->result;printf("window|%d|%s|%d|%d|",mode,members[member],n,rc);header(result);printf("|%d|",result==subject);header(subject);printf("|");header(chars);printf("|");int length;const unsigned char *bytes=(const unsigned char*)Jim_GetString(result,&length);for(int j=0;j<length;j++)printf("%02x",bytes[j]);puts("");
  for(int j=0;j<count;j++)Jim_DecrRefCount(ip,v[j]);if(!mode&&n!=5)Jim_FreeNewObj(ip,chars);
 }Jim_FreeInterp(ip);return 0;}
