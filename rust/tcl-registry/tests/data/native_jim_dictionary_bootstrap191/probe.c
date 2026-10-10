#include "jim.h"
#include <stdio.h>

extern int Jim_stdlibInit(Jim_Interp *interp);

static void report(Jim_Interp *interp, const char *label, const char *source)
{
    int length;
    int code = Jim_Eval(interp, source);
    const unsigned char *bytes = (const unsigned char *)Jim_GetString(Jim_GetResult(interp), &length);
    printf("RESULT %s %d ", label, code);
    for (int index = 0; index < length; ++index) {
        printf("%02x", bytes[index]);
    }
    printf("\n");
}

int main(void)
{
    Jim_Interp *interp = Jim_CreateInterp();
    Jim_RegisterCoreCommands(interp);
    report(interp, "core",
        "set d {first NEW};set ok BEFORE;set entered 0;"
        "set c [catch {dict update d first ok {set entered 1}} r];"
        "list [info commands {dict update}] $c $r $ok $entered $d");
    int code = Jim_stdlibInit(interp);
    printf("INITIALIZE stdlib %d\n", code);
    report(interp, "stdlib",
        "set d {first NEW};set ok BEFORE;set entered 0;"
        "set c [catch {dict update d first ok {set entered 1}} r];"
        "list [info commands {dict update}] $c $r $ok $entered $d");
    Jim_FreeInterp(interp);
    return 0;
}
