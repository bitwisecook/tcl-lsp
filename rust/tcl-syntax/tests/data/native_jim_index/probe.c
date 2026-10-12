#include "jim.h"
#include <stdio.h>
#include <stdlib.h>
int main(int argc,char **argv){
 const char *sources[]={"2*3","end-2*3","(2+3)*2","1?2:3","$x","[set x 1]","end-1.5","endx","2147483648","end+1","-1","end","3+4/2","2**3","int(2.5)","end-2\0tail","2*3\0tail"};
 const int lengths[]={-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,10,8};
 int n=argc>1?atoi(argv[1]):0;Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);
 Jim_Obj *o=Jim_NewStringObj(i,sources[n],lengths[n]);Jim_IncrRefCount(o);int index=42;
 int code=Jim_GetIndex(i,o,&index);
 printf("code\t%d\nindex\t%d\nobject\t%s\t%d\t%d\n",code,index,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,o->refCount);
 int len;const unsigned char *bytes=(const unsigned char*)Jim_GetString(i->result,&len);printf("result\t");for(int k=0;k<len;k++)printf("%02x",bytes[k]);puts("");fflush(stdout);
 Jim_DecrRefCount(i,o);Jim_FreeInterp(i);return 0;
}
