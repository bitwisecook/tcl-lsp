#include "tclInt.h"
#include <stdio.h>
#include <string.h>
#define COUNT int
static Tcl_Interp *active;
static int observed = -1;
static int present(void) { return ((Interp *)active)->ensembleRewrite.sourceObjs != NULL; }
static void update(Tcl_Obj *object) {
 observed = present();
#if TCL_MAJOR_VERSION >= 9
 Tcl_InitStringRep(object, "noop", 4);
#else
 object->bytes = Tcl_Alloc(5); memcpy(object->bytes, "noop", 5); object->length = 4;
#endif
}
static Tcl_ObjType type = {"lookupProbe", NULL, NULL, update, NULL};
static int noop(ClientData data, Tcl_Interp *interp, COUNT count, Tcl_Obj *const words[]) {
 (void)data; (void)interp; (void)count; (void)words; return TCL_OK;
}
static int worker(ClientData data, Tcl_Interp *interp, COUNT count, Tcl_Obj *const words[]) {
 (void)data; (void)count;
 active = interp; int before = present(); observed = -1; int code;
 const char *mode = Tcl_GetString(words[1]);
 if (!strcmp(mode, "lookup")) {
  Tcl_Obj *head = Tcl_NewObj(); Tcl_IncrRefCount(head);
  Tcl_InvalidateStringRep(head); head->typePtr = &type;
  code = Tcl_EvalObjv(interp, 1, &head, 0); Tcl_DecrRefCount(head);
 } else if (!strcmp(mode, "bytecode")) {
  Tcl_Obj *body = Tcl_NewStringObj("set x 1", -1); Tcl_IncrRefCount(body);
  code = Tcl_EvalObjEx(interp, body, 0); Tcl_DecrRefCount(body);
 } else {
  Tcl_Obj *head = Tcl_NewStringObj(!strcmp(mode, "invoke") ? "noop" : "notThere", -1);
  Tcl_IncrRefCount(head); code = Tcl_EvalObjv(interp, 1, &head, !strcmp(mode, "invoke") ? TCL_EVAL_INVOKE : 0);
  Tcl_DecrRefCount(head);
 }
 char value[100]; snprintf(value, sizeof value, "before=%d lookup=%d after=%d code=%d", before, observed, present(), code);
 Tcl_SetObjResult(interp, Tcl_NewStringObj(value, -1)); return TCL_OK;
}
int main(int argc, char **argv) {
 (void)argc; Tcl_FindExecutable(argv[0]); Tcl_Interp *interp = Tcl_CreateInterp();
 if (Tcl_Init(interp) != TCL_OK) { fprintf(stderr, "%s\n", Tcl_GetStringResult(interp)); return 1; }
 Tcl_CreateObjCommand(interp, "worker", worker, NULL, NULL); Tcl_CreateObjCommand(interp, "noop", noop, NULL, NULL);
 if (Tcl_Eval(interp, "namespace ensemble create -command ens -map {run worker}") != TCL_OK) return 2;
 Tcl_DeleteCommand(interp, "unknown");
 const char *modes[] = {"lookup", "bytecode", "invoke", "missing"};
 for (int k=0; k<4; k++) {
  char source[100]; snprintf(source, sizeof source, "ens run %s", modes[k]);
  int code = Tcl_Eval(interp, source);
  printf("%s\t%s\t%d\n", modes[k], Tcl_GetStringResult(interp), code);
 }
 Tcl_DeleteInterp(interp); Tcl_Finalize(); return 0;
}
