// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <tcl.h>
#include <stdio.h>
static Tcl_Interp *active;
static int lengths, elements, duplicates;
static Tcl_Obj *pooled;
static Tcl_Size Length(Tcl_Obj *obj) {
    (void)obj; lengths++; Tcl_SetVar(active,"world","LENGTH",TCL_GLOBAL_ONLY); return 0;
}
static int Elements(Tcl_Interp *i,Tcl_Obj *obj,Tcl_Size *n,Tcl_Obj ***v) {
    (void)obj; elements++; Tcl_SetVar(i,"world","ELEMENTS",TCL_GLOBAL_ONLY); *n=0;*v=NULL;return TCL_OK;
}
static void Duplicate(Tcl_Obj *src,Tcl_Obj *dst) {
    duplicates++; dst->typePtr=src->typePtr; Tcl_SetVar(active,"world","DUPLICATE",TCL_GLOBAL_ONLY);
}
static void StringRep(Tcl_Obj *o) { Tcl_InitStringRep(o,"a b",3); }
static const Tcl_ObjType custom={"pooled_callbacks",NULL,Duplicate,StringRep,NULL,TCL_OBJTYPE_V2(Length,NULL,NULL,NULL,Elements,NULL,NULL,NULL)};
static int Poison(void *u,Tcl_Interp *i,Tcl_Size n,Tcl_Obj *const v[]) {
    (void)u;if(n!=2)return TCL_ERROR;active=i;pooled=v[1];
    if(pooled->typePtr && pooled->typePtr->freeIntRepProc) pooled->typePtr->freeIntRepProc(pooled);
    pooled->typePtr=&custom; Tcl_SetObjResult(i,Tcl_NewIntObj(1));return TCL_OK;
}
static void Probe(Tcl_Interp *i,const char *label,const char *src) {
    lengths=elements=duplicates=0; Tcl_SetVar(i,"world","SAFE",TCL_GLOBAL_ONLY);
    int code=Tcl_Eval(i,src);
    printf("%s|%d|%s|%s|%d|%d|%d\n",label,code,Tcl_GetStringResult(i),Tcl_GetVar(i,"world",TCL_GLOBAL_ONLY),lengths,elements,duplicates);
}
int main(int argc,char **argv) {
    (void)argc;Tcl_FindExecutable(argv[0]);Tcl_Interp *i=Tcl_CreateInterp();
    Tcl_CreateObjCommand2(i,"poison",Poison,NULL,NULL);
    Probe(i,"fresh_compiled","proc f {} {set ran NO; foreach v {a b} {set ran YES}; return $ran}; f");
    Probe(i,"poison_compiled","proc poisonLiteral {} {poison {a b}}; poisonLiteral; f");
    Probe(i,"later_compiled","proc h {} {set ran NO; foreach v {a b} {set ran YES}; return $ran}; h");
    Probe(i,"generic_same_pool","proc g {} {set iterate foreach; set ran NO; $iterate v {a b} {set ran YES}; return $ran}; g");
    Tcl_DeleteInterp(i);Tcl_Finalize();return 0;
}
