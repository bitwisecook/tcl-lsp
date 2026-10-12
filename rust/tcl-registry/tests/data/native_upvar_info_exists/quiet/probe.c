#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static int caseId, markCount;
static void hex(const char *p,int n){for(int i=0;i<n;i++)printf("%02x",(unsigned char)p[i]);}
static int Mark(ClientData data,Tcl_Interp *ip,int argc,Tcl_Obj *const argv[]){
Tcl_Obj *v=argv[1];int refs=v->refCount,resident=v->bytes!=NULL; const char *type=v->typePtr?v->typePtr->name:"none";
printf("H|%d|%d|%s|%d|%d|",caseId,markCount++,type,resident,refs);Size n;const char *s=Tcl_GetStringFromObj(v,&n);hex(s,n);puts("");return TCL_OK;}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();Tcl_CreateObjCommand(ip,"mark",Mark,NULL,NULL);
const char *cases[]={"upvar #0 ::g alias; trace add variable alias read traceRead; info exists alias","upvar #0 ::g alias; trace add variable alias read traceFail; info exists alias","upvar 0 a(k) alias; trace add variable alias read traceFail; info exists alias"};
for(caseId=0;caseId<sizeof(cases)/sizeof(*cases);caseId++){
Tcl_EvalEx(ip,"catch {unset ::alias};set ::x GLOBAL;set ::a(k) GLOBALARRAY;catch {unset ::g};set ::log {}; proc tap {v} {lappend ::log $v;return $v};proc traceRead {n e op} {set ::g CREATED}; proc traceFail {n e op} {error TRACE}",-1,TCL_EVAL_GLOBAL);
char script[4096];snprintf(script,sizeof script,"proc p {name idx} {set x LOCAL;set a(k) LOCALARRAY;%s}",cases[caseId]);if(Tcl_EvalEx(ip,script,-1,TCL_EVAL_GLOBAL)!=TCL_OK){printf("SETUP|%d\n",caseId);continue;}
Tcl_Obj *v[]={Tcl_NewStringObj("p",-1),Tcl_NewStringObj("x",-1),Tcl_NewStringObj("k",-1)};for(int i=0;i<3;i++)Tcl_IncrRefCount(v[i]);markCount=0;
int code=Tcl_EvalObjv(ip,3,v,TCL_EVAL_GLOBAL);Tcl_Obj *r=Tcl_GetObjResult(ip);int refs=r->refCount,resident=r->bytes!=NULL;const char *type=r->typePtr?r->typePtr->name:"none";
printf("R|%d|%d|%s|%d|%d|",caseId,code,type,resident,refs);Size n;const char *s=Tcl_GetStringFromObj(r,&n);hex(s,n);printf("|");
Proc *p=TclFindProc((Interp*)ip,"p");ByteCode *bc;
#ifdef ByteCodeGetInternalRep
ByteCodeGetInternalRep(p->bodyPtr,&tclByteCodeType,bc);
#else
bc=(ByteCode*)p->bodyPtr->internalRep.otherValuePtr;
#endif
if(p->bodyPtr->typePtr&&!strcmp(p->bodyPtr->typePtr->name,"bytecode")&&bc)for(int off=0;off<bc->numCodeBytes;){const InstructionDesc *d=&tclInstructionTable[bc->codeStart[off]];if(!strcmp(d->name,"upvar")||!strncmp(d->name,"exist",5)){printf("%s",d->name);if(d->numBytes==5)printf(":%d",TclGetInt4AtPtr(bc->codeStart+off+1));printf(",");}if(!d->numBytes)break;off+=d->numBytes;}puts("");
for(int i=0;i<3;i++)Tcl_DecrRefCount(v[i]);}
Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
