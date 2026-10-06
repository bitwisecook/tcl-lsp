#include "jim.h"
#include <stdio.h>
static const char *patterns[]={"raw*","raw?","raw[\xff]","raw\xff","raw\xc3\xbf","N::raw*","N::raw?","N::raw[\xff]","::raw?","raw\0::N::*"};
static const int lengths[]={4,4,6,4,5,9,9,11,6,10};
int main(void){Jim_Interp *ip=Jim_CreateInterp();Jim_RegisterCoreCommands(ip);if(Jim_InitStaticExtensions(ip)!=JIM_OK)return 2;
 if(Jim_Eval(ip,"namespace eval N {}")!=JIM_OK)return 3;
 const char *names[]={"raw\xff","raw\xc3\xbf","N::raw\xff","N::raw\xc3\xbf"};
 for(int n=0;n<4;n++){Jim_Obj *v[]={Jim_NewStringObj(ip,"proc",-1),Jim_NewStringObj(ip,names[n],-1),Jim_NewStringObj(ip,"",0),Jim_NewStringObj(ip,"",0)};for(int j=0;j<4;j++)Jim_IncrRefCount(v[j]);int rc=Jim_EvalObjVector(ip,4,v);for(int j=0;j<4;j++)Jim_DecrRefCount(ip,v[j]);if(rc!=JIM_OK)return 4;}
 for(int n=0;n<10;n++){Jim_Obj *v[]={Jim_NewStringObj(ip,"info",-1),Jim_NewStringObj(ip,"commands",-1),Jim_NewStringObj(ip,patterns[n],lengths[n])};for(int j=0;j<3;j++)Jim_IncrRefCount(v[j]);int rc=Jim_EvalObjVector(ip,3,v);printf("0|%d|%d|",n,rc);int count=Jim_ListLength(ip,ip->result);printf("%d",count);for(int k=0;k<count;k++){Jim_Obj *member=Jim_ListGetIndex(ip,ip->result,k);int size;const unsigned char *b=(void*)Jim_GetString(member,&size);printf("|");for(int j=0;j<size;j++)printf("%02x",b[j]);}puts("");for(int j=0;j<3;j++)Jim_DecrRefCount(ip,v[j]);}
 Jim_FreeInterp(ip);return 0;}
