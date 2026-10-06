#include "jim-regexp.c"
#include <stdio.h>
static void snap(const char *name, Jim_Obj *o) {
 printf("%s\t%s\t%d\t%d\n",name,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount);fflush(stdout);
}
int main(int argc,char **argv) {
 int mode=argc>1?atoi(argv[1]):0;
 Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_regexpInit(i);
 const char *text=mode==4?"\xff":mode==5?"\xc0\x80":mode==6?"\xf0\x9f\x98\x80":mode==7?"(":"a";
 int length=mode==5?2:mode==6?4:1;
 Jim_Obj *p=Jim_NewStringObj(i,text,length);Jim_IncrRefCount(p);
 regex_t *compiled=SetRegexpFromAny(i,p,0);snap("pattern",p);printf("compile\t%d\n",compiled!=NULL);fflush(stdout);
 if(compiled && mode==1) {
  Jim_InvalidateStringRep(p);regex_t *again=SetRegexpFromAny(i,p,0);printf("cache\t%d\n",again==compiled);snap("after",p);
 } else if(compiled && mode==2) {
  Jim_InvalidateStringRep(p);regex_t *again=SetRegexpFromAny(i,p,REG_ICASE);printf("cache\t%d\n",again==compiled);snap("after",p);
 } else if(compiled && mode==3) {
  Jim_Obj *copy=Jim_DuplicateObj(i,p);Jim_IncrRefCount(copy);snap("duplicate",copy);printf("cache\t%d\n",SetRegexpFromAny(i,copy,0)==compiled);fflush(stdout);Jim_DecrRefCount(i,copy);
 } else if(compiled) {
  if(mode==8)Jim_InvalidateStringRep(p);
  Jim_Obj *head=Jim_NewStringObj(i,"regexp",-1);Jim_IncrRefCount(head);
  Jim_Obj *s=Jim_NewStringObj(i,text,length);Jim_IncrRefCount(s);
  Jim_Obj *name=Jim_NewStringObj(i,"m",-1);Jim_IncrRefCount(name);
  Jim_Obj *words[4]={head,p,s,name};int code=Jim_RegexpCmd(i,4,words);printf("code\t%d\n",code);snap("patternafter",p);snap("subject",s);snap("result",i->result);
  Jim_Obj *value=Jim_GetVariable(i,name,JIM_NONE);
  if(value){snap("range",value);int n;const unsigned char *bytes=(const unsigned char*)Jim_GetString(value,&n);printf("bytes\t");int k;for(k=0;k<n;k++)printf("%02x",bytes[k]);puts("");fflush(stdout);}
  Jim_DecrRefCount(i,name);Jim_DecrRefCount(i,s);Jim_DecrRefCount(i,head);
 }
 Jim_DecrRefCount(i,p);Jim_FreeInterp(i);return 0;
}
