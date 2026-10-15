// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "jim.h"

static Jim_CmdProc *original_alias;
static void bytes(Jim_Obj *obj)
{
    int len;
    const char *s = Jim_GetString(obj, &len);
    for (int n = 0; n < len; n++) printf("%02x", (unsigned char)s[n]);
}
static int report(Jim_Interp *interp, const char *label, const char *script)
{
    int code = Jim_Eval(interp, script);
    printf("%s|%d|", label, code);
    bytes(Jim_GetResult(interp));
    printf("\n");
    return code;
}
static int publish_then_break(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int code = original_alias(interp, argc, argv);
    if (code != JIM_OK) return code;
    printf("BREAK_PUBLISHED|0|");
    bytes(argv[1]);
    printf("\n");
    return JIM_BREAK;
}
int main(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    if (report(interp, "SETUP", "namespace eval ReceiptReuse {proc p {} {return P}; proc q {} {return Q}}; alias ReceiptReuse::a ReceiptReuse::p") != JIM_OK) return 3;
    const char *import = "namespace import ::ReceiptReuse::a";
    if (report(interp, "REUSE_1", import) != JIM_OK) return 4;
    if (report(interp, "VALUE_1", "list [info alias a] [a]") != JIM_OK) return 5;
    if (report(interp, "REPLACE", "alias ReceiptReuse::a ReceiptReuse::q") != JIM_OK) return 6;
    if (report(interp, "REUSE_2", import) != JIM_OK) return 7;
    if (report(interp, "VALUE_2", "list [info alias a] [a]") != JIM_OK) return 8;
    if (report(interp, "RECREATE", "namespace delete ReceiptReuse; namespace eval ReceiptReuse {proc p {} {return R}}; alias ReceiptReuse::a ReceiptReuse::p") != JIM_OK) return 9;
    if (report(interp, "REUSE_3", import) != JIM_OK) return 10;
    if (report(interp, "VALUE_3", "list [info alias a] [a]") != JIM_OK) return 11;
    if (report(interp, "REPEATED_PATTERN", "namespace import ::ReceiptReuse::a ::ReceiptReuse::a") != JIM_OK) return 12;
    if (report(interp, "VALUE_4", "list [info alias a] [a]") != JIM_OK) return 13;
    Jim_FreeInterp(interp);

    interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 14;
    if (report(interp, "BREAK_SETUP", "namespace eval ReceiptBreak {proc p {} {return P}; proc q {} {return Q}}") != JIM_OK) return 15;
    Jim_Obj *name = Jim_NewStringObj(interp, "alias", -1);
    Jim_IncrRefCount(name);
    Jim_Cmd *cmd = Jim_GetCommand(interp, name, JIM_ERRMSG);
    if (!cmd || (cmd->flags & JIM_CMD_ISPROC) || cmd->u.native.privData) return 16;
    original_alias = cmd->u.native.cmdProc;
    Jim_DecrRefCount(interp, name);
    if (Jim_CreateCommand(interp, "alias", publish_then_break, NULL, NULL) != JIM_OK) return 17;
    if (report(interp, "BREAK_IMPORT", "namespace import ::ReceiptBreak::*") != JIM_OK) return 18;
    if (report(interp, "BREAK_COMMANDS", "list [info -nons commands p] [info -nons commands q]") != JIM_OK) return 19;
    Jim_FreeInterp(interp);
    return 0;
}
