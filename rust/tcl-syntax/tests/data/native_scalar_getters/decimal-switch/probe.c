/* Exact pinned Jim_StringToWide(..., 10) string-only switch stage. */
#include <jim.h>
#include <errno.h>
#include <stdio.h>
int main(void) {
    const char *inputs[] = {"0","1","-1","+1"," 1 \t","1.0","0x10","010",
        "9223372036854775807","9223372036854775808","18446744073709551615",
        "18446744073709551616","-9223372036854775808","1\0X","","bad"};
    for (int range=0;range<2;range++) for (int i=0;i<16;i++) {
        errno=range?ERANGE:0;
        jim_wide value=777;
        int status=Jim_StringToWide(inputs[i],&value,10);
        printf("%d\t%d\t%d\t%lld\n",i,range,status,(long long)value);
    }
    return 0;
}
