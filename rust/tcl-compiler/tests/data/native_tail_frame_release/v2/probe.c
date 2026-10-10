#include <stdint.h>
#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
#else
#include "tcl.h"
#endif
static unsigned made,freed[32],nfree;
#ifdef JIM_PROBE
static void drop(Jim_Interp *interp,Jim_Obj *object) {(void)interp;if(nfree<32)freed[nfree++]=(unsigned)(uintptr_t)object->internalRep.ptr;object->internalRep.ptr=NULL;}
static const Jim_ObjType tracked={"tail-release-observer",drop,NULL,NULL,0};
static int make_value(Jim_Interp *interp,int argc,Jim_Obj *const *argv) {(void)argc;(void)argv;Jim_Obj *value=Jim_NewStringObj(interp,"OPAQUE",6);if(value->typePtr!=NULL)return JIM_ERR;value->typePtr=&tracked;value->internalRep.ptr=(void*)(uintptr_t)++made;Jim_SetResult(interp,value);return JIM_OK;}
#else
static void drop(Tcl_Obj *object) {if(nfree<32)freed[nfree++]=(unsigned)(uintptr_t)object->internalRep.twoPtrValue.ptr1;object->internalRep.twoPtrValue.ptr1=NULL;}
static const Tcl_ObjType tracked={"tail-release-observer",drop,NULL,NULL,NULL};
static int make_value(ClientData data,Tcl_Interp *interp,int argc,Tcl_Obj *const *argv) {(void)data;(void)argc;(void)argv;Tcl_Obj *value=Tcl_NewStringObj("OPAQUE",6);value->typePtr=&tracked;value->internalRep.twoPtrValue.ptr1=(void*)(uintptr_t)++made;Tcl_SetObjResult(interp,value);return TCL_OK;}
#endif
#if !defined(JIM_PROBE) && (TCL_MAJOR_VERSION > 8 || TCL_MINOR_VERSION >= 5)
#define TRACE_INSTALL "trace add variable n unset [list watch $n]"
#else
#define TRACE_INSTALL "trace variable n u [list watch $n]"
#endif
static const char *sources[4][2]={
 {"proc f {n} {if {$n <= 0} {return [info level]}; f [expr {$n - 1}]}; f 2", "proc f {n} {while {1} {if {$n <= 0} {return [info level]};set n [expr {$n - 1}];continue}}; f 2"},
 {"set n 99; proc f {n} {if {$n <= 0} {upvar 1 n parent;return $parent};f [expr {$n - 1}]}; f 2", "set n 99; proc f {n} {while {1} {if {$n <= 0} {upvar 1 n parent;return $parent};set n [expr {$n - 1}];continue}}; f 2"},
 {"set events {};proc watch {tag args} {lappend ::events [list $tag [uplevel 1 {info level}]]};proc f {n} {" TRACE_INSTALL ";if {$n <= 0} {return DONE};f [expr {$n - 1}]};list [f 2] $events", "set events {};proc watch {tag args} {lappend ::events [list $tag [uplevel 1 {info level}]]};proc f {n} {while {1} {" TRACE_INSTALL ";if {$n <= 0} {return DONE};set n [expr {$n - 1}];continue}};list [f 2] $events"},
 {"proc f {n value} {if {$n <= 0} {return DONE};f [expr {$n - 1}] [make]};f 2 INITIAL", "proc f {n value} {while {1} {if {$n <= 0} {return DONE};set __next_n [expr {$n - 1}];set __next_value [make];set n $__next_n;set value $__next_value;unset __next_n __next_value;continue}};f 2 INITIAL"}
};
int main(int argc,char **argv){(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#endif
for(unsigned c=0;c<4;c++)for(unsigned mode=0;mode<2;mode++){made=nfree=0;
#ifdef JIM_PROBE
Jim_Interp *interp=Jim_CreateInterp();Jim_RegisterCoreCommands(interp);if(Jim_InitStaticExtensions(interp)!=JIM_OK)return 2;Jim_CreateCommand(interp,"make",make_value,NULL,NULL);int code=Jim_Eval(interp,sources[c][mode]);int length;const char *bytes=Jim_GetString(Jim_GetResult(interp),&length);
#else
Tcl_Interp *interp=Tcl_CreateInterp();if(Tcl_Init(interp)!=TCL_OK)return 2;Tcl_CreateObjCommand(interp,"make",make_value,NULL,NULL);int code=Tcl_Eval(interp,sources[c][mode]);int length;
#if TCL_MAJOR_VERSION>=9
Tcl_Size native_length;const char *bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&native_length);length=(int)native_length;
#else
const char *bytes=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&length);
#endif
#endif
printf("case\t%u\t%s\t%d\t",c,mode?"loop":"original",code);for(int i=0;i<length;i++)printf("%02x",(unsigned char)bytes[i]);printf("\tmade=%u\tfree=",made);for(unsigned i=0;i<nfree;i++)printf("%s%u",i?",":"",freed[i]);puts("");
#ifdef JIM_PROBE
Jim_FreeInterp(interp);
#else
Tcl_DeleteInterp(interp);
#endif
}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;}
