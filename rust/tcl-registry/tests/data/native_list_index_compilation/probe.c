#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Size;
#else
typedef int Size;
#endif
static void hex(const char *p,int n){for(int i=0;i<n;i++)printf("%02x",(unsigned char)p[i]);}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();const char *bodies[]={"lindex $x 0","lindex $x $i","lindex $x","lindex $x {0 1}","lindex $x 0 1","lindex $x bad","lindex $x -1","lindex $x end","lindex $x end-1","lindex $x end+1","lindex $x 1+1","lindex $x \\060","lindex {*}$i","lindex {*}{a b} 0","lindex $x 2147483647","lindex $x {0 bad}","lindex {} 0 bad","lindex \\{ 0","lindex $x 0 [return EARLY]"};
for(int n=0;n<sizeof(bodies)/sizeof(*bodies);n++){
char definition[2048];snprintf(definition,sizeof definition,"proc p {x i} {%s}",bodies[n]);int setup=Tcl_EvalEx(ip,definition,-1,TCL_EVAL_GLOBAL);if(setup){printf("setup|%d|%d\n",n,setup);continue;}
Tcl_Obj *v[]={Tcl_NewStringObj("p",-1),Tcl_NewStringObj("{{A B} C} D",-1),Tcl_NewStringObj("0",-1)};for(int i=0;i<3;i++)Tcl_IncrRefCount(v[i]);int rc=Tcl_EvalObjv(ip,3,v,TCL_EVAL_GLOBAL);Tcl_Obj *r=Tcl_GetObjResult(ip);int refs=r->refCount,resident=r->bytes!=NULL,same=r==v[1];const char *type=r->typePtr?r->typePtr->name:"none";printf("%d|%d|%s|%d|%d|%d|",n,rc,type,resident,refs,same);Size size;const char *text=Tcl_GetStringFromObj(r,&size);hex(text,size);printf("|");
Proc *p=TclFindProc((Interp*)ip,"p");ByteCode *code;
#ifdef ByteCodeGetInternalRep
ByteCodeGetInternalRep(p->bodyPtr,&tclByteCodeType,code);
#else
code=(ByteCode*)p->bodyPtr->internalRep.otherValuePtr;
#endif
if(p->bodyPtr->typePtr&&!strcmp(p->bodyPtr->typePtr->name,"bytecode")&&code)for(int off=0;off<code->numCodeBytes;){unsigned int op=code->codeStart[off];const InstructionDesc *d=&tclInstructionTable[op];if(!strcmp(d->name,"listIndex")||!strcmp(d->name,"listindex")||!strcmp(d->name,"listIndexImm")||!strcmp(d->name,"listIndexMulti")||!strcmp(d->name,"lindexMulti")){printf("%s",d->name);if(d->numBytes==5)printf(":%d",TclGetInt4AtPtr(code->codeStart+off+1));}if(!d->numBytes)break;off+=d->numBytes;}puts("");for(int i=0;i<3;i++)Tcl_DecrRefCount(v[i]);}
Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;}
