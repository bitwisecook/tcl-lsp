/* Pinned Jim scalar GetDouble parsing and exact cache discriminator. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "jim.h"
int main(void) {
    const char *cases[] = {
        "0755", "0x10", "0x1p4", "0x1.8p2", "0b10", "0o10", "0d10",
        "nan(foo)", "NaN(0x1)", "1e400", "1e-400", "18446744073709551615",
        "18446744073709551616", "-18446744073709551615", "-18446744073709551616",
        "9223372036854775808", " 1 ", "1_0", "true", "0x1.fffffffffffffp1023",
        "0x1p-1074", "0x1p-1075", "0x1.8p-1075", "-0", "-0.0", "0x1."
    };
    unsigned i;
    for (i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
        Jim_Interp *interp = Jim_CreateInterp();
        Jim_Obj *input = Jim_NewStringObj(interp, cases[i], -1);
        double result = 0;
        union { double d; uint64_t bits; } value;
        int code;
        Jim_IncrRefCount(input);
        code = Jim_GetDouble(interp, input, &result);
        value.d = result;
        printf("%s\tcode=%d\ttype=%s\tbits=%016" PRIx64,
            cases[i], code, input->typePtr ? input->typePtr->name : "string", value.bits);
        if (code == JIM_OK && input->typePtr && strcmp(input->typePtr->name, "coerced-double") == 0)
            printf("\twide=%" PRId64, (int64_t)input->internalRep.wideValue);
        putchar('\n');
        Jim_DecrRefCount(interp, input);
        Jim_FreeInterp(interp);
    }
    return 0;
}
