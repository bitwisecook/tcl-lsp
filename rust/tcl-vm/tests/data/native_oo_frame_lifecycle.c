// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tcl.h>
#include <stdio.h>
#include <string.h>
static int evaluate(Tcl_Interp *i,const char *s){int code=Tcl_Eval(i,s);printf("SCRIPT %d %s\n",code,Tcl_GetStringResult(i));return code;}
int main(int argc,char **argv){Tcl_FindExecutable(argv[0]);Tcl_Interp*i=Tcl_CreateInterp();
if(evaluate(i,"oo::class create C {method object {} {self object}; method deleted {} {rename [self] {}; self}}; C create obj"))return 1;
if(evaluate(i,"obj object"))return 2;
Tcl_Obj *first=Tcl_GetObjResult(i);Tcl_IncrRefCount(first);
evaluate(i,"rename obj renamed");evaluate(i,"renamed object");Tcl_Obj *renamed=Tcl_GetObjResult(i);Tcl_IncrRefCount(renamed);
evaluate(i,"renamed deleted");printf("RETAINED %s %s\n",Tcl_GetString(first),Tcl_GetString(renamed));
Tcl_DecrRefCount(first);Tcl_DecrRefCount(renamed);
evaluate(i,"proc ::method args {error GLOBAL_WORKER}; namespace eval caller {proc build {} {set private CALLER; oo::class create ::D {set scope [namespace current]; set inherited [info exists private]; set local [uplevel 1 {set private}]; proc born {} {return HERE}; meth m {} {return OK}}}}; caller::build; D create other; list [other m] $::oo::define::scope $::oo::define::inherited $::oo::define::local [::oo::define::born]");
evaluate(i,"proc frame_parent {} {set private CALLER; namespace eval ::child {set local [uplevel 1 {set private}]; namespace current}; namespace current}; namespace eval ::caller {frame_parent}");
evaluate(i,"namespace eval ::caller {proc parent {} {set private CALLER; set got [namespace eval ::child {set fromScript [uplevel 1 {set private}]; set fromList [uplevel 1 [list set private]]; list $fromScript $fromList [namespace current]}]; list $got [namespace current]}; parent}");
Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
