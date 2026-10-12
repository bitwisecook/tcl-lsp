#include <stdio.h>
#include <limits.h>
#include <string.h>
#include <stdlib.h>
#ifdef USE_JIM
#include "jim.h"
typedef Jim_Obj Obj; typedef Jim_Interp ProbeInterp; typedef int Len;
#define NEW(i,s,n) Jim_NewStringObj(i,s,n)
#define INC(o) Jim_IncrRefCount(o)
#define DEC(i,o) Jim_DecrRefCount(i,o)
#define GET(o,n) Jim_GetString(o,n)
#define RESULT(i) Jim_GetResult(i)
#define LIST(i,n,v) Jim_NewListObj(i,v,n)
#define VAR(i,n) Jim_GetVariableStr(i,n,0)
#define EVAL(i,n,v) Jim_EvalObjVector(i,n,v)
#define SCRIPT(i,o) Jim_EvalObj(i,o)
static ProbeInterp *create(void){ProbeInterp *i=Jim_CreateInterp(); Jim_RegisterCoreCommands(i); if(Jim_InitStaticExtensions(i)!=JIM_OK){fprintf(stderr,"Jim static extensions failed: %s\n",Jim_String(Jim_GetResult(i)));exit(2);}return i;}
#define DESTROY(i) Jim_FreeInterp(i)
#else
#include "tcl.h"
#include "tclInt.h"
typedef Tcl_Obj Obj; typedef Tcl_Interp ProbeInterp;
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Len;
#else
typedef int Len;
#endif
#define NEW(i,s,n) Tcl_NewStringObj(s,n)
#define INC(o) Tcl_IncrRefCount(o)
#define DEC(i,o) Tcl_DecrRefCount(o)
#define GET(o,n) Tcl_GetStringFromObj(o,n)
#define RESULT(i) Tcl_GetObjResult(i)
#define LIST(i,n,v) Tcl_NewListObj(n,v)
#define VAR(i,n) Tcl_GetVar2Ex(i,n,NULL,TCL_GLOBAL_ONLY)
#define EVAL(i,n,v) Tcl_EvalObjv(i,n,v,TCL_EVAL_GLOBAL)
#define SCRIPT(i,o) Tcl_EvalObjEx(i,o,TCL_EVAL_GLOBAL)
static ProbeInterp *create(void){return Tcl_CreateInterp();}
#define DESTROY(i) Tcl_DeleteInterp(i)
#endif

static ProbeInterp *active; static const char *caseid;
static Obj *word(const char*s){return NEW(active,s,-1);}
static void hex(Obj*o){Len n;const unsigned char*s=(const unsigned char*)GET(o,&n);putchar('"');for(Len k=0;k<n;k++)printf("%02x",s[k]);putchar('"');}
static void report(const char*op,int c){Obj*r=RESULT(active);INC(r);printf("{\"case\":\"%s\",\"op\":\"%s\",\"code\":%d,\"bytes\":",caseid,op,c);hex(r);puts("}");DEC(active,r);}
static int call(const char*op,int n,Obj**v,int observe){for(int j=0;j<n;j++)INC(v[j]);int c=EVAL(active,n,v);if(observe)report(op,c);for(int j=0;j<n;j++)DEC(active,v[j]);return c;}
static void script(const char*op,const char*s){Obj*v=word(s);INC(v);int c=SCRIPT(active,v);report(op,c);DEC(active,v);}
static Obj*key(int k,int opaque){if(opaque&&k==0)return NEW(active,"k\0x",3);if(opaque&&k==1)return NEW(active,"k\xff",2);if(opaque&&k==2)return NEW(active,"k\xc0\x80",3);char b[30];sprintf(b,"k%02d",k);return word(b);}
static void setelement(int k,int opaque){Obj*v[]={word("array"),word("set"),word("a"),NULL};Obj*pair[]={key(k,opaque),word("V")};v[3]=LIST(active,2,pair);call("set",4,v,0);}
static void globalkeys(void){printf("{\"case\":\"%s\",\"op\":\"physical-global-keys\",\"keys\":[",caseid);int first=1;
#ifdef USE_JIM
 Jim_HashTable*t=&active->topFramePtr->vars;for(unsigned k=0;k<t->size;k++)for(Jim_HashEntry*h=t->table[k];h;h=h->next){if(!first)putchar(',');first=0;hex((Obj*)h->key);}
#else
 Namespace*n=(Namespace*)Tcl_GetGlobalNamespace(active);Tcl_HashTable*t;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 t=&n->varTable;
#else
 t=&n->varTable.table;
#endif
 Tcl_HashSearch search;for(Tcl_HashEntry*h=Tcl_FirstHashEntry(t,&search);h;h=Tcl_NextHashEntry(&search)){if(!first)putchar(',');first=0;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 const unsigned char*z=(const unsigned char*)Tcl_GetHashKey(t,h);putchar('"');for(;*z;z++)printf("%02x",*z);putchar('"');
#else
 hex(h->key.objPtr);
#endif
 }
#endif
 puts("]}");}
