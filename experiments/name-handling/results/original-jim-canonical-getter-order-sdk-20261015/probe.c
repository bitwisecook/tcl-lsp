#include <stdio.h>
#include <string.h>
#include "jim.h"
extern Jim_Obj *JimCanonicalNamespace(Jim_Interp *, Jim_Obj *, Jim_Obj *);
static Jim_Obj *observed_namespace;
static const char *current_label;
static const char *name_bytes;
static int name_length;
static void update_name(Jim_Obj *object) {
 printf("NAMESPACE_AT_NAME_GETTER|%s|%d|%s\n", current_label, observed_namespace->bytes != NULL, observed_namespace->typePtr ? observed_namespace->typePtr->name : "none");
 object->bytes = Jim_Alloc(name_length + 1);
 memcpy(object->bytes, name_bytes, name_length);
 object->bytes[name_length] = 0;
 object->length = name_length;
}
static const Jim_ObjType name_type = { "canonical-name-probe", NULL, NULL, update_name, JIM_TYPE_NONE };
static Jim_Interp *fresh(void) { Jim_Interp *i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i); return Jim_InitStaticExtensions(i) == JIM_OK ? i : NULL; }
static void sample(const char *kind, Jim_Obj *object) { printf("%s|%s|%d|%s\n", kind, current_label, object->bytes != NULL, object->typePtr ? object->typePtr->name : "none"); }
static void row(const char *kind, Jim_Obj *object) { int length; const char *bytes = Jim_GetString(object, &length); printf("%s|%s|0|", kind, current_label); for (int n=0;n<length;n++) printf("%02x", (unsigned char)bytes[n]); puts(""); }
int main(void) {
 Jim_Interp *version=fresh(); if (!version) return 2; current_label="VERSION"; if (Jim_Eval(version,"info patchlevel") != JIM_OK) return 3; row("VERSION", Jim_GetResult(version)); Jim_FreeInterp(version);
 const struct { const char *label, *bytes; int length, root; } cases[] = {
  { "ABSOLUTE", "::target\0tail", 13, 0 },
  { "RELATIVE", "child\0tail", 10, 0 },
  { "ROOT_RELATIVE", "child\0tail", 10, 1 }
 };
 for (unsigned n=0;n<sizeof(cases)/sizeof(cases[0]);n++) {
  Jim_Interp *interp=fresh(); if (!interp) return 4;
  current_label=cases[n].label; name_bytes=cases[n].bytes; name_length=cases[n].length;
  Jim_Obj *namespace=cases[n].root ? Jim_NewStringObj(interp,"",0) : Jim_NewIntObj(interp,42); Jim_IncrRefCount(namespace); observed_namespace=namespace;
  Jim_Obj *name=Jim_NewStringObj(interp,"",0); Jim_IncrRefCount(name); Jim_InvalidateStringRep(name); name->typePtr=&name_type;
  sample("NAMESPACE_BEFORE", namespace); sample("NAME_BEFORE", name);
  Jim_Obj *result=JimCanonicalNamespace(interp,namespace,name); Jim_IncrRefCount(result);
  sample("NAMESPACE_AFTER",namespace); sample("NAME_AFTER",name);
  printf("RESULT_IDENTITY|%s|0|%s\n",current_label,result == name ? "31" : "30");
  row("RESULT",result); row("NAMESPACE_GETTER",namespace);
  Jim_DecrRefCount(interp,result); Jim_DecrRefCount(interp,name); Jim_DecrRefCount(interp,namespace); Jim_FreeInterp(interp);
 }
 return 0;
}
