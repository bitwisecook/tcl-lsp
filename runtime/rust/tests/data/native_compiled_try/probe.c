#include "tclInt.h"
#include "tclCompile.h"
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION < 9
typedef int ProbeSize;
#else
typedef Tcl_Size ProbeSize;
#endif
static const char *bodies[]={"foreach i {A B} {set last $i}; list $last [info locals]",
"foreach {i j} {A B C} {set last [list $i $j]}; list $last [info locals]",
"foreach i {A B} j {C} {set last [list $i $j]}; list $last [info locals]",
"foreach {a(k)} {A} {set last $a(k)}; list $last [info locals]",
"namespace eval N {}; foreach ::N::i {A} {set last $::N::i}; list $last [info locals]",
"foreach {} {A} {set never 1}",
"foreach i {A} {a(k)} {B} {set last $i}; list $last [info locals]",
"foreach \\x69 {A} {set last $i}; list $last [info locals]",
"set names i; foreach $names {A} {set last $i}; list $last [info locals]",
"set body {set last $i}; foreach i {A} $body; list $last [info locals]",
"foreach i [set values A] {set body $i}; info locals",
"foreach i {A B} {continue}; return AFTER",
"foreach i [return -level 0 -code continue VALUE] {set last $i}",
"foreach i {A} {set broken \"}",
"foreach {i {} {A} {set last $i}",
"foreach {a\\u0000b} {A} {set last DONE}; info locals",
"foreach i {A} {set first $i}; foreach j {B} {set second $j}; info locals",
"lmap i {A B} {list $i $i}",
"lmap i {A B} {if {$i eq \"B\"} {break}; set i}",
"try {set b BODY}; info locals",
"try {set b BODY} finally {set f FINALLY}; list $b $f [info locals]",
"try {set b BODY} finally {}; info locals",
"try {set b BODY} on ok {m o} {list $m [dict get $o -code]}; info locals",
"try {error BODY} on error {m o} {list $m [dict get $o -code]}; info locals",
"try {error BODY {} {TCL TEST}} trap {TCL} {m o} {list $m [dict get $o -code]}; info locals",
"try {error BODY} trap {} {m o} {set chosen $m}; info locals",
"try {error BODY} on error {m o extra} {set chosen $m}; info locals",
"try {error BODY} on error {a(k) o} {set chosen $a(k)}; info locals",
"try {error BODY} on error {m o} - on ok {a(k) opts} {set chosen $m}; info locals",
"try {error BODY} on error {m o} -; info locals",
"set which error; try {error BODY} on $which {m o} {set chosen $m}; info locals",
"set body {error BODY}; try $body on error {m o} {set chosen $m}; info locals",
"set handler {set chosen $m}; try {error BODY} on error {m o} $handler; info locals",
"set handler {set done DONE}; try {set b BODY} finally $handler; info locals",
"try {error BODY {} {TCL TEST}} trap {TCL} {m o} {set chosen $m} finally {set done DONE}; info locals",
"try {continue} on continue {m o} {list $m [dict get $o -code]}; info locals",
"try {error BODY} on error {first opts} - trap {TCL} {second options} {set chosen $second}; info locals",
"try {set body B} on ok {result options} {set handler H} finally {set final F}; info locals",
"try {return ORIGINAL} finally {return OVERRIDE}",
"try {error ORIGINAL} finally {error OVERRIDE}"};
static void hex(const char *p,int n){for(int j=0;j<n;j++)printf("%02x",(unsigned char)p[j]);}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);puts("case\tcode\tresult\tlocals\tinstructions\tranges\tliterals");for(int i=0;i<40;i++){
 Tcl_Interp *in=Tcl_CreateInterp();Tcl_Obj *args=Tcl_NewStringObj("",0),*body=Tcl_NewStringObj(bodies[i],-1);Tcl_IncrRefCount(args);Tcl_IncrRefCount(body);
 Tcl_Obj *def[]={Tcl_NewStringObj("proc",-1),Tcl_NewStringObj("p",-1),args,body};Tcl_IncrRefCount(def[0]);Tcl_IncrRefCount(def[1]);
 if(Tcl_EvalObjv(in,4,def,0)!=TCL_OK)return 2;
 Tcl_Obj *call=Tcl_NewStringObj("p",-1);Tcl_IncrRefCount(call);int code=Tcl_EvalObjv(in,1,&call,0);printf("%d\t%d\t",i,code);ProbeSize len;const char *r=Tcl_GetStringFromObj(Tcl_GetObjResult(in),&len);hex(r,len);putchar('\t');
 Proc *proc=TclFindProc((Interp*)in,"p");if(!proc)return 3;for(CompiledLocal *l=proc->firstLocalPtr;l;l=l->nextPtr){if(l!=proc->firstLocalPtr)putchar(',');hex(l->name,(int)l->nameLength);}putchar('\t');
 ByteCode *bc=proc->bodyPtr->typePtr&&!strcmp(proc->bodyPtr->typePtr->name,"bytecode")?(ByteCode*)proc->bodyPtr->internalRep.twoPtrValue.ptr1:NULL;
 if(bc){for(unsigned char *pc=bc->codeStart;pc<bc->codeStart+bc->numCodeBytes;pc+=tclInstructionTable[*pc].numBytes){if(pc!=bc->codeStart)putchar(',');printf("%ld:%s",(long)(pc-bc->codeStart),tclInstructionTable[*pc].name);}}
 putchar('\t');if(bc)for(int n=0;n<bc->numExceptRanges;n++){if(n)putchar(',');ExceptionRange *e=&bc->exceptArrayPtr[n];printf("%ld:%ld:%ld:%ld:%ld:%ld:%ld",(long)e->type,(long)e->nestingLevel,(long)e->codeOffset,(long)e->numCodeBytes,(long)e->breakOffset,(long)e->continueOffset,(long)e->catchOffset);}
 putchar('\t');if(bc)for(int n=0;n<bc->numLitObjects;n++){if(n)putchar(',');r=Tcl_GetStringFromObj(bc->objArrayPtr[n],&len);hex(r,len);}putchar('\n');
 Tcl_DecrRefCount(call);for(int n=0;n<4;n++)Tcl_DecrRefCount(def[n]);Tcl_DeleteInterp(in);
 }Tcl_Finalize();return 0;}
