// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
#include <stdio.h>
#include "jim.h"
static void bytes(Jim_Obj *obj) {int len;const char *s=Jim_GetString(obj,&len);for(int n=0;n<len;n++)printf("%02x",(unsigned char)s[n]);}
static int run(const char *label, const char *pattern, const char *replacement)
{
    Jim_Interp *interp=Jim_CreateInterp();Jim_RegisterCoreCommands(interp);
    if(Jim_InitStaticExtensions(interp)!=JIM_OK)return 2;
    const char *setup="namespace eval ReceiptImportSrc {proc p {} {return P}}; alias ReceiptImportSrc::a ReceiptImportSrc::p; alias ReceiptImportSrc::b ReceiptImportSrc::a";
    if(Jim_Eval(interp,setup)!=JIM_OK)return 3;
    if(replacement && Jim_Eval(interp,replacement)!=JIM_OK)return 4;
    const char *source=pattern ? pattern : "namespace import";
    int code=Jim_Eval(interp,source);
    printf("IMPORT|%s|%d|",label,code);bytes(Jim_GetResult(interp));printf("\n");
    const char *report="list [info -nons commands p] [info -nons commands a] [info -nons commands b]";
    /* Restore a changed ordinary info implementation solely for this subsequent report. */
    if(replacement && Jim_Eval(interp,"catch {rename info {}}; catch {rename original_info info}")!=JIM_OK)return 5;
    code=Jim_Eval(interp,report);
    printf("COMMANDS|%s|%d|",label,code);bytes(Jim_GetResult(interp));printf("\n");
    code=Jim_Eval(interp,"if {[exists -command p]} {list [info alias p] [p]} elseif {[exists -command a]} {list [info alias a] [a]} elseif {[exists -command b]} {list [info alias b] [b]} else {list}");
    printf("ALIAS|%s|%d|",label,code);bytes(Jim_GetResult(interp));printf("\n");
    Jim_FreeInterp(interp);return 0;
}
int main(void)
{
    const char *labels[]={"SIMPLE","ALIAS","DEEP_ALIAS","EMPTY","EMPTY_REST","MUTABLE_INFO"};
    const char *inputs[]={"namespace import ::ReceiptImportSrc::p","namespace import ::ReceiptImportSrc::a","namespace import ::ReceiptImportSrc::b","namespace import ::ReceiptMissing::*",NULL,"namespace import ::ReceiptImportSrc::p"};
    for(int n=0;n<6;n++){int code=run(labels[n],inputs[n],n==5 ? "rename info original_info; proc info args {return {}}" : NULL);if(code)return code;}
    return 0;
}
