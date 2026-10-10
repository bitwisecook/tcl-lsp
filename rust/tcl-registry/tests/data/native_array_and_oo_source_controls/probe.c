#include <stdio.h>
#ifdef JIM_PROBE
#include "jim.h"
#else
#include "tcl.h"
#endif
static const char *cases[][2] = {
{"array-set-parens","array set a(b) {k v}; list [array exists a] [array exists a(b)] [info exists a(b)]"},
{"scalar-set-parens","set a(b) VALUE; list [array exists a] [array exists a(b)] [set a(b)]"},
{"array-get-parens","set a(b) VALUE; array get a(b)"},
{"array-unset-parens","set a(b) VALUE; array unset a(b); list [info exists a(b)] [array exists a]"},
{"array-names-parens","set a(b) VALUE; array names a(b)"},
{"array-set-qualified-parens","namespace eval N {}; array set ::N::a(b) {k v}; list [array exists ::N::a] [array exists ::N::a(b)] [info exists ::N::a(b)]"},
{"separate-root-array","set {a(b)(k)} VALUE; list [array exists a] [array exists a(b)] [set {a(b)(k)}]"},
{"superclass-getter","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass; info class superclasses A"},
{"superclass-one","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass B; info class superclasses A"},
{"superclass-set-one","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass -set B; info class superclasses A"},
{"superclass-set-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass -set; info class superclasses A"},
{"superclass-clear","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass -clear; info class superclasses A"},
{"superclass-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass [binary format c 0]; info class superclasses A"},
{"superclass-zero-suffix","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass [binary format H* 00536574]; info class superclasses A"},
{"superclass-dash-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass [binary format H* 2d00736574]; info class superclasses A"},
{"superclass-escaped-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A superclass -s\\u0000et; info class superclasses A"},
{"mixin-getter","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin; info class mixins A"},
{"mixin-one","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin B; info class mixins A"},
{"mixin-set-one","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin -set B; info class mixins A"},
{"mixin-set-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin -set; info class mixins A"},
{"mixin-clear","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin -clear; info class mixins A"},
{"mixin-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin [binary format c 0]; info class mixins A"},
{"mixin-zero-suffix","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin [binary format H* 00536574]; info class mixins A"},
{"mixin-dash-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin [binary format H* 2d00736574]; info class mixins A"},
{"mixin-escaped-zero","oo::class create A {}; oo::class create B {}; oo::class create C {}; oo::define A mixin -s\\u0000et; info class mixins A"},
};
int main(int argc, char **argv) {
(void)argc;
#ifndef JIM_PROBE
Tcl_FindExecutable(argv[0]);
#endif
for (unsigned c=0;c<sizeof(cases)/sizeof(cases[0]);c++) {
#ifdef JIM_PROBE
(void)argv; Jim_Interp *interp=Jim_CreateInterp(); Jim_RegisterCoreCommands(interp);
if(Jim_InitStaticExtensions(interp)!=JIM_OK)return 2;
int code=Jim_Eval(interp,cases[c][1]); int length; const char *value=Jim_GetString(Jim_GetResult(interp),&length);
#else
Tcl_Interp *interp=Tcl_CreateInterp(); if(Tcl_Init(interp)!=TCL_OK)return 2;
int code=Tcl_Eval(interp,cases[c][1]);
#if TCL_MAJOR_VERSION>=9
Tcl_Size length;
#else
int length;
#endif
const char *value=Tcl_GetStringFromObj(Tcl_GetObjResult(interp),&length);
#endif
printf("%s\t%d\t%ld\t",cases[c][0],code,(long)length);
for(long i=0;i<(long)length;i++)printf("%02x",(unsigned char)value[i]); puts("");
#ifdef JIM_PROBE
Jim_FreeInterp(interp);
#else
Tcl_DeleteInterp(interp);
#endif
}
#ifndef JIM_PROBE
Tcl_Finalize();
#endif
return 0;
}
