/* SPDX-License-Identifier: AGPL-3.0-or-later */
#include "jim.h"
#include <stdio.h>
#include <string.h>
static const char *kind(Jim_Obj *o) { return o->typePtr ? o->typePtr->name : "NULL"; }
static void hex(const char *p,int n) {int j;for(j=0;j<n;j++)printf("%02x",(unsigned char)p[j]);}
static void state(Jim_Obj *o) {printf("%s,%d,%d,%d,%d,",kind(o),o->bytes!=NULL,Jim_IsList(o),o->refCount,o->bytes?o->length:-1);if(o->bytes)hex(o->bytes,o->length);}
static Jim_Obj *list(Jim_Interp *ip,const char *a,const char *b,Jim_Obj **child) {Jim_Obj *v[2];int n=0;if(a){v[n++]=Jim_NewStringObj(ip,a,-1);if(child)*child=v[0];}if(b)v[n++]=Jim_NewStringObj(ip,b,-1);return Jim_NewListObj(ip,v,n);}
static Jim_Obj *parsed(Jim_Interp *ip,const char *s) {Jim_Obj *o=Jim_NewStringObj(ip,s,-1);Jim_ListLength(ip,o);return o;}
static void direct(Jim_Interp *ip,int id) {
 Jim_Obj *v[3],*c[3]={NULL,NULL,NULL},*r; int n=0,j; const char *render; int length;
 switch(id) {
 case 0: break;
 case 1: v[n++]=Jim_NewStringObj(ip,"",0); break;
 case 2: v[n++]=Jim_NewStringObj(ip,"",0); break;
 case 3: v[n++]=list(ip,"a","b",&c[0]); break;
 case 4: v[n++]=list(ip,NULL,NULL,NULL); break;
 case 5: v[n++]=list(ip,"a",NULL,&c[0]); v[n++]=list(ip,"b",NULL,&c[1]); break;
 case 6: v[n++]=list(ip,"a",NULL,&c[0]); v[n++]=list(ip,"#b",NULL,&c[1]); break;
 case 7: v[n++]=list(ip,"#a",NULL,&c[0]); v[n++]=list(ip,"b",NULL,&c[1]); break;
 case 8: v[n++]=parsed(ip," a  {b} "); break;
 case 9: v[n]=list(ip,"a","b",&c[0]); Jim_String(v[n++]); break;
 case 10: v[n++]=Jim_NewStringObj(ip,"",0); v[n++]=list(ip,"a",NULL,&c[1]); break;
 case 11: v[n++]=Jim_NewStringObj(ip,"  ",-1); v[n++]=list(ip,"a",NULL,&c[1]); break;
 case 12: v[n++]=list(ip,"a",NULL,&c[0]); v[n++]=list(ip,NULL,NULL,NULL); break;
 case 13: v[n++]=list(ip,NULL,NULL,NULL); v[n++]=list(ip,"#b",NULL,&c[1]); break;
 case 14: v[n++]=Jim_NewStringObj(ip," a ",-1); v[n++]=Jim_NewStringObj(ip," b ",-1); break;
 case 15: v[n++]=Jim_NewStringObj(ip,"",0); v[n++]=Jim_NewStringObj(ip,"",0); break;
 case 16: v[n++]=Jim_NewStringObj(ip," ",1); v[n++]=Jim_NewStringObj(ip,"\t",1); break;
 case 17: v[n++]=Jim_NewStringObj(ip,"a\\ ",-1); v[n++]=Jim_NewStringObj(ip,"b",-1); break;
 case 18: v[n++]=Jim_NewStringObj(ip,"a\\\\ ",-1); v[n++]=Jim_NewStringObj(ip,"b",-1); break;
 case 19: v[n++]=list(ip,"a",NULL,&c[0]); v[n++]=parsed(ip," #b "); break;
 default: return;
 }
 for(j=0;j<n;j++) Jim_IncrRefCount(v[j]);
 printf("D\t%d\tbefore\t",id); for(j=0;j<n;j++) {if(j)putchar(';');state(v[j]);} putchar('\n');
 r=Jim_ConcatObj(ip,n,v); Jim_IncrRefCount(r);
 printf("D\t%d\tresult\t",id); state(r); printf("\tsame_first=%d\tbacking_first=%d\tchildren=",n&&r==v[0],n&&r->typePtr&&v[0]->typePtr&&!strcmp(kind(r),"list")&&!strcmp(kind(v[0]),"list")&&r->internalRep.listValue.ele==v[0]->internalRep.listValue.ele);
 for(j=0;j<n;j++) {if(j)putchar(','); printf("%d",c[j]?c[j]->refCount:-1);} putchar('\n');
 printf("D\t%d\tafter\t",id); for(j=0;j<n;j++){if(j)putchar(';');state(v[j]);}putchar('\n');
 render=Jim_GetString(r,&length); printf("D\t%d\tvalue\t",id);hex(render,length);putchar('\n');
 Jim_DecrRefCount(ip,r); for(j=0;j<n;j++) Jim_DecrRefCount(ip,v[j]);
}
int main(void) {int j;Jim_Interp *ip=Jim_CreateInterp();Jim_RegisterCoreCommands(ip);for(j=0;j<20;j++)direct(ip,j);Jim_FreeInterp(ip);return 0;}
