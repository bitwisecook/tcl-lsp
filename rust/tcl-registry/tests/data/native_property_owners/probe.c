#include "tclOOInt.h"
#include <stdio.h>
#include <stdlib.h>
static void eval(Tcl_Interp *ip,const char *s) {if(Tcl_Eval(ip,s)!=TCL_OK){fprintf(stderr,"%s: %s\n",s,Tcl_GetStringResult(ip));exit(2);}}
static Object *object(Tcl_Interp *ip,const char *name) {Tcl_Obj *v=Tcl_NewStringObj(name,-1);Tcl_IncrRefCount(v);Object *o=(Object *)Tcl_GetObjectFromObj(ip,v);Tcl_DecrRefCount(v);return o;}
static void row(const char *s,Tcl_Size epoch,Tcl_Size delta,int same,int refs,int child,const char *type) {printf("%s\t%ld\t%ld\t%d\t%d\t%d\t%s\n",s,(long)epoch,(long)delta,same,refs,child,type?type:"none");}
int main(int argc,char **argv) {
 Tcl_FindExecutable(argv[0]);Tcl_Interp *ip=Tcl_CreateInterp();if(Tcl_Init(ip)!=TCL_OK){fprintf(stderr,"init: %s\n",Tcl_GetStringResult(ip));return 2;}
 eval(ip,"oo::configurable create C {property yellow -get {return Y} -set {value {return}}; property zinc -get {return Z} -set {value {return}}}; C create o");
 Object *o=object(ip,"o"),*c=object(ip,"C");Tcl_Size e=o->fPtr->epoch;
 eval(ip,"info object properties o -all");Tcl_Obj *head=o->properties.allReadableCache;Tcl_Size n;Tcl_Obj **members;Tcl_ListObjGetElements(NULL,head,&n,&members);
 row("all-header",o->fPtr->epoch,e-o->fPtr->epoch,Tcl_GetObjResult(ip)==head,head->refCount,members[0]->refCount,members[0]->typePtr?members[0]->typePtr->name:NULL);
 Tcl_Obj *copy=TclListObjCopy(NULL,head);Tcl_IncrRefCount(copy);Tcl_Obj **copied;Tcl_ListObjGetElements(NULL,copy,&n,&copied);row("copy-header",o->fPtr->epoch,0,copy!=head&&copied[0]==members[0],copy->refCount,members[0]->refCount,members[0]->typePtr?members[0]->typePtr->name:NULL);Tcl_DecrRefCount(copy);
 e=o->fPtr->epoch;eval(ip,"oo::objdefine o {::oo::objdefine::method quiet {} {return}};info object properties o -all");row("instance-method",o->fPtr->epoch,o->fPtr->epoch-e,o->properties.allReadableCache==head,o->properties.allReadableCache->refCount,0,NULL);
 e=o->fPtr->epoch;eval(ip,"catch {oo::define C {::oo::define::method live {} {return};error LATE}}");row("class-method-error",o->fPtr->epoch,o->fPtr->epoch-e,o->properties.allReadableCache==head,0,0,NULL);
 eval(ip,"info object properties o -all");row("class-method-refresh",o->fPtr->epoch,0,o->properties.allReadableCache==head,o->properties.allReadableCache->refCount,0,NULL);
 eval(ip,"oo::class create U");Object *u=object(ip,"U");e=u->fPtr->epoch;eval(ip,"oo::define U superclass oo::object");row("untangled-structure",u->fPtr->epoch,u->fPtr->epoch-e,0,0,0,NULL);
 e=o->fPtr->epoch;eval(ip,"oo::define C {::oo::define::superclass oo::object}");row("used-structure",o->fPtr->epoch,o->fPtr->epoch-e,0,0,0,NULL);
 eval(ip,"info object properties o -all; o configure -y");Tcl_ListObjGetElements(NULL,o->properties.allReadableCache,&n,&members);row("property-read",o->fPtr->epoch,0,1,0,members[0]->refCount,members[0]->typePtr?members[0]->typePtr->name:NULL);
 Tcl_DeleteInterp(ip);Tcl_Finalize();return 0;
}
