#include <stdio.h>
#include <string.h>
#ifdef JIM_PROBE
#include "jim.h"
int main(void) {
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    if (Jim_InitStaticExtensions(interp) != JIM_OK) return 2;
    int code = Jim_Eval(interp, "info patchlevel");
    int length;
    const char *bytes = Jim_GetString(Jim_GetResult(interp), &length);
    printf("VERSION|%d|", code);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
    puts("");
    code = Jim_Eval(interp, "info commands ::oo::class");
    bytes = Jim_GetString(Jim_GetResult(interp), &length);
    printf("OO_COMMAND|%d|", code);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
    puts("");
    puts("OO_BOOTSTRAP|0|UNAVAILABLE");
    Jim_FreeInterp(interp);
    return 0;
}
#else
#include "tcl.h"
#if TCL_MAJOR_VERSION > 8 || (TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION >= 6)
#include "tclInt.h"
#include "tclOOInt.h"
static void hex(const char *bytes, long length) {
    for (long i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
}
static void text(const char *label, const char *bytes) {
    printf("%s|0|", label);
    if (bytes) hex(bytes, (long)strlen(bytes));
    puts("");
}
static Object *lookup(Tcl_Interp *interp, const char *name) {
    Tcl_Obj *original = Tcl_NewStringObj(name, -1);
    Tcl_IncrRefCount(original);
    Object *object = (Object *)Tcl_GetObjectFromObj(interp, original);
    Tcl_DecrRefCount(original);
    return object;
}
static const char *method_type(Method *method) {
    if (!method) return "ABSENT";
#if TCL_MAJOR_VERSION >= 9
    return method->type2Ptr ? method->type2Ptr->name : "FLAGS_ONLY";
#else
    return method->typePtr ? method->typePtr->name : "FLAGS_ONLY";
#endif
}
static void method_state(const char *label, const char *side, Tcl_HashTable *table) {
    Tcl_HashSearch search;
    for (Tcl_HashEntry *entry = Tcl_FirstHashEntry(table, &search); entry; entry = Tcl_NextHashEntry(&search)) {
        Tcl_Obj *name = (Tcl_Obj *)Tcl_GetHashKey(table, entry);
        Method *method = (Method *)Tcl_GetHashValue(entry);
#if TCL_MAJOR_VERSION >= 9
        Tcl_Size length;
#else
        int length;
#endif
        const char *bytes = Tcl_GetStringFromObj(name, &length);
        printf("%s_METHOD|0|%s|", label, side);
        hex(bytes, (long)length);
        putchar('|');
        const char *type = method_type(method);
        hex(type, (long)strlen(type));
        printf("|flags=%ld\n", (long)method->flags);
    }
}
static void namespace_state(const char *label, Tcl_Namespace *publicNamespace) {
    Namespace *ns = (Namespace *)publicNamespace;
    printf("%s|0|", label);
    hex(ns->fullName, (long)strlen(ns->fullName));
    printf("|%ld", (long)ns->commandPathLength);
    for (long i = 0; i < ns->commandPathLength; i++) {
        putchar('|');
        Namespace *path = ns->commandPathArray[i].nsPtr;
        if (path) hex(path->fullName, (long)strlen(path->fullName));
        else printf("MISSING");
    }
    puts("");
}
static void snapshot(Tcl_Interp *interp, const char *label, const char *name) {
    Object *object = lookup(interp, name);
    if (!object) {
        printf("%s|1|MISSING\n", label);
        Tcl_ResetResult(interp);
        return;
    }
    Tcl_CmdInfo commandInfo;
    int info = Tcl_GetCommandInfoFromToken(object->command, &commandInfo);
    Class *cls = object->classPtr;
    Command *nativeCommand = (Command *)object->command;
#if TCL_MAJOR_VERSION > 9 || (TCL_MAJOR_VERSION == 9 && TCL_MINOR_VERSION >= 1)
    int nativeClientSame = nativeCommand->objClientData2 == object;
#else
    int nativeClientSame = nativeCommand->objClientData == object;
#endif
    printf("%s_NATIVE_CLIENT|0|same_object=%d\n", label, nativeClientSame);
    if (cls) method_state(label, "CLASS", &cls->classMethods);
    if (object->methodsPtr) method_state(label, "OBJECT", object->methodsPtr);

    printf("%s|0|class=%d|command_client_object=%d|public_class_same=%d|public_namespace_same=%d|foundation_object=%d|foundation_class=%d|self_class_root=%d|object_mixins=%ld|object_filters=%ld|class_mixins=%ld|class_filters=%ld|superclasses=%ld|super_object=%d|super_class=%d|epoch=%ld|creation=%ld\n",
        label, cls != NULL, info && commandInfo.objClientData == object,
        Tcl_GetObjectAsClass((Tcl_Object)object) == (Tcl_Class)cls,
        Tcl_GetObjectNamespace((Tcl_Object)object) == object->namespacePtr,
        cls == object->fPtr->objectCls, cls == object->fPtr->classCls,
        object->selfCls == object->fPtr->classCls,
        (long)object->mixins.num, (long)object->filters.num,
        cls ? (long)cls->mixins.num : -1, cls ? (long)cls->filters.num : -1,
        cls ? (long)cls->superclasses.num : -1,
        cls && cls->superclasses.num == 1 && cls->superclasses.list[0] == object->fPtr->objectCls,
        cls && cls->superclasses.num == 1 && cls->superclasses.list[0] == object->fPtr->classCls,
        (long)object->epoch, (long)object->creationEpoch);
    char sublabel[160];
    snprintf(sublabel, sizeof(sublabel), "%s_CONSTRUCTOR_TYPE", label);
    text(sublabel, cls ? method_type(cls->constructorPtr) : "NO_CLASS");
    snprintf(sublabel, sizeof(sublabel), "%s_DESTRUCTOR_TYPE", label);
    text(sublabel, cls ? method_type(cls->destructorPtr) : "NO_CLASS");
    snprintf(sublabel, sizeof(sublabel), "%s_PRIVATE_NAMESPACE", label);
    namespace_state(sublabel, object->namespacePtr);
#if TCL_MAJOR_VERSION >= 9
    snprintf(sublabel, sizeof(sublabel), "%s_CLASS_DEFINITION_NAMESPACE", label);
    text(sublabel, cls && cls->clsDefinitionNs ? Tcl_GetString(cls->clsDefinitionNs) : "ABSENT");
    snprintf(sublabel, sizeof(sublabel), "%s_OBJECT_DEFINITION_NAMESPACE", label);
    text(sublabel, cls && cls->objDefinitionNs ? Tcl_GetString(cls->objDefinitionNs) : "ABSENT");
#endif
}
static int run(Tcl_Interp *interp, const char *label, const char *source) {
    int code = Tcl_EvalEx(interp, source, -1, 0);
#if TCL_MAJOR_VERSION >= 9
    Tcl_Size length;
#else
    int length;
#endif
    const char *bytes = Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
    printf("%s|%d|", label, code);
    hex(bytes, (long)length);
    puts("");
    return code;
}
#endif
int main(int argc, char **argv) {
    (void)argc;
    Tcl_FindExecutable(argv[0]);
    Tcl_Interp *interp = Tcl_CreateInterp();
    if (Tcl_Init(interp) != TCL_OK) return 2;
#if TCL_MAJOR_VERSION > 8 || (TCL_MAJOR_VERSION == 8 && TCL_MINOR_VERSION >= 6)
    run(interp, "VERSION", "info patchlevel");
    snapshot(interp, "OBJECT_ROOT", "::oo::object");
    snapshot(interp, "CLASS_ROOT", "::oo::class");
    if (run(interp, "CLASS_CREATE_OVERRIDE", "oo::objdefine ::oo::class method create args {return OVERRIDE}; ::oo::class create C")) return 7;
    snapshot(interp, "CLASS_ROOT_WITH_OVERRIDE", "::oo::class");
    if (run(interp, "CLASS_CREATE_OVERRIDE_REMOVAL", "oo::objdefine ::oo::class deletemethod create")) return 8;

#if TCL_MAJOR_VERSION >= 9
    snapshot(interp, "CONFIGURABLE", "::oo::configurable");
    snapshot(interp, "SUPPORT", "::oo::configuresupport::configurable");
    Tcl_Namespace *clsNs = Tcl_FindNamespace(interp, "::oo::configuresupport::configurableclass", NULL, TCL_GLOBAL_ONLY);
    Tcl_Namespace *objNs = Tcl_FindNamespace(interp, "::oo::configuresupport::configurableobject", NULL, TCL_GLOBAL_ONLY);
    if (!clsNs || !objNs) return 3;
    namespace_state("SUPPORT_CLASS_WORKERS", clsNs);
    namespace_state("SUPPORT_OBJECT_WORKERS", objNs);
    if (run(interp, "RENAME_SUPPORT", "rename ::oo::configuresupport::configurable ::HeldSupport")) return 4;
    snapshot(interp, "MOVED_SUPPORT", "::HeldSupport");
    if (run(interp, "ALTER_SUPPORT_LIFECYCLE", "oo::define ::HeldSupport {constructor {} {return}; destructor {return}}")) return 5;
    snapshot(interp, "LIFECYCLE_CHANGED_SUPPORT", "::HeldSupport");
    if (run(interp, "ALTER_SUPPORT_DISPATCH", "oo::class create ::ProbeMixin {}; oo::define ::HeldSupport {mixin ::ProbeMixin; method observe args {next {*}$args}; filter observe}")) return 6;
    snapshot(interp, "DISPATCH_CHANGED_SUPPORT", "::HeldSupport");
#else
    puts("CONFIGURABLE|0|UNAVAILABLE");
#endif
#else
    int code = Tcl_EvalEx(interp, "info patchlevel", -1, 0);
    int length;
    const char *bytes = Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
    printf("VERSION|%d|", code);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
    puts("");
    code = Tcl_EvalEx(interp, "info commands ::oo::class", -1, 0);
    bytes = Tcl_GetStringFromObj(Tcl_GetObjResult(interp), &length);
    printf("OO_COMMAND|%d|", code);
    for (int i = 0; i < length; i++) printf("%02x", (unsigned char)bytes[i]);
    puts("");
    puts("OO_BOOTSTRAP|0|UNAVAILABLE");
#endif
    Tcl_DeleteInterp(interp);
    Tcl_Finalize();
    return 0;
}
#endif
