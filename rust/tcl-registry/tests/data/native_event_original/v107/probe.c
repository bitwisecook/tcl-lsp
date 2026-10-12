#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
typedef Jim_Interp Interp;
typedef Jim_Obj Obj;
static Interp *fresh(void) { Interp *i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if (Jim_InitStaticExtensions(i) != JIM_OK) return NULL; return i; }
static Obj *string(Interp *i, const char *p, int n) { return Jim_NewStringObj(i, p, n); }
static void retain(Obj *o) { Jim_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { Jim_DecrRefCount(i, o); }
static int invoke(Interp *i, int n, Obj **v) { return Jim_EvalObjVector(i, n, v); }
static int source(Interp *i, const char *p, int n) { Obj *o = string(i, p, n); retain(o); int c = Jim_EvalObj(i, o); release(i, o); return c; }
static void result(Interp *i, const char *label, int c) { int n; const char *p = Jim_GetString(Jim_GetResult(i), &n); printf("%s|%d|", label, c); for (int k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); puts(""); }
static void options(Interp *i, const char *label, int c) { (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-C-return-options-api\n", label, c); }
static void destroy(Interp *i) { Jim_FreeInterp(i); }
#else
#include "tcl.h"
typedef Tcl_Interp Interp;
typedef Tcl_Obj Obj;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
static Interp *fresh(void) { Interp *i = Tcl_CreateInterp(); if (Tcl_Init(i) != TCL_OK) return NULL; return i; }
static Obj *string(Interp *i, const char *p, int n) { (void)i; return Tcl_NewStringObj(p, n); }
static void retain(Obj *o) { Tcl_IncrRefCount(o); }
static void release(Interp *i, Obj *o) { (void)i; Tcl_DecrRefCount(o); }
static int invoke(Interp *i, int n, Obj **v) { return Tcl_EvalObjv(i, n, v, 0); }
static int source(Interp *i, const char *p, int n) { return Tcl_EvalEx(i, p, n, 0); }
static void bytes(const char *p, Count n) { for (Count k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); }
static void result(Interp *i, const char *label, int c) { Count n; const char *p = Tcl_GetStringFromObj(Tcl_GetObjResult(i), &n); printf("%s|%d|", label, c); bytes(p, n); puts(""); }
static void options(Interp *i, const char *label, int c) {
#if TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION == 4
    (void)i; printf("%s_OPTIONS_NOT_TESTED|%d|no-Tcl_GetReturnOptions\n", label, c);
#else
    Obj *o = Tcl_GetReturnOptions(i, c); retain(o); Count n; const char *p = Tcl_GetStringFromObj(o, &n); printf("%s_OPTIONS|%d|", label, c); bytes(p, n); puts(""); release(i, o);
#endif
}
static void destroy(Interp *i) { Tcl_DeleteInterp(i); }
#endif

static void input(const char *label, const char *p, int n) { printf("%s_INPUT|0|", label); for (int k = 0; k < n; k++) printf("%02x", (unsigned char)p[k]); puts(""); }


#ifdef JIM_PROBE
static Obj *list(Interp *i,int n,Obj **v) { return Jim_NewListObj(i,v,n); }
#else
static Obj *list(Interp *i,int n,Obj **v) { (void)i; return Tcl_NewListObj(n,v); }
#endif
int main(int argc,char **argv) {
 setvbuf(stdout,NULL,_IONBF,0);int selected=argc==2?atoi(argv[1]):-1;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh();if(!i)return 2;int c=source(i,"info patchlevel",(int)(sizeof("info patchlevel")-1));result(i,"VERSION",c);destroy(i);
 if(selected==0 || selected<0) { i=fresh();if(!i)return 2;input("SAME_STAGGER","set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 10 {set x NEXT; lappend log NEXT}; vwait x; set log",(int)(sizeof("set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 10 {set x NEXT; lappend log NEXT}; vwait x; set log")-1));c=source(i,"set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 10 {set x NEXT; lappend log NEXT}; vwait x; set log",(int)(sizeof("set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 10 {set x NEXT; lappend log NEXT}; vwait x; set log")-1));result(i,"SAME_STAGGER",c);options(i,"SAME_STAGGER",c);destroy(i); }
 if(selected==1 || selected<0) { i=fresh();if(!i)return 2;input("ARRAY_STAGGER","array set a {k OLD}; after 0 {set a(k) OLD}; after 10 {set a(k) NEW}; vwait a(k); set a(k)",(int)(sizeof("array set a {k OLD}; after 0 {set a(k) OLD}; after 10 {set a(k) NEW}; vwait a(k); set a(k)")-1));c=source(i,"array set a {k OLD}; after 0 {set a(k) OLD}; after 10 {set a(k) NEW}; vwait a(k); set a(k)",(int)(sizeof("array set a {k OLD}; after 0 {set a(k) OLD}; after 10 {set a(k) NEW}; vwait a(k); set a(k)")-1));result(i,"ARRAY_STAGGER",c);options(i,"ARRAY_STAGGER",c);destroy(i); }
 if(selected==2 || selected<0) { i=fresh();if(!i)return 2;input("BATCH_CANCEL","set log {}; after 0 {lappend ::log FIRST; after cancel $::later}; set ::later [after 0 {lappend ::log SECOND}]; update; set log",(int)(sizeof("set log {}; after 0 {lappend ::log FIRST; after cancel $::later}; set ::later [after 0 {lappend ::log SECOND}]; update; set log")-1));c=source(i,"set log {}; after 0 {lappend ::log FIRST; after cancel $::later}; set ::later [after 0 {lappend ::log SECOND}]; update; set log",(int)(sizeof("set log {}; after 0 {lappend ::log FIRST; after cancel $::later}; set ::later [after 0 {lappend ::log SECOND}]; update; set log")-1));result(i,"BATCH_CANCEL",c);options(i,"BATCH_CANCEL",c);destroy(i); }
 if(selected==3 || selected<0) { i=fresh();if(!i)return 2;input("BATCH_NEW","set log {}; after 0 {lappend ::log FIRST; after 0 {lappend ::log NEW}}; after 0 {lappend ::log SECOND}; update; set log",(int)(sizeof("set log {}; after 0 {lappend ::log FIRST; after 0 {lappend ::log NEW}}; after 0 {lappend ::log SECOND}; update; set log")-1));c=source(i,"set log {}; after 0 {lappend ::log FIRST; after 0 {lappend ::log NEW}}; after 0 {lappend ::log SECOND}; update; set log",(int)(sizeof("set log {}; after 0 {lappend ::log FIRST; after 0 {lappend ::log NEW}}; after 0 {lappend ::log SECOND}; update; set log")-1));result(i,"BATCH_NEW",c);options(i,"BATCH_NEW",c);destroy(i); }
 if(selected==4 || selected<0) { i=fresh();if(!i)return 2;input("GLOBAL_PREFIX_BGERROR","set done 0; proc handler {prefix message options} {set ::done [list $prefix $message [dict get $options -code]]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done",(int)(sizeof("set done 0; proc handler {prefix message options} {set ::done [list $prefix $message [dict get $options -code]]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done")-1));c=source(i,"set done 0; proc handler {prefix message options} {set ::done [list $prefix $message [dict get $options -code]]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done",(int)(sizeof("set done 0; proc handler {prefix message options} {set ::done [list $prefix $message [dict get $options -code]]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done")-1));result(i,"GLOBAL_PREFIX_BGERROR",c);options(i,"GLOBAL_PREFIX_BGERROR",c);destroy(i); }

#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
