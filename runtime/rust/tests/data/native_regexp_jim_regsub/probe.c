#include "jim-regexp.c"
#include <stdio.h>
static void snap(const char *name,Jim_Obj *o){printf("object\t%s\t%s\t%d\t%d\n",name,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount);fflush(stdout);}
int main(int argc,char **argv){int mode=argc>1?atoi(argv[1]):0;Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_regexpInit(i);
 const char *p=mode==1?"\xff":mode==2?"\xc0\x80":mode==3?"\xf0\x9f\x98\x80":mode==4?"z":mode==5?"(a)":mode==6?"":mode==7?"a":mode==8?"a":"a";
 const char *s=mode==1?"\xff":mode==2?"\xc0\x80":mode==3?"\xf0\x9f\x98\x80":"aba";
 const char *r=mode==5?"\\1&":mode==8?"string toupper":"X";
 Jim_Obj *h=Jim_NewStringObj(i,"regsub",-1),*pattern=Jim_NewStringObj(i,p,-1),*subject=Jim_NewStringObj(i,s,-1),*replacement=Jim_NewStringObj(i,r,-1),*all=Jim_NewStringObj(i,mode==8?"-command":"-all",-1);Jim_Obj *words[8];int n=0;words[n++]=h;words[n++]=all;
 Jim_Obj *start=NULL,*index=NULL;if(mode==7){start=Jim_NewStringObj(i,"-start",-1);index=Jim_NewStringObj(i,"end-1",-1);words[n++]=start;words[n++]=index;}
 words[n++]=pattern;words[n++]=subject;words[n++]=replacement;for(int j=0;j<n;j++)Jim_IncrRefCount(words[j]);
 if(mode==9)SetRegexpFromAny(i,pattern,0);
 int code=Jim_RegsubCmd(i,n,words);printf("code\t%d\n",code);snap("pattern",pattern);snap("subject",subject);snap("replacement",replacement);snap("result",i->result);if(index)snap("index",index);
 int len;const unsigned char *bytes=(const unsigned char*)Jim_GetString(i->result,&len);printf("bytes\t");for(int k=0;k<len;k++)printf("%02x",bytes[k]);puts("");fflush(stdout);
 for(int j=0;j<n;j++)Jim_DecrRefCount(i,words[j]);Jim_FreeInterp(i);return 0;}
