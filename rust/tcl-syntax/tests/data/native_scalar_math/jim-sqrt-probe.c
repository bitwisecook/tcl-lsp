/* Jim 0.84's exact native scalar math result and operand cache protocol. */
#include <stdio.h>
#include "jim.h"

static const char *type_name(Jim_Obj *value) {
    return value->typePtr ? value->typePtr->name : "string";
}

int main(void) {
    const char *labels[] = {"integer", "double", "string", "bad-string", "negative"};
    int i;
    for (i = 0; i < 5; ++i) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_Obj *input;
        const char *before;
        int code;
        Jim_RegisterCoreCommands(interp);
        switch (i) {
        case 0: input = Jim_NewIntObj(interp, 4); break;
        case 1: input = Jim_NewDoubleObj(interp, 4.0); break;
        case 2: input = Jim_NewStringObj(interp, "4", -1); break;
        case 3: input = Jim_NewStringObj(interp, "not-number", -1); break;
        default: input = Jim_NewIntObj(interp, -1); break;
        }
        Jim_IncrRefCount(input);
        Jim_SetVariableStr(interp, "x", input);
        before = type_name(input);
        code = Jim_Eval(interp, "expr {sqrt($x)}");
        printf("%s code=%d before=%s after=%s result=%s\n",
            labels[i], code, before, type_name(input), type_name(Jim_GetResult(interp)));
        Jim_DecrRefCount(interp, input);
        Jim_FreeInterp(interp);
    }
    return 0;
}
