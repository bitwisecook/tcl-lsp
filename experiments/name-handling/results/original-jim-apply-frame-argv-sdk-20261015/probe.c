#include <stdio.h>
#include "jim.h"
static Jim_Interp *fresh(void) { Jim_Interp *i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i); return Jim_InitStaticExtensions(i) == JIM_OK ? i : NULL; }
static void row(const char *kind, const char *label, int code, Jim_Obj *object) { int length; const char *bytes = Jim_GetString(object, &length); printf("%s|%s|%d|", kind, label, code); for (int n = 0; n < length; n++) printf("%02x", (unsigned char)bytes[n]); puts(""); }
static void sample(const char *kind, const char *label, Jim_Obj *object) { printf("%s|%s|%d|%s\n", kind, label, object->bytes != NULL, object->typePtr ? object->typePtr->name : "none"); }
struct testcase { const char *label, *formal, *body; int level; };
int main(void) {
 Jim_Interp *version = fresh(); if (!version) return 2; int code = Jim_Eval(version, "info patchlevel"); row("VERSION", "VERSION", code, Jim_GetResult(version)); Jim_FreeInterp(version);
 const struct testcase cases[] = {
  { "COMMENT", "arg", "# only a comment", 0 },
  { "INVALID", "arg", "{", 0 },
  { "RETURN", "arg", "return $arg", 0 },
  { "LEVEL", "arg", "info level 0", 1 },
  { "MISSING", "arg", "return $missing", 0 },
  { "WRONG_ARITY", "", "# only a comment", 0 },
  { "EMPTY", "arg", "", 0 }
 };
 for (unsigned n = 0; n < sizeof(cases) / sizeof(cases[0]); n++) {
  const struct testcase *test = &cases[n]; Jim_Interp *interp = fresh(); if (!interp) return 3;
  Jim_Obj *namespace = Jim_NewIntObj(interp, 42); Jim_IncrRefCount(namespace);
  Jim_Obj *argument = Jim_NewIntObj(interp, 84); Jim_IncrRefCount(argument);
  Jim_Obj *elements[] = { Jim_NewStringObj(interp, test->formal, -1), Jim_NewStringObj(interp, test->body, -1), namespace };
  Jim_Obj *lambda = Jim_NewListObj(interp, elements, 3); Jim_IncrRefCount(lambda);
  Jim_Obj *args[] = { Jim_NewStringObj(interp, "apply", -1), lambda, argument }; Jim_IncrRefCount(args[0]);
  sample("NAMESPACE_BEFORE", test->label, namespace); sample("ARGUMENT_BEFORE", test->label, argument);
  code = Jim_EvalObjVector(interp, 3, args);
  sample("NAMESPACE_AFTER", test->label, namespace); sample("ARGUMENT_AFTER", test->label, argument);
  Jim_Obj *result = Jim_GetResult(interp);
  Jim_Obj *selected = test->level && code == JIM_OK ? Jim_ListGetIndex(interp, result, 1) : result;
  printf("RESULT_IDENTITY|%s|0|%s\n", test->label, selected == argument ? "31" : "30");
  row("RESULT", test->label, code, result); row("NAMESPACE_GETTER", test->label, 0, namespace); row("ARGUMENT_GETTER", test->label, 0, argument);
  Jim_DecrRefCount(interp, args[0]); Jim_DecrRefCount(interp, lambda); Jim_DecrRefCount(interp, namespace); Jim_DecrRefCount(interp, argument); Jim_FreeInterp(interp);
 }
 return 0;
}
