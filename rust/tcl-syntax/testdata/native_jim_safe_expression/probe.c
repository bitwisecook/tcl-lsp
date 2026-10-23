#include <stdio.h>
#include <string.h>
#include "jim.h"
static const char *type(Jim_Obj *o){return o->typePtr?o->typePtr->name:"none";}
static void hex(const char *s,int n){int k;for(k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);if(!n)printf("-");}
int main(void){Jim_Interp*i=Jim_CreateInterp();const char *cases[]={"17","1+2","$17+2","$v+2","$17","${1+2}","$d(k)","[set side 9]","0 ? [set side 9] : 17","1 || [set side 9]","\"$17\"","{17}","1.0","1.5","NaN","1\0X","1\xc0\x80X","{"};int mode,k;Jim_RegisterCoreCommands(i);Jim_Eval(i,"set side 0;set v 10;set d [dict create k 10]");for(mode=0;mode<3;mode++)for(k=0;k<18;k++){Jim_Obj *o=Jim_NewStringObj(i,cases[k],k==15?3:k==16?4:(int)strlen(cases[k]));Jim_Obj *file=NULL;int rc,n;const char *msg; jim_wide value=123;
Jim_IncrRefCount(o);if(mode==1){file=Jim_NewStringObj(i,"file",4);Jim_IncrRefCount(file);Jim_SetSourceInfo(i,o,file,7);}if(mode==2){int len;(void)Jim_Utf8Length(i,o);(void)Jim_GetString(o,&len);}printf("%d\t%d\t%s\t%d\t",mode,k,type(o),o->bytes!=NULL);rc=Jim_GetWideExpr(i,o,&value);printf("%d\t%lld\t%s\t%d\t",rc,(long long)value,type(o),o->bytes!=NULL);msg=Jim_GetString(Jim_GetResult(i),&n);hex(msg,n);printf("\n");Jim_DecrRefCount(i,o);if(file)Jim_DecrRefCount(i,file);}
Jim_Eval(i,"set side");printf("SIDE\t%s\n",Jim_String(Jim_GetResult(i)));Jim_FreeInterp(i);return 0;}
