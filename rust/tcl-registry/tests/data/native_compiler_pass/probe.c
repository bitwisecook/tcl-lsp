#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
static int passes;
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 6
extern int __real_TclCompileArraySetCmd(Tcl_Interp *, Tcl_Parse *, Command *, CompileEnv *);
int __wrap_TclCompileArraySetCmd(Tcl_Interp *ip,Tcl_Parse *p,Command *c,CompileEnv *e) {
    ++passes; return __real_TclCompileArraySetCmd(ip,p,c,e);
}
#endif
static int trace(ClientData data,Tcl_Interp *ip,int level,const char *command,Command *token,int objc,Tcl_Obj *const objv[]) {
    (void)data;(void)ip;(void)level;(void)command;(void)token;(void)objc;(void)objv;return TCL_OK;
}
static void run(int mode) {
    Tcl_Interp *root=Tcl_CreateInterp(),*ip=root;
    if(mode==1) { Tcl_Init(root); ip=Tcl_CreateSlave(root,"child",0); }
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    if(mode==2) {Tcl_LimitSetCommands(ip,10000);Tcl_LimitTypeSet(ip,TCL_LIMIT_COMMANDS);}
    if(mode==3) {Tcl_Time when;Tcl_GetTime(&when);when.sec+=10000;Tcl_LimitSetTime(ip,&when);Tcl_LimitTypeSet(ip,TCL_LIMIT_TIME);}
#endif
    if(mode==4) Tcl_CreateObjTrace(ip,0,0,(Tcl_CmdObjTraceProc *)trace,NULL,NULL);
    const char *ns= mode==6 || mode==7 ? "::tcl" : "::N";
    const char *tail=mode==5 || mode==6 || mode==7 ? "; missing" : "";
#if TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5
    if(mode==7){Tcl_LimitSetCommands(ip,10000);Tcl_LimitTypeSet(ip,TCL_LIMIT_COMMANDS);}
#endif
    if (!ip) {printf("%d\tchild-unavailable\n",mode);Tcl_DeleteInterp(root);return;}
    char setup[256];snprintf(setup,sizeof(setup),"namespace eval %s {}; proc %s::p {data} {array set a $data%s}",ns,ns,tail);
    if(Tcl_EvalEx(ip,setup,-1,0)!=TCL_OK){printf("%d\tsetup-error\n",mode);Tcl_DeleteInterp(root);return;}
    char name[64];snprintf(name,sizeof(name),"%s::p",ns);
    Tcl_Obj *words[]={Tcl_NewStringObj(name,-1),Tcl_NewStringObj("k V",-1)};
    for(int i=0;i<2;i++)Tcl_IncrRefCount(words[i]);
    passes=0;int code=Tcl_EvalObjv(ip,2,words,0);Proc *proc=TclFindProc((Interp*)ip,name);
    printf("%d\t%d\t%d\t%d\n",mode,code,passes,proc?proc->numCompiledLocals:-1);
    for(int i=0;i<2;i++)Tcl_DecrRefCount(words[i]);
    Tcl_DeleteInterp(root);
}
int main(int argc,char **argv){(void)argc;setbuf(stdout,NULL);Tcl_FindExecutable(argv[0]);for(int mode=0;mode<8;mode++)run(mode);Tcl_Finalize();return 0;}
