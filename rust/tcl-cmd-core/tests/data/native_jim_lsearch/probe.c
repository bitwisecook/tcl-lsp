#include "jim.h"
#include <stdio.h>
#include <stdlib.h>
extern int Jim_regexpInit(Jim_Interp*);
static int callback(Jim_Interp *i,int argc,Jim_Obj *const*argv){
 printf("callback\t%d",argc);for(int j=0;j<argc;j++)printf("\t%s:%d",argv[j]->typePtr?argv[j]->typePtr->name:"none",argv[j]->refCount);puts("");fflush(stdout);
 Jim_SetResultInt(i,2);return JIM_OK;
}
static int returns(Jim_Interp *i,int argc,Jim_Obj *const*argv){Jim_SetResultString(i,"ORIGINAL_RETURN",-1);return JIM_RETURN;}
int main(int argc,char **argv){int n=argc>1?atoi(argv[1]):0;Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);Jim_regexpInit(i);Jim_CreateCommand(i,"two",callback,NULL,NULL);Jim_CreateCommand(i,"returns",returns,NULL,NULL);
 static const char *cases[][8]={
  {"lsearch","a *","*"},
  {"lsearch","-g","ab zz","a*"},
  {"lsearch","-glob","ab zz","a*"},
  {"lsearch","-regexp","ab zz","a"},
  {"lsearch","-command","two","ab zz","a"},
  {"lsearch","-bool","-command","two","ab zz","a"},
  {"lsearch","-all","-bool","-command","two","ab zz","a"},
  {"lsearch","-inline","ab zz","a"},
  {"lsearch","-inline","-regexp","ab zz","a"},
  {"lsearch","-all","-inline","-stride","2","a A b B","a"},
  {"lsearch","-all","-inline","-stride","2","-index","0","a A b B"},
  {"lsearch","-inline","-stride","2","-index","1","a A b B","A"},
  {"lsearch","-inline","-index","1","{a A} {b B}","A"},
  {"lsearch","-bool","-not","a b","z"},
  {"lsearch","-bool","-not","a b","a"},
  {"lsearch","-not","a b","a"},
  {"lsearch","-nocase","\xff X","\xc3\xbf"},
  {"lsearch","-regexp","","["},
  {"lsearch","-regexp","a","["},
  {"lsearch","-command","returns","a","a"},
  {"lsearch","-all","-bool","-stride","2","a A b B","a"},
  {"lsearch","--","a b","a"},
  {"lsearch","-regexp","-stride","2","a b c","a"},
  {"lsearch","-index","end+1","{a A} {b B}","A"}
 };
 Jim_Obj *words[12];int count=0;for(int j=0;j<8 && cases[n][j];j++){words[count]=Jim_NewStringObj(i,cases[n][j],-1);Jim_IncrRefCount(words[count++]);}
 if(n==8)Jim_CreateCommand(i,"regexp",callback,NULL,NULL);
 if(n==10){words[count]=Jim_NewStringObj(i,"a",-1);Jim_IncrRefCount(words[count++]);}
 int code=Jim_EvalObjVector(i,count,words);
 printf("code\t%d\nresult\t%s\t%d\t%d\n",code,i->result->typePtr?i->result->typePtr->name:"none",i->result->bytes!=NULL,i->result->refCount);
 for(int j=1;j<count;j++)printf("argument\t%d\t%s\t%d\t%d\n",j,words[j]->typePtr?words[j]->typePtr->name:"none",words[j]->bytes!=NULL,words[j]->refCount);
 int len;const unsigned char *bytes=(const unsigned char*)Jim_GetString(i->result,&len);printf("bytes\t");for(int k=0;k<len;k++)printf("%02x",bytes[k]);puts("");fflush(stdout);
 for(int j=0;j<count;j++)Jim_DecrRefCount(i,words[j]);Jim_FreeInterp(i);return 0;}
