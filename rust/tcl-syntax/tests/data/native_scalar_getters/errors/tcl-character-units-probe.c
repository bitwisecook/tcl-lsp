/* Pinned canonical C UTF decoder, isolated unit encoder and UtfPrev owner. */
#include <stdio.h>
#include "tcl.h"
static const char *cases[]={"A","\000","\200x","\237x","\300\200","\300\201","\301\277","\340\200\200","\355\240\200","\360\237\230\200","\364\217\277\277","\364\220\200\200","\342\202","A\200\200"};
static const int lengths[]={1,1,2,2,2,2,2,3,3,4,4,4,2,3};
static void hex(const char *bytes,int length){int i;for(i=0;i<length;i++)printf("%02x",(unsigned char)bytes[i]);}
int main(int argc,char **argv){int at;(void)argc;Tcl_FindExecutable(argv[0]);
for(at=0;at<14;at++){int pos=0,i,first=1;Tcl_UniChar unit=0;char encoded[80],*out=encoded;
printf("case=%d input=",at);hex(cases[at],lengths[at]);printf(" units=");
while(pos<lengths[at]){int width=Tcl_UtfToUniChar(cases[at]+pos,&unit);char isolated[8]={0};int wrote=Tcl_UniCharToUtf((int)unit,isolated);
if(!first)putchar(',');first=0;printf("%x:%d",(unsigned)unit,width);for(i=0;i<wrote;i++)*out++=isolated[i];pos+=width;}
printf(" encoded=");hex(encoded,(int)(out-encoded));printf(" boundaries=");
for(i=1;i<=lengths[at];i++){if(i!=1)putchar(',');printf("%d",(int)(Tcl_UtfPrev(cases[at]+i,cases[at])-cases[at]));}
putchar('\n');}Tcl_Finalize();return 0;}
