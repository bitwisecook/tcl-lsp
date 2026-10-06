#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj O;
typedef Jim_Interp I;
#define CREATE() Jim_CreateInterp()
#define STR(i,s,n) Jim_NewStringObj(i,s,n)
#define NUM(i,n) Jim_NewIntObj(i,n)
#define LIST(i,v,n) Jim_NewListObj(i,v,n)
#define DICT(i,v,n) Jim_NewDictObj(i,v,n)
#define PIN(i,o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
#define DELETE(i) Jim_FreeInterp(i)
#define TEXT(o,n) Jim_GetString(o,n)
#define TYPE(o) ((o)->typePtr?(o)->typePtr->name:"none")
#else
#include <tcl.h>
typedef Tcl_Obj O;
typedef Tcl_Interp I;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
#define CREATE() Tcl_CreateInterp()
#define STR(i,s,n) Tcl_NewStringObj(s,n)
#define NUM(i,n) Tcl_NewIntObj(n)
#define LIST(i,v,n) Tcl_NewListObj(n,v)
#define PIN(i,o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
#define DELETE(i) Tcl_DeleteInterp(i)
#define TEXT(o,n) Tcl_GetStringFromObj(o,n)
#define TYPE(o) ((o)->typePtr?(o)->typePtr->name:"none")
#if TCL_MINOR_VERSION != 4 || TCL_MAJOR_VERSION != 8
static O *dict(I*i,O**v,int n){O*d=Tcl_NewDictObj();int k;for(k=0;k<n;k+=2)Tcl_DictObjPut(i,d,v[k],v[k+1]);return d;}
#define DICT(i,v,n) dict(i,v,n)
#endif
#endif
static void hex(const char*s,int n){int k;for(k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);if(!n)printf("-");}
int main(int argc,char**argv){I*i;int c;(void)argc;
#ifndef USE_JIM
Tcl_FindExecutable(argv[0]);
#else
(void)argv;
#endif
i=CREATE();for(c=0;c<7;c++){O*r,*child,*v[4];const char*s;int childref;
#ifdef USE_JIM
int len;
#else
Len len;
#endif
#if !defined(USE_JIM) && TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
if(c==4||c==5){printf("%d\tunavailable\n",c);continue;}
#endif
v[0]=STR(i,"#first",6);v[1]=STR(i,"#later",6);child=v[0];
if(c==1){v[0]=STR(i,"a\"b",3);v[1]=STR(i,"]",1);child=v[0];}
if(c==2){const char raw[]={ 'A',0,(char)255 };v[0]=STR(i,raw,3);v[1]=STR(i,"\\\n",2);child=v[0];}
if(c==3){O*inner[2];inner[0]=NUM(i,17);inner[1]=STR(i,"a\"b",3);v[0]=LIST(i,inner,2);v[1]=STR(i,"#later",6);child=v[0];}
#if defined(USE_JIM) || TCL_MAJOR_VERSION != 8 || TCL_MINOR_VERSION != 4
if(c==4){v[0]=STR(i,"#key",4);v[1]=STR(i,"a\"b",3);r=DICT(i,v,2);child=v[1];}
else if(c==5){O*inner[2];inner[0]=STR(i,"#key",4);inner[1]=STR(i,"a\"b",3);v[0]=DICT(i,inner,2);v[1]=STR(i,"#later",6);r=LIST(i,v,2);child=v[0];}
else
#endif
if(c==6){r=STR(i,"ORIGINAL",8);
#ifdef USE_JIM
(void)Jim_ListLength(i,r);child=Jim_ListGetIndex(i,r,0);
#else
(void)Tcl_ListObjLength(i,r,&len);(void)Tcl_ListObjIndex(i,r,0,&child);
#endif
}else r=LIST(i,v,2);
PIN(i,r);childref=child->refCount;printf("%d\t%s\t%d\t%s\t%d\t",c,TYPE(r),r->bytes!=NULL,TYPE(child),child->bytes!=NULL);s=TEXT(r,&len);hex(s,(int)len);printf("\t%s\t%d\t%s\t%d\t%d\t%d\n",TYPE(r),r->bytes!=NULL,TYPE(child),child->bytes!=NULL,childref,child->refCount);DROP(i,r);
}DELETE(i);
#ifndef USE_JIM
Tcl_Finalize();
#endif
return 0;}
