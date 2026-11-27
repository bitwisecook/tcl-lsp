#include <jim.h>
#include <stdio.h>
#include <string.h>
static const char*const words[]={"provide","present","--",NULL};
static const char*const other[]={"present","provide",NULL};
static void hex(const char*s,int n){int k;for(k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);}
static Jim_Obj*str(Jim_Interp*i,const char*s,int n){Jim_Obj*o=Jim_NewStringObj(i,s,n);Jim_IncrRefCount(o);return o;}
static void row(Jim_Interp*i,const char*stage,Jim_Obj*o,const char*const*table,int flags){char*before=o->bytes;int index=-7;int code=Jim_GetEnum(i,o,table,&index,NULL,flags);const char*t=o->typePtr?o->typePtr->name:"none";int en=!strcmp(t,"get-enum");int n;const char*r=Jim_GetString(Jim_GetResult(i),&n);printf("%s\t%d\t%d\t%s\t%d\t%d\t%d\t%d\t",stage,code,index,t,en?o->internalRep.ptrIntValue.int1:-1,en?o->internalRep.ptrIntValue.ptr==table:0,o->bytes!=NULL,o->bytes==before);hex(r,n);putchar('\n');}
int main(int argc,char**argv){Jim_Interp*i=Jim_CreateInterp();Jim_Obj*o=str(i,"pro",3);row(i,"abbrev-errmsg",o,words,JIM_ERRMSG|JIM_ENUM_ABBREV);row(i,"exact-miss-different-flags",o,words,JIM_ERRMSG);row(i,"abbrev-noerrmsg-different-flags",o,words,JIM_ENUM_ABBREV);row(i,"changed-table",o,other,JIM_ENUM_ABBREV);Jim_InvalidateStringRep(o);row(i,"cached-absent",o,other,JIM_ENUM_ABBREV);Jim_DecrRefCount(i,o);
 const char*names[]={"","-","p","bogus","--",NULL};int k;for(k=0;names[k];k++){o=str(i,names[k],-1);char stage[80];sprintf(stage,"fresh-%s",names[k][0]?names[k]:"empty");row(i,stage,o,words,JIM_ENUM_ABBREV|JIM_ERRMSG);Jim_DecrRefCount(i,o);}
 const char raw[]={ 'p','r','o','v','i','d','e',0,'x' };o=str(i,raw,sizeof(raw));row(i,"counted-nul-exact",o,words,JIM_ERRMSG);Jim_DecrRefCount(i,o);
 o=str(i,"provide",-1);Jim_CompareStringImmediate(i,o,words[0]);row(i,"compared-exact",o,words,JIM_ERRMSG);Jim_DecrRefCount(i,o);
 o=str(i,"bogus",-1);Jim_CompareStringImmediate(i,o,"bogus");row(i,"compared-failed",o,words,JIM_ERRMSG);Jim_DecrRefCount(i,o);
 o=Jim_NewIntObj(i,42);Jim_IncrRefCount(o);row(i,"numeric-failed",o,words,JIM_ERRMSG);Jim_DecrRefCount(i,o);
 Jim_FreeInterp(i);return 0;}
