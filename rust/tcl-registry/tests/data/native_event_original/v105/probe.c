#include <stdio.h>
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
 (void)argc;
#ifndef JIM_PROBE
 Tcl_FindExecutable(argv[0]);
#else
 (void)argv;
#endif
 Interp *i=fresh();if(!i)return 2;int c=source(i,"info patchlevel",15);result(i,"VERSION",c);destroy(i);
 i=fresh();if(!i)return 2;input("GLOBAL_FROM_NAMESPACE","set ::done 0; namespace eval N {variable x LOCAL; after 0 {set x GLOBAL; set ::done 1}; vwait ::done; list $x $::x}",115);c=source(i,"set ::done 0; namespace eval N {variable x LOCAL; after 0 {set x GLOBAL; set ::done 1}; vwait ::done; list $x $::x}",115);result(i,"GLOBAL_FROM_NAMESPACE",c);options(i,"GLOBAL_FROM_NAMESPACE",c);destroy(i);
 i=fresh();if(!i)return 2;input("GLOBAL_FROM_PROC","set ::x OLD; proc p {} {set x LOCAL; after 0 {set x NEW}; vwait x; list $x $::x}; p",83);c=source(i,"set ::x OLD; proc p {} {set x LOCAL; after 0 {set x NEW}; vwait x; list $x $::x}; p",83);result(i,"GLOBAL_FROM_PROC",c);options(i,"GLOBAL_FROM_PROC",c);destroy(i);
 i=fresh();if(!i)return 2;input("SAME_WRITE","set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 0 {set x NEXT; lappend log NEXT}; vwait x; set log",120);c=source(i,"set x SAME; set log {}; after 0 {set x SAME; lappend log SAME}; after 0 {set x NEXT; lappend log NEXT}; vwait x; set log",120);result(i,"SAME_WRITE",c);options(i,"SAME_WRITE",c);destroy(i);
 i=fresh();if(!i)return 2;input("ARRAY_SAME_WRITE","array set a {k OLD}; after 0 {set a(k) OLD}; after 0 {set a(k) NEW}; vwait a(k); set a(k)",89);c=source(i,"array set a {k OLD}; after 0 {set a(k) OLD}; after 0 {set a(k) NEW}; vwait a(k); set a(k)",89);result(i,"ARRAY_SAME_WRITE",c);options(i,"ARRAY_SAME_WRITE",c);destroy(i);
 i=fresh();if(!i)return 2;input("UNSET_WRITE","set x OLD; after 0 {unset x}; vwait x; info exists x",52);c=source(i,"set x OLD; after 0 {unset x}; vwait x; info exists x",52);result(i,"UNSET_WRITE",c);options(i,"UNSET_WRITE",c);destroy(i);
 i=fresh();if(!i)return 2;input("NO_SOURCES","vwait absent",12);c=source(i,"vwait absent",12);result(i,"NO_SOURCES",c);options(i,"NO_SOURCES",c);destroy(i);
 i=fresh();if(!i)return 2;input("CONCAT_TRIM","set done 0; after 0 { set} { done } {1 }; vwait done; set done",62);c=source(i,"set done 0; after 0 { set} { done } {1 }; vwait done; set done",62);result(i,"CONCAT_TRIM",c);options(i,"CONCAT_TRIM",c);destroy(i);
 i=fresh();if(!i)return 2;input("PURE_LIST_SCRIPT","set done 0; set body [list set done {A B}]; after 0 $body; vwait done; set done",79);c=source(i,"set done 0; set body [list set done {A B}]; after 0 $body; vwait done; set done",79);result(i,"PURE_LIST_SCRIPT",c);options(i,"PURE_LIST_SCRIPT",c);destroy(i);
 i=fresh();if(!i)return 2;input("CANCEL_COUNTED","set done 0; after 0 {set done 1}; after cancel {set done 1}; update; set done",77);c=source(i,"set done 0; after 0 {set done 1}; after cancel {set done 1}; update; set done",77);result(i,"CANCEL_COUNTED",c);options(i,"CANCEL_COUNTED",c);destroy(i);
 i=fresh();if(!i)return 2;input("CANCEL_CONCAT","set done 0; after 0 {set done 1}; after cancel { set} { done 1 }; update; set done",82);c=source(i,"set done 0; after 0 {set done 1}; after cancel { set} { done 1 }; update; set done",82);result(i,"CANCEL_CONCAT",c);options(i,"CANCEL_CONCAT",c);destroy(i);
 i=fresh();if(!i)return 2;input("INFO_SCRIPT","set id [after idle {set done 1}]; lindex [after info $id] 0",59);c=source(i,"set id [after idle {set done 1}]; lindex [after info $id] 0",59);result(i,"INFO_SCRIPT",c);options(i,"INFO_SCRIPT",c);destroy(i);
 i=fresh();if(!i)return 2;input("INFO_KIND","set id [after 0 {set done 1}]; lindex [after info $id] 1",56);c=source(i,"set id [after 0 {set done 1}]; lindex [after info $id] 1",56);result(i,"INFO_KIND",c);options(i,"INFO_KIND",c);destroy(i);
 i=fresh();if(!i)return 2;input("INFO_MISSING","after info after#99999999",25);c=source(i,"after info after#99999999",25);result(i,"INFO_MISSING",c);options(i,"INFO_MISSING",c);destroy(i);
 i=fresh();if(!i)return 2;input("UPDATE_IDLE","set x OLD; after 0 {set x TIMER}; after idle {set x IDLE}; update idletasks; set x",82);c=source(i,"set x OLD; after 0 {set x TIMER}; after idle {set x IDLE}; update idletasks; set x",82);result(i,"UPDATE_IDLE",c);options(i,"UPDATE_IDLE",c);destroy(i);
 i=fresh();if(!i)return 2;input("DELAY_NO_EVENTS","set x OLD; after 0 {set x TIMER}; after 1; set x",48);c=source(i,"set x OLD; after 0 {set x TIMER}; after 1; set x",48);result(i,"DELAY_NO_EVENTS",c);options(i,"DELAY_NO_EVENTS",c);destroy(i);
 i=fresh();if(!i)return 2;input("AFTER_ABBREVIATION","after in",8);c=source(i,"after in",8);result(i,"AFTER_ABBREVIATION",c);options(i,"AFTER_ABBREVIATION",c);destroy(i);
 i=fresh();if(!i)return 2;input("AFTER_AMBIGUOUS","after i",7);c=source(i,"after i",7);result(i,"AFTER_AMBIGUOUS",c);options(i,"AFTER_AMBIGUOUS",c);destroy(i);
 i=fresh();if(!i)return 2;input("UPDATE_ABBREVIATION","update i",8);c=source(i,"update i",8);result(i,"UPDATE_ABBREVIATION",c);options(i,"UPDATE_ABBREVIATION",c);destroy(i);
 i=fresh();if(!i)return 2;input("PREFIX_BGERROR","set done 0; proc handler {prefix message} {set ::done [list $prefix $message]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done",160);c=source(i,"set done 0; proc handler {prefix message} {set ::done [list $prefix $message]}; interp bgerror {} [list handler PRE]; after 0 {error BOOM}; vwait done; set done",160);result(i,"PREFIX_BGERROR",c);options(i,"PREFIX_BGERROR",c);destroy(i);

 const char names[][7]={{'k',(char)0xff},{'k',0,'t','a','i','l'},{'k',(char)0xc0,(char)0x80,'t','a','i','l'}};const int lengths[]={2,6,7};const char *tags[]={"RAW_FF","RAW_ZERO","ENCODED_ZERO"};
 for(int mode=0;mode<3;mode++) {
  i=fresh();if(!i)return 2;char label[100];Obj *key=string(i,names[mode],lengths[mode]);retain(key);
  Obj *bodyWords[]={string(i,"set",3),key,string(i,"VALUE",5)};Obj *body=list(i,3,bodyWords);retain(body);
  Obj *schedule[]={string(i,"after",5),string(i,"0",1),body};for(int k=0;k<3;k++)retain(schedule[k]);
  snprintf(label,sizeof(label),"ORIGINAL_LIST_%s_SCHEDULE",tags[mode]);input(label,names[mode],lengths[mode]);c=invoke(i,3,schedule);result(i,label,c);options(i,label,c);for(int k=0;k<3;k++)release(i,schedule[k]);
  c=source(i,"update",6);snprintf(label,sizeof(label),"ORIGINAL_LIST_%s_UPDATE",tags[mode]);result(i,label,c);options(i,label,c);
  Obj *get[]={string(i,"set",3),key};retain(get[0]);c=invoke(i,2,get);snprintf(label,sizeof(label),"ORIGINAL_LIST_%s_VALUE",tags[mode]);result(i,label,c);options(i,label,c);release(i,get[0]);release(i,body);release(i,key);destroy(i);
 }
#ifndef JIM_PROBE
 Tcl_Finalize();
#endif
 return 0;
}
