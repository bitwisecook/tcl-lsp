
#include <stdio.h>
#include <stdint.h>
#include "tcl.h"
#if TCL_MAJOR_VERSION>=9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
static void hex(const char*s,int n){putchar('"');for(int i=0;i<n;i++)printf("%02x",(unsigned char)s[i]);putchar('"');}
static void point(int u){char buf[16];int n=Tcl_UniCharToUtf(u,buf);printf("{\"op\":\"UniCharToUtf\",\"point\":%d,\"unicode_width\":%zu,\"length\":%d,\"bytes\":",u,sizeof(Tcl_UniChar),n);hex(buf,n);puts("}");}
static void sequence(const char*name,const uint32_t*v,int n){Tcl_UniChar units[8];for(int i=0;i<n;i++){if(sizeof(Tcl_UniChar)<4&&v[i]>0xffff){printf("{\"op\":\"NewUnicodeObj\",\"case\":\"%s\",\"admitted\":false,\"unicode_width\":%zu}\n",name,sizeof(Tcl_UniChar));return;}units[i]=(Tcl_UniChar)v[i];}Tcl_Obj*o=Tcl_NewUnicodeObj(units,n);Tcl_IncrRefCount(o);printf("{\"op\":\"Unicode-before\",\"case\":\"%s\",\"admitted\":true,\"unicode_width\":%zu,\"type\":\"%s\",\"resident\":%d}\n",name,sizeof(Tcl_UniChar),o->typePtr?o->typePtr->name:"none",o->bytes!=NULL);Len len;const char*s=Tcl_GetStringFromObj(o,&len);printf("{\"op\":\"Unicode-string\",\"case\":\"%s\",\"length\":%d,\"bytes\":",name,(int)len);hex(s,(int)len);puts("}");Tcl_DecrRefCount(o);}
int main(void){setvbuf(stdout,NULL,_IONBF,0);Tcl_FindExecutable("2286-native-unicode-encoder");Tcl_Interp*interp=Tcl_CreateInterp();int points[]={0,0x80,0xff,0xd800,0xdc00,0x1f600};for(int i=0;i<6;i++)point(points[i]);uint32_t nul[]={0},hi[]={0xd800},lo[]={0xdc00},pair[]={0xd800,0xdc00},astralpair[]={0xd83d,0xde00},reverse[]={0xdc00,0xd800},scalar[]={0x1f600},mixed[]={0,0x80,0xff,0xd83d,0xde00};sequence("zero",nul,1);sequence("high",hi,1);sequence("low",lo,1);sequence("adjacent-surrogates",pair,2);sequence("astral-surrogate-pair",astralpair,2);sequence("reverse-surrogates",reverse,2);sequence("astral-scalar",scalar,1);sequence("mixed",mixed,5);Tcl_DeleteInterp(interp);return 0;}
