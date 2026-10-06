#include "tcl.h"
#include <stdio.h>
#include <string.h>
static const char *sources[]={"global $::name", "global ${::name}", "global $a()", "global $a(::tail)", "global $a($::name)", "global $a([foo])", "global $a(\\x61)", "global $a($::name)::tail"};
int main(int argc,char **argv){Tcl_Interp*i;Tcl_FindExecutable(argv[0]);i=Tcl_CreateInterp();
for(int n=0;n<8;n++){Tcl_Parse p;int c=Tcl_ParseCommand(i,sources[n],strlen(sources[n]),0,&p);if(c!=TCL_OK)return 2;Tcl_Token*w=p.tokenPtr+1+p.tokenPtr[0].numComponents;Tcl_Token*last=w+w->numComponents;
printf("%d|%d|%d|",n,w->numComponents,last->type);for(int x=0;x<last->size;x++)printf("%02x",(unsigned char)last->start[x]);puts("");Tcl_FreeParse(&p);}Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
