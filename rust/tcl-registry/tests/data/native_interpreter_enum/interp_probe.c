#include <tcl.h>
#include <stdio.h>
#include <string.h>
#if TCL_MAJOR_VERSION >= 9
typedef Tcl_Size Count;
#else
typedef int Count;
#endif
/* Exact native source IndexRep layout, not a public cache reconstruction. */
struct IndexRep { void *table; Count stride; Count index; };
static void *rootTable, *childTable;
static void hex(const char*s,Count n){Count k;for(k=0;k<n;k++)printf("%02x",(unsigned char)s[k]);}
static struct IndexRep *rep(Tcl_Obj*o){return o->typePtr && !strcmp(o->typePtr->name,"index")?(struct IndexRep*)o->internalRep.otherValuePtr:NULL;}
static void snapshot(const char*stage,Tcl_Obj*o,int code,char*before,Tcl_Interp*i,int printTable){
 struct IndexRep*r=rep(o);Count n;const char*result=Tcl_GetStringFromObj(Tcl_GetObjResult(i),&n);
 printf("%s\t%d\t%s\t%lld\t%d\t%d\t%d\t%d\t",stage,code,o->typePtr?o->typePtr->name:"none",r?(long long)r->index:-1,r? r->table==rootTable:0,r?r->table==childTable:0,o->bytes!=NULL,o->bytes==before);hex(result,n);putchar('\n');
 if(printTable&&r){int k;for(k=0;;k++){const char*word=*(const char**)((char*)r->table+k*r->stride);if(!word)break;printf("table-%s\t%d\t",stage,k);hex(word,(Count)strlen(word));putchar('\n');}}
}
static Tcl_Obj*word(const char*s,int n){Tcl_Obj*o=Tcl_NewStringObj(s,n);Tcl_IncrRefCount(o);return o;}
static void call(Tcl_Interp*i,const char*head,Tcl_Obj*o,const char*stage,int print){Tcl_Obj*h=word(head,-1);Tcl_Obj*v[]={h,o};char*before=o->bytes;int code=Tcl_EvalObjv(i,2,v,TCL_EVAL_DIRECT);struct IndexRep*r=rep(o);if(!strcmp(stage,"root-inventory")&&r)rootTable=r->table;if(!strcmp(stage,"child-inventory")&&r)childTable=r->table;snapshot(stage,o,code,before,i,print);Tcl_DecrRefCount(h);}
int main(int argc,char**argv){Tcl_FindExecutable(argv[0]);Tcl_Interp*i=Tcl_CreateInterp();Tcl_Obj*o=word("exists",-1);call(i,"interp",o,"root-inventory",1);Tcl_DecrRefCount(o);if(!Tcl_CreateSlave(i,"kid",0))return 3;o=word("issafe",-1);call(i,"kid",o,"child-inventory",1);Tcl_DecrRefCount(o);
 const char*names[]={"sl","children","s","bogus","h",NULL};int k;for(k=0;names[k];k++){o=word(names[k],-1);char stage[80];sprintf(stage,"root-%s",names[k]);call(i,"interp",o,stage,0);Tcl_DecrRefCount(o);}
 o=word("ev",-1);call(i,"interp",o,"root-ev",0);call(i,"kid",o,"child-ev-switch",0);Tcl_InvalidateStringRep(o);call(i,"kid",o,"child-ev-cached-absent",0);call(i,"interp",o,"root-ev-switch-absent",0);Tcl_DecrRefCount(o);
#if TCL_MAJOR_VERSION >= 9 && TCL_MINOR_VERSION >= 1
 if(Tcl_Init(i)!=TCL_OK){fprintf(stderr,"init: %s\n",Tcl_GetStringResult(i));return 4;}
 int setup=Tcl_Eval(i,"oo::configurable create P {property yellow -get {return Y}; property yes -get {return YES}}; P create obj");if(setup!=TCL_OK){fprintf(stderr,"property setup: %s\n",Tcl_GetStringResult(i));return 5;}
 const char*const originalTable[]={"-yellow","-yes",NULL};Count idx=-1;o=word("-yellow",-1);Tcl_GetIndexFromObj(i,o,originalTable,"property",0,&idx);struct IndexRep*old=rep(o);void*oldTable=old->table;
 Tcl_Obj*head=word("obj",-1),*configure=word("configure",-1);Tcl_Obj*v[]={head,configure,o};char*before=o->bytes;int code=Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);snapshot("property-warm-index",o,code,before,i,0);printf("property-retained-own-table\t%d\n",rep(o)&&rep(o)->table==oldTable);
 Tcl_InvalidateStringRep(o);code=Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);snapshot("property-index-absent",o,code,NULL,i,0);printf("property-absent-retained-own-table\t%d\n",rep(o)&&rep(o)->table==oldTable);Tcl_DecrRefCount(o);
 o=word("-ye",-1);v[2]=o;before=o->bytes;code=Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);snapshot("property-ambiguous-fresh",o,code,before,i,0);Tcl_DecrRefCount(o);
 o=word("-yel",-1);v[2]=o;before=o->bytes;code=Tcl_EvalObjv(i,3,v,TCL_EVAL_DIRECT);snapshot("property-prefix-fresh",o,code,before,i,0);Tcl_DecrRefCount(o);Tcl_DecrRefCount(head);Tcl_DecrRefCount(configure);
#endif
 Tcl_DeleteInterp(i);Tcl_Finalize();return 0;}
