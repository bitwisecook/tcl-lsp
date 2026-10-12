#include <stdio.h>
#include <string.h>
#ifdef PROBE_JIM
#include "jim.h"
typedef Jim_Obj Obj; typedef Jim_Interp InterpHandle;
#define STR(i,s) Jim_NewStringObj(i,s,-1)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define PIN(o) Jim_IncrRefCount(o)
#define DROP(i,o) Jim_DecrRefCount(i,o)
#define RESULT(i) ((i)->result)
#define GETVAR(i,s) Jim_GetVariableStr(i,s,JIM_NONE)
#define GETSTRING(o,n) Jim_GetString(o,n)
#else
#include "tclInt.h"
typedef Tcl_Obj Obj; typedef Tcl_Interp InterpHandle;
#define STR(i,s) Tcl_NewStringObj(s,-1)
#define LIST(i,n,v) Tcl_NewListObj(n,v)
#define PIN(o) Tcl_IncrRefCount(o)
#define DROP(i,o) Tcl_DecrRefCount(o)
#define RESULT(i) (((Interp *)(i))->objResultPtr)
#define GETVAR(i,s) Tcl_GetVar2Ex(i,s,NULL,TCL_GLOBAL_ONLY)
#define GETSTRING(o,n) Tcl_GetStringFromObj(o,n)
#endif
static InterpHandle *active; static Obj *varlist,*valuelist,*name0,*member0,*body; static int current_case,seq;
static const char *type(Obj*o){return !o?"absent":o->typePtr?o->typePtr->name:"none";}
static int backing_refs(Obj*o){
#ifdef PROBE_JIM
(void)o;return -1;
#else
if(!o||!o->typePtr||strcmp(o->typePtr->name,"list"))return -1;
#if TCL_MAJOR_VERSION>=9
return (int)ListObjStorePtr(o)->refCount;
#elif TCL_MINOR_VERSION>=5
return ((List*)o->internalRep.twoPtrValue.ptr1)->refCount;
#else
return -1;
#endif
#endif
}
static void objstate(Obj*o){printf("%s,%d,%d,%d",type(o),o?o->refCount:-1,o&&o->bytes!=NULL,backing_refs(o));}
static void snapshot(const char*window,Obj*cell){
printf("S\t%d\t%d\t%s\t",current_case,seq++,window);objstate(varlist);putchar('\t');objstate(valuelist);putchar('\t');objstate(name0);putchar('\t');objstate(member0);putchar('\t');objstate(body);putchar('\t');objstate(cell);printf("\t%d\t",cell&&cell==member0);objstate(RESULT(active));putchar('\n');
}
#ifdef PROBE_JIM
static int observe(InterpHandle*i,int argc,Obj*const*argv){(void)argc;(void)argv;snapshot("body",GETVAR(i,"v"));Jim_SetResultString(i,"BODY",4);return JIM_OK;}
#else
static int observe(ClientData c,InterpHandle*i,int argc,Obj*const*argv){(void)c;(void)argc;(void)argv;snapshot("body",GETVAR(i,"v"));Tcl_SetObjResult(i,STR(i,"BODY"));return TCL_OK;}
static char*watch(ClientData c,InterpHandle*i,const char*n1,const char*n2,int flags){(void)c;(void)n1;(void)n2;(void)flags;snapshot("write",GETVAR(i,"v"));return NULL;}
#endif
static void hex(const char*s,int n){for(int k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);}
int main(void){for(current_case=0;current_case<10;current_case++){
#ifdef PROBE_JIM
active=Jim_CreateInterp();Jim_RegisterCoreCommands(active);Jim_CreateCommand(active,"observe",observe,NULL,NULL);
#else
active=Tcl_CreateInterp();Tcl_CreateObjCommand(active,"observe",observe,NULL,NULL);Tcl_TraceVar(active,"v",TCL_TRACE_WRITES|TCL_GLOBAL_ONLY,watch,NULL);
#endif
seq=0;Obj*names[2]={STR(active,"v"),STR(active,"w")};Obj*members[3]={STR(active,"A"),STR(active,"B"),STR(active,"C")};
if(current_case==8){Obj*inner=STR(active,"N");members[0]=LIST(active,1,&inner);}
varlist=LIST(active,current_case==2?2:1,names);valuelist=LIST(active,current_case==2?3:current_case==5?2:current_case==3||current_case==4?0:1,members);name0=names[0];member0=current_case==3||current_case==4?NULL:members[0];body=STR(active,current_case==3||current_case==4?"{":"observe");
Obj*args[6];int count=4;args[0]=STR(active,current_case==4||current_case==5?"lmap":"foreach");args[1]=varlist;args[2]=valuelist;args[3]=body;
if(current_case==6||current_case==7){count=6;args[1]=varlist;args[2]=STR(active,"{");args[3]=LIST(active,0,NULL);args[4]=LIST(active,0,NULL);args[5]=body;if(current_case==7){args[1]=args[3];args[3]=varlist;}}
if(current_case==9){args[1]=LIST(active,0,NULL);varlist=args[1];name0=NULL;}
varlist=args[1];valuelist=args[2];if(current_case==6||current_case==7){member0=NULL;if(current_case==7)name0=NULL;}
for(int k=0;k<count;k++)PIN(args[k]);
if(current_case==1){
#ifdef PROBE_JIM
int length;
#else
int length;
#endif
/* These inputs deliberately enter with an existing resident spelling. */
#ifdef PROBE_JIM
Jim_GetString(varlist,&length);Jim_GetString(valuelist,&length);
#else
Tcl_GetString(varlist);Tcl_GetString(valuelist);
#endif
}
snapshot("before",NULL);
#ifdef PROBE_JIM
int code=Jim_EvalObjVector(active,count,args);
#else
int code=Tcl_EvalObjv(active,count,args,TCL_EVAL_GLOBAL);
#endif
snapshot("after",NULL);
#ifdef PROBE_JIM
int resultlen;
#else
#if TCL_MAJOR_VERSION>=9
Tcl_Size resultlen;
#else
int resultlen;
#endif
#endif
const char*result=GETSTRING(RESULT(active),&resultlen);printf("R\t%d\t%d\t",current_case,code);hex(result,resultlen);putchar('\n');
for(int k=0;k<count;k++)DROP(active,args[k]);
#ifdef PROBE_JIM
Jim_FreeInterp(active);
#else
Tcl_DeleteInterp(active);
#endif
}return 0;}
