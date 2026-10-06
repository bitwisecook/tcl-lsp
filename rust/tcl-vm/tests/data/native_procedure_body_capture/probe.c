#include <stdio.h>
#include <string.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj;typedef Jim_Interp ProbeInterp;
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define EVAL(i,n,v) Jim_EvalObjVector(i,n,v)
#define TYPE(o) ((o)->typePtr?(o)->typePtr->name:"none")
static ProbeInterp *create(void){ProbeInterp*i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);return i;}
static Obj *stored(ProbeInterp*i,Obj*n){Jim_Cmd*c=Jim_GetCommand(i,n,JIM_NONE);return c?c->u.proc.bodyObjPtr:NULL;}
#define DESTROY(i) Jim_FreeInterp(i)
#else
#include "tcl.h"
#include "tclInt.h"
typedef Tcl_Obj Obj;typedef Tcl_Interp ProbeInterp;
#define NEW(i,s,n) Tcl_NewStringObj(s,n)
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define LIST(i,n,v) Tcl_NewListObj(n,v)
#define EVAL(i,n,v) Tcl_EvalObjv(i,n,v,TCL_EVAL_GLOBAL)
#define TYPE(o) ((o)->typePtr?(o)->typePtr->name:"none")
static ProbeInterp *create(void){return Tcl_CreateInterp();}
static Obj *stored(ProbeInterp*i,Obj*n){Tcl_CmdInfo c;if(!Tcl_GetCommandInfo(i,"p",&c))return NULL;
#if TCL_MAJOR_VERSION == 9 && TCL_MINOR_VERSION >= 1
return ((Proc*)c.objClientData2)->bodyPtr;
#else
return ((Proc*)c.objClientData)->bodyPtr;
#endif
}
#define DESTROY(i) Tcl_DeleteInterp(i)
#endif
int main(void){for(int kind=0;kind<5;kind++){for(int refs=1;refs<=2;refs++){
#ifdef USE_JIM
if(kind==2)continue;
#endif
ProbeInterp*i=create();Obj*b;
if(kind==1){Obj*words[2]={NEW(i,"return",6),NEW(i,"OK",2)};b=LIST(i,2,words);}
#ifndef USE_JIM
else if(kind==2)b=Tcl_NewByteArrayObj((const unsigned char*)"return OK",9);
#endif
else if(kind==3)b=NEW(i,"set value \"",11);
else if(kind==4)b=NEW(i,"return OK\0error BAD",18);
else b=NEW(i,"return OK",9);
for(int k=0;k<refs;k++)INC(b);
const char*pretype=TYPE(b);int prebytes=b->bytes!=NULL;
Obj*v[4]={NEW(i,"proc",4),NEW(i,"p",1),NEW(i,"",0),b};for(int k=0;k<3;k++)INC(v[k]);
int code=EVAL(i,4,v);int originalrefs=b->refCount;const char*posttype=TYPE(b);int postbytes=b->bytes!=NULL;
Obj*s=stored(i,v[1]);if(!s){fprintf(stderr,"no stored body kind%d code%d\n",kind,code);return 2;}
printf("%d\t%d\t%d\t%d\t%d\t%s\t%d\t%s\t%d\t%s\t%d\t%d\n",kind,refs,code,b==s,originalrefs,pretype,prebytes,posttype,postbytes,TYPE(s),s->bytes!=NULL,s->refCount);
for(int k=0;k<3;k++)DEC(i,v[k]);for(int k=0;k<refs;k++)DEC(i,b);DESTROY(i);
}}return 0;}
