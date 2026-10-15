#include <stdio.h>
#include "jim.h"
static Jim_Interp *fresh(void) { Jim_Interp *i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i); return Jim_InitStaticExtensions(i) == JIM_OK ? i : NULL; }
static void row(const char *kind, const char *label, int code, Jim_Obj *object) { int length; const char *bytes = Jim_GetString(object, &length); printf("%s|%s|%d|", kind, label, code); for (int n = 0; n < length; n++) printf("%02x", (unsigned char)bytes[n]); puts(""); }
static void sample(const char *kind, const char *label, Jim_Obj *object) { printf("%s|%s|%d|%s\n", kind, label, object->bytes != NULL, object->typePtr ? object->typePtr->name : "none"); }
struct testcase { const char *label, *body, *formal, *argument; };
int main(void) {
 Jim_Interp *version = fresh(); if (!version) return 2; int code = Jim_Eval(version, "info patchlevel"); row("VERSION", "VERSION", code, Jim_GetResult(version)); Jim_FreeInterp(version);
 const struct testcase cases[] = {
  { "MISSING_LOCAL_HEAD", "$missing", NULL, NULL },
  { "MISSING_ROOT_HEAD", "${::missing}", NULL, NULL },
  { "MISSING_LOCAL_ARGUMENT", "return $missing", NULL, NULL },
  { "MISSING_ROOT_ARGUMENT", "::return ${::missing}", NULL, NULL },
  { "LOCAL_ARGUMENT", "return $arg", "arg", "VALUE" },
  { "ROOT_ARGUMENT", "::return ${::global}", NULL, NULL },
  { "COMPUTED_LOCAL_HEAD", "$arg VALUE", "arg", "::return" },
  { "BRACKET_MISSING_ARGUMENT", "return [set missing]", NULL, NULL },
  { "COMMENT_CONTROL", "# only a comment", NULL, NULL },
  { "INVALID_CONTROL", "{", NULL, NULL }
 };
 for (unsigned n = 0; n < sizeof(cases) / sizeof(cases[0]); n++) {
  const struct testcase *test = &cases[n]; Jim_Interp *interp = fresh(); if (!interp) return 3;
  if (Jim_Eval(interp, "set ::global VALUE") != JIM_OK) return 4;
  Jim_Obj *namespace = Jim_NewIntObj(interp, 42); Jim_IncrRefCount(namespace);
  Jim_Obj *formals = test->formal ? Jim_NewStringObj(interp, test->formal, -1) : Jim_NewListObj(interp, NULL, 0);
  Jim_Obj *elements[] = { formals, Jim_NewStringObj(interp, test->body, -1), namespace };
  Jim_Obj *lambda = Jim_NewListObj(interp, elements, 3); Jim_IncrRefCount(lambda);
  Jim_Obj *args[] = { Jim_NewStringObj(interp, "apply", -1), lambda, NULL }; Jim_IncrRefCount(args[0]);
  if (test->argument) { args[2] = Jim_NewStringObj(interp, test->argument, -1); Jim_IncrRefCount(args[2]); }
  sample("BEFORE", test->label, namespace);
  code = Jim_EvalObjVector(interp, test->argument ? 3 : 2, args);
  sample("AFTER", test->label, namespace); row("RESULT", test->label, code, Jim_GetResult(interp)); row("GETTER", test->label, 0, namespace);
  if (test->argument) Jim_DecrRefCount(interp, args[2]); Jim_DecrRefCount(interp, args[0]); Jim_DecrRefCount(interp, lambda); Jim_DecrRefCount(interp, namespace); Jim_FreeInterp(interp);
 }
 return 0;
}
