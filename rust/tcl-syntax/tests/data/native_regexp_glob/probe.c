#include "tclInt.h"
#include <stdio.h>
#include <string.h>
int main(void) {
 const char *patterns[] = {"", "foo", "^foo$", ".*", ".+", "a.*b.*c", "\\B", "\\n", "\\*", "***=x?", "[a]", "a$b", "a\\", "^.*foo.*$"};
 Tcl_FindExecutable("regexp-glob");
 for (int n=0;n<14;n++) { Tcl_DString ds;int exact=0,quant=0; int code;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==5
 code=TclReToGlob(NULL,patterns[n],(int)strlen(patterns[n]),&ds,&exact);
#else
 code=TclReToGlob(NULL,patterns[n],strlen(patterns[n]),&ds,&exact,&quant);
#endif
 printf("%d\t%d\t",n,code); if(code==TCL_OK) {const unsigned char *b=(const unsigned char*)Tcl_DStringValue(&ds);for(int k=0;k<Tcl_DStringLength(&ds);k++)printf("%02x",b[k]);Tcl_DStringFree(&ds);} puts("");
 } Tcl_Finalize();return 0;
}
