/* Public C Tcl source ingress: actual ReadChars versus counted EvalEx. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void hex(const char *s, Len n) {
    putchar('"');
    for (Len j = 0; j < n; ++j) printf("%02x", (unsigned char)s[j]);
    putchar('"');
}
static void object(const char *path, const char *operation, Tcl_Obj *value, int code) {
    const char *type = value->typePtr ? value->typePtr->name : "none";
    int resident = value->bytes != NULL;
    Len length = 0;
    const char *bytes = Tcl_GetStringFromObj(value, &length);
    printf("{\"path\":\"%s\",\"op\":\"%s\",\"code\":%d,\"type_before_get_string\":\"%s\",\"resident_before_get_string\":%s,\"length\":%lld,\"bytes\":", path, operation, code, type, resident ? "true" : "false", (long long)length);
    hex(bytes, length);
    puts("}");
}
static void evaluate(const char *path, const char *source, Len length) {
    Tcl_Interp *interp = Tcl_CreateInterp();
    int code = Tcl_EvalEx(interp, source, length, TCL_EVAL_GLOBAL);
    object(path, "eval-result", Tcl_GetObjResult(interp), code);
    const char *variables[] = {"plain", "braced", "escaped", "numeric", "slashzero"};
    for (int j = 0; j < 5; ++j) {
        Tcl_Obj *value = Tcl_GetVar2Ex(interp, variables[j], NULL, TCL_GLOBAL_ONLY);
        if (value != NULL) object(path, variables[j], value, TCL_OK);
        else printf("{\"path\":\"%s\",\"op\":\"%s\",\"missing\":true}\n", path, variables[j]);
    }
    Tcl_DeleteInterp(interp);
}
int main(int argc, char **argv) {
    if (argc != 2) { fputs("usage: probe source-file\n", stderr); return 2; }
    Tcl_FindExecutable(argv[0]);
    FILE *file = fopen(argv[1], "rb");
    if (!file) { perror("fopen"); return 2; }
    if (fseek(file, 0, SEEK_END) != 0) return 2;
    long size = ftell(file);
    if (size < 0 || fseek(file, 0, SEEK_SET) != 0) return 2;
    char *raw = malloc((size_t)size + 1);
    if (!raw || fread(raw, 1, (size_t)size, file) != (size_t)size) return 2;
    fclose(file); raw[size] = 0;
    Tcl_Interp *interp = Tcl_CreateInterp();
    int code = Tcl_EvalEx(interp, "info patchlevel", -1, TCL_EVAL_GLOBAL);
    object("startup", "patchlevel", Tcl_GetObjResult(interp), code);
    evaluate("counted-native-source", raw, (Len)size);
    Tcl_Channel channel = Tcl_OpenFileChannel(interp, argv[1], "r", 0);
    if (!channel) { fputs("OpenFileChannel failed\n", stderr); return 2; }
    if (Tcl_SetChannelOption(interp, channel, "-encoding", "utf-8") != TCL_OK ||
        Tcl_SetChannelOption(interp, channel, "-translation", "auto") != TCL_OK) {
        fputs("channel configuration failed\n", stderr); return 2;
    }
    Tcl_DString options; Tcl_DStringInit(&options);
    code = Tcl_GetChannelOption(interp, channel, "-encoding", &options);
    printf("{\"path\":\"character-channel\",\"op\":\"encoding\",\"code\":%d,\"bytes\":", code);
    hex(Tcl_DStringValue(&options), (Len)Tcl_DStringLength(&options)); puts("}");
    Tcl_DStringFree(&options); Tcl_DStringInit(&options);
    code = Tcl_GetChannelOption(interp, channel, "-translation", &options);
    printf("{\"path\":\"character-channel\",\"op\":\"translation\",\"code\":%d,\"bytes\":", code);
    hex(Tcl_DStringValue(&options), (Len)Tcl_DStringLength(&options)); puts("}");
    Tcl_DStringFree(&options);
    Tcl_Obj *read = Tcl_NewObj(); Tcl_IncrRefCount(read);
    Len count = Tcl_ReadChars(channel, read, -1, 0);
    printf("{\"path\":\"character-channel\",\"op\":\"read-count\",\"count\":%lld}\n", (long long)count);
    if (count < 0) { fputs("ReadChars failed\n", stderr); return 2; }
    object("character-channel", "source", read, TCL_OK);
    Len length; const char *bytes = Tcl_GetStringFromObj(read, &length);
    evaluate("character-channel-source", bytes, length);
    Tcl_DecrRefCount(read);
    if (Tcl_Close(interp, channel) != TCL_OK) return 2;
    Tcl_DeleteInterp(interp); free(raw); return 0;
}