static void tablemeta(const char*op){
#ifdef USE_JIM
 Jim_HashTable*t=&active->topFramePtr->vars;printf("{\"case\":\"%s\",\"op\":\"%s\",\"global_buckets\":%u,\"global_entries\":%u,\"global_hash_uniq\":%u}\n",caseid,op,t->size,t->used,t->uniq);
#else
 Namespace*n=(Namespace*)Tcl_GetGlobalNamespace(active);Tcl_HashTable*t;
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 t=&n->varTable;
#else
 t=&n->varTable.table;
#endif
 Var*v=(Var*)Tcl_FindNamespaceVar(active,"a",NULL,TCL_GLOBAL_ONLY);Tcl_HashTable*e=NULL;
 if(v&&TclIsVarArray(v)){
#if TCL_MAJOR_VERSION==8 && TCL_MINOR_VERSION==4
 e=v->value.tablePtr;
#else
 e=&v->value.tablePtr->table;
#endif
 }
 printf("{\"case\":\"%s\",\"op\":\"%s\",\"global_buckets\":%ld,\"global_entries\":%ld,\"array_buckets\":%ld,\"array_entries\":%ld}\n",caseid,op,(long)t->numBuckets,(long)t->numEntries,e?(long)e->numBuckets:-1,e?(long)e->numEntries:-1);
#endif
}
static void enumerate(void){tablemeta("physical-table");script("array-names","array names a");script("array-get","array get a");script("array-search","set result {};set id [array startsearch a];while {[array anymore a $id]} {lappend result [array nextelement a $id]};array donesearch a $id;set result");}
int main(void){printf("{\"case\":\"actual-native-abi\",\"op\":\"abi\",\"plain_char_signed\":%d,\"unsigned_int_bits\":%d,\"size_t_bits\":%d}\n",((char)255)<0,(int)(sizeof(unsigned int)*CHAR_BIT),(int)(sizeof(size_t)*CHAR_BIT));int sizes[]={3,11,12,13,15,16,17,47,48,49,191,192,193,767,768,769};char label[100];
for(int opaque=0;opaque<2;opaque++)for(int x=0;x<16;x++){int n=sizes[x];sprintf(label,"array-%d-opaque%d",n,opaque);caseid=label;active=create();for(int k=0;k<n;k++)setelement(k,opaque);enumerate();Obj*pair[]={word("unset"),word("a(k03)")};call("delete",2,pair,0);setelement(3,opaque);enumerate();DESTROY(active);}
for(int opaque=0;opaque<2;opaque++){sprintf(label,"variables-opaque%d",opaque);caseid=label;active=create();tablemeta("initial-physical-table");globalkeys();script("initial-info-globals","info globals");Obj*keys[15];for(int k=0;k<15;k++){keys[k]=key(k,opaque);INC(keys[k]);Obj*v[]={word("set"),keys[k],word("V")};call("set",3,v,0);}tablemeta("physical-table");script("info-globals","info globals k*");script("info-vars","info vars k*");script("proc-define","proc p {names} {foreach name $names {set $name V};list [info locals k*] [info vars k*]}");Obj*v[]={word("p"),LIST(active,15,keys)};call("proc-local-inventory",2,v,1);for(int k=0;k<15;k++)DEC(active,keys[k]);DESTROY(active);}
caseid="array-undefined-trace-shell";active=create();script("setup","proc noop {args} {};array set a {k00 V k01 V k02 V};if {[catch {trace add variable a(k03) read noop}]} {trace variable a(k03) r noop};if {[catch {trace add variable a(k04) read noop}]} {trace variable a(k04) r noop};if {[catch {trace add variable a(k05) read noop}]} {trace variable a(k05) r noop};if {[catch {trace add variable a(k06) read noop}]} {trace variable a(k06) r noop};if {[catch {trace add variable a(k07) read noop}]} {trace variable a(k07) r noop};if {[catch {trace add variable a(k08) read noop}]} {trace variable a(k08) r noop};if {[catch {trace add variable a(k09) read noop}]} {trace variable a(k09) r noop};if {[catch {trace add variable a(k10) read noop}]} {trace variable a(k10) r noop};if {[catch {trace add variable a(k11) read noop}]} {trace variable a(k11) r noop}");enumerate();script("fill-shell","set a(k03) V");enumerate();script("remove-shell","if {[catch {trace remove variable a(k04) read noop}]} {trace vdelete a(k04) r noop}");enumerate();DESTROY(active);
caseid="array-teardown";active=create();script("teardown","set log {};proc cb {n k op} {global log;lappend log $k};array set a {k00 V k01 V k02 V k03 V k04 V k05 V k06 V k07 V k08 V k09 V k10 V k11 V k12 V};foreach k [array names a] {if {[catch {trace add variable a($k) unset cb}]} {trace variable a($k) u cb}};unset a;set log");DESTROY(active);
caseid="variable-undefined-trace-shell";active=create();script("setup","proc noop {args} {};set k00 V;set k01 V;set k02 V;foreach key {k03 k04 k05 k06 k07 k08 k09 k10 k11} {if {[catch {trace add variable $key read noop}]} {trace variable $key r noop}}");tablemeta("physical-table");script("info-globals","info globals k*");script("info-vars","info vars k*");script("fill-shell","set k03 V");script("info-vars","info vars k*");script("remove-shell","if {[catch {trace remove variable k04 read noop}]} {trace vdelete k04 r noop}");script("info-vars","info vars k*");DESTROY(active);
caseid="dynamic-local-teardown";active=create();script("teardown","set log {};proc cb {n k op} {global log;lappend log $n};proc p {} {foreach key {k00 k01 k02 k03 k04 k05 k06 k07 k08 k09 k10 k11 k12} {set $key V;if {[catch {trace add variable $key unset cb}]} {trace variable $key u cb}}};p;set log");DESTROY(active);return 0;}
