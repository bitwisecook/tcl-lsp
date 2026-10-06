/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "jim.h"
#include <stdio.h>
int main(void) {Jim_Interp *ip=Jim_CreateInterp();Jim_Obj *name;Jim_RegisterCoreCommands(ip);name=Jim_NewStringObj(ip,"lseq",4);Jim_IncrRefCount(name);printf("SUPPORTED\t%d\n",Jim_GetCommand(ip,name,JIM_NONE)!=NULL);Jim_DecrRefCount(ip,name);Jim_FreeInterp(ip);return 0;}
