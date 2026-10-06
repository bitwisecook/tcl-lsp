#include "tcl.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static void snap(const char *label,Tcl_Obj *o) {
 printf("%s\t%s\t%d\t%d\n",label,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(int)o->refCount);
}
static void bytes(const char *label,Tcl_Obj *o) {
 Count n; const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(o,&n);printf("%s\t",label);Count k;for(k=0;k<n;k++)printf("%02x",s[k]);puts("");
}
int main(int argc,char **argv) {
 int which=argc>1?atoi(argv[1]):0; Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();
 const unsigned char *text=(const unsigned char*)"a";int length=1;
 if(which==2){text=(const unsigned char*)"(";}
 if(which==3){text=(const unsigned char*)"\xff";}
 if(which==4){text=(const unsigned char*)"\xc0\x80";length=2;}
 if(which==5){text=(const unsigned char*)"\xf0\x9f\x98\x80";length=4;}
 Tcl_Obj *p=Tcl_NewStringObj((const char*)text,length);Tcl_IncrRefCount(p);
 Tcl_RegExp r=Tcl_GetRegExpFromObj(i,p,TCL_REG_ADVANCED);snap("pattern",p);
 printf("compile\t%d\n",r!=NULL);
 if(r && (which==0||which==1||which==9||which==10)) {
  Tcl_Obj *chosen=p;
  if(which==9){chosen=Tcl_DuplicateObj(p);Tcl_IncrRefCount(chosen);snap("duplicate",chosen);}
  if(which==0||which==9||which==10)Tcl_InvalidateStringRep(chosen);
  Tcl_RegExp again=Tcl_GetRegExpFromObj(i,chosen,TCL_REG_ADVANCED|((which==1||which==10)?TCL_REG_NOCASE:0));
  printf("cache\t%d\n",again==r);snap("after",chosen);r=again;
  if(chosen!=p)Tcl_DecrRefCount(chosen);
 }
 if(which==6||which==7||which==8) {
  Tcl_Obj *head=Tcl_NewStringObj("regsub",-1),*all=Tcl_NewStringObj("-all",-1),*pattern=Tcl_NewStringObj(which==6?"(a)":which==7?"a":"z",-1);
  Tcl_Obj *subject=Tcl_NewStringObj("ab",-1),*spec=Tcl_NewStringObj(which==6?"\\1X":"X",-1);
  Tcl_Obj *words[5]={head,all,pattern,subject,spec};int n;for(n=0;n<5;n++)Tcl_IncrRefCount(words[n]);
  int code=Tcl_EvalObjv(i,5,words,0);printf("command\t%d\t%d\n",code,Tcl_GetObjResult(i)==subject);
  snap("commandpattern",pattern);snap("commandsubject",subject);snap("commandspec",spec);snap("commandresult",Tcl_GetObjResult(i));bytes("commandbytes",Tcl_GetObjResult(i));
  for(n=0;n<5;n++)Tcl_DecrRefCount(words[n]);
 } else if(r && which!=1 && which!=10) {
  Tcl_Obj *s=Tcl_NewStringObj((const char*)text,length);Tcl_IncrRefCount(s);
  int matched=Tcl_RegExpExecObj(i,r,s,0,1,0);printf("match\t%d\n",matched);snap("subject",s);
  if(matched==1){Tcl_RegExpInfo info;Tcl_RegExpGetInfo(r,&info);Tcl_Obj *range=Tcl_GetRange(s,info.matches[0].start,info.matches[0].end-1);Tcl_IncrRefCount(range);snap("range",range);bytes("rangebytes",range);Tcl_DecrRefCount(range);}
  Tcl_DecrRefCount(s);
 }
 Tcl_DecrRefCount(p);Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
