#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
static int passes,writes;
extern int __real_TclCompileArraySetCmd(Tcl_Interp *, Tcl_Parse *, Command *, CompileEnv *);
int __wrap_TclCompileArraySetCmd(Tcl_Interp *ip,Tcl_Parse *p,Command *c,CompileEnv *e){++passes;return __real_TclCompileArraySetCmd(ip,p,c,e);}
static char *limit_trace(ClientData data,Tcl_Interp *ip,const char *a,const char *b,int flags){(void)a;(void)b;(void)flags;++writes;if(data){Tcl_LimitSetCommands(ip,1000000);Tcl_LimitTypeSet(ip,TCL_LIMIT_COMMANDS);}return NULL;}
static void run(int enabled){Tcl_Interp *ip=Tcl_CreateInterp();Tcl_EvalEx(ip,"namespace eval ::N {}",-1,0);Tcl_TraceVar2(ip,"errorInfo",NULL,TCL_TRACE_WRITES|TCL_GLOBAL_ONLY,limit_trace,(ClientData)(long)enabled);Tcl_Obj *setup[]={Tcl_NewStringObj("proc",-1),Tcl_NewStringObj("::N::p",-1),Tcl_NewStringObj("data",-1),Tcl_NewStringObj("array set a $data; set {",-1)};for(int n=0;n<4;n++)Tcl_IncrRefCount(setup[n]);int s=Tcl_EvalObjv(ip,4,setup,0);Tcl_Obj *words[]={Tcl_NewStringObj("::N::p",-1),Tcl_NewStringObj("k V",-1)};for(int n=0;n<2;n++)Tcl_IncrRefCount(words[n]);passes=writes=0;int code=Tcl_EvalObjv(ip,2,words,0);Proc *proc=TclFindProc((Interp*)ip,"::N::p");printf("%d\t%d\t%d\t%d\t%d\t%d\n",enabled,s,code,passes,writes,proc->numCompiledLocals);for(int n=0;n<4;n++)Tcl_DecrRefCount(setup[n]);for(int n=0;n<2;n++)Tcl_DecrRefCount(words[n]);Tcl_DeleteInterp(ip);}
int main(int argc,char **argv){(void)argc;setbuf(stdout,NULL);Tcl_FindExecutable(argv[0]);run(0);run(1);Tcl_Finalize();return 0;}
