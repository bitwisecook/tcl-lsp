// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later


#include <stdio.h>
#include <string.h>
#include "jim.h"
static const char *phase;
static void sample(const char *kind,Jim_Obj *o) {
 printf("%s|%s|%d|%s\n",kind,phase,o->bytes!=NULL,o->typePtr?o->typePtr->name:"none");
}
static void result(Jim_Interp *i,int code,Jim_Obj *argument) {
 Jim_Obj *r=Jim_GetResult(i);sample("RESULT_HEADER",r);
 if(code==0)printf("RESULT_ARGUMENT_IS_ORIGINAL|%s|%d\n",phase,Jim_ListLength(i,r)==2 && Jim_ListGetIndex(i,r,1)==argument);
 int length;const char *bytes=Jim_GetString(r,&length);printf("RESULT|%s|%d|",phase,code);
 for(int k=0;k<length;k++)printf("%02x",(unsigned char)bytes[k]);puts("");
}
int main(void) {
 setvbuf(stdout,NULL,_IONBF,0);
 const struct {const char *label,*member;int length,count,change;} cases[]={
 {"EXACT","upvar",5,1,0},{"ABBREVIATED","upv",3,1,0},
 {"EXACT_ARITY","upvar",5,0,0},{"ABBREVIATED_ARITY","upv",3,0,0},
 {"EXACT_NUL","upvar\0TAIL",10,1,0},{"ABBREVIATED_NUL","upv\0TAIL",8,1,0},
 {"ABBREVIATED_CANONICAL_REPLACED","upv",3,1,1},
 {"ABBREVIATED_SHORT_REPLACED","upv",3,1,2},{"EXACT_SHORT_REPLACED","upvar",5,1,2},
 {"EXACT_BOTH_REPLACED","upvar",5,1,3},{"ABBREVIATED_BOTH_REPLACED","upv",3,1,3},
 {"EXACT_NUL_BOTH_REPLACED","upvar\0TAIL",10,1,3}
 };
 for(unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
  Jim_Interp *i=Jim_CreateInterp();Jim_RegisterCoreCommands(i);if(Jim_InitStaticExtensions(i)!=JIM_OK)return 2;
  if((cases[n].change&1) && Jim_Eval(i,"proc {namespace upvar} {args} {return [list FULL {*}$args]}")!=JIM_OK)return 3;
  if((cases[n].change&2) && Jim_Eval(i,"proc {namespace upv} {args} {return [list SHORT {*}$args]}")!=JIM_OK)return 4;
  Jim_Obj *caller=Jim_NewIntObj(i,42);Jim_IncrRefCount(caller);
  Jim_Obj *head=Jim_NewStringObj(i,"namespace",-1);Jim_IncrRefCount(head);
  Jim_Obj *member=Jim_NewStringObj(i,cases[n].member,cases[n].length);Jim_IncrRefCount(member);
  for(int reuse=0;reuse<2;reuse++) {
   char label[128];snprintf(label,sizeof(label),"%s_%s",cases[n].label,reuse?"REUSE":"COLD");phase=label;
   if(reuse)Jim_InvalidateStringRep(caller);
   Jim_Obj *argument=Jim_NewIntObj(i,84);Jim_IncrRefCount(argument);
   Jim_Obj *argv[]={head,member,argument};Jim_Obj *vector=Jim_NewListObj(i,argv,2+cases[n].count);Jim_IncrRefCount(vector);
   sample("CALLER_BEFORE",caller);sample("MEMBER_BEFORE",member);sample("ARGUMENT_BEFORE",argument);
   int code=Jim_EvalNamespace(i,vector,caller);
   sample("CALLER_AFTER",caller);sample("MEMBER_AFTER",member);sample("ARGUMENT_AFTER",argument);
   result(i,code,argument);
   Jim_DecrRefCount(i,vector);Jim_DecrRefCount(i,argument);
  }
  Jim_DecrRefCount(i,member);Jim_DecrRefCount(i,head);Jim_DecrRefCount(i,caller);Jim_FreeInterp(i);
 }
 return 0;
}
