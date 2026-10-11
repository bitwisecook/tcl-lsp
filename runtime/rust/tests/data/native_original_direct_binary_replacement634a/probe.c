/* SPDX-License-Identifier: AGPL-3.0-or-later
 * Genuine C9 TclStringReplace producer boundary. Parsed first/count values
 * are supplied as C literals; this is NOT Tcl_EvalObjv/index-parser evidence.
 * Actual private header supplies the symbol declaration and flags enum.
 */
#include "tclInt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
struct Case { const char *id; Tcl_Size first, count; int omit, shared, in_place, resident_subject, resident_replacement; };
static const struct Case cases[] = {
    { "equal-unshared-inplace",1,2,0,0,1,0,0 },
    { "equal-shared-inplace",1,2,0,1,1,0,0 },
    { "whole-original-replacement",0,4,0,0,1,0,0 },
    { "partial-deletion-unspecified-prefix",1,2,1,0,1,0,0 },
    { "whole-deletion-empty",0,4,1,0,1,0,0 },
    { "equal-unshared-no-inplace",1,2,0,0,0,0,0 },
    { "replacement-resident-string",1,2,0,0,1,0,1 },
    { "subject-resident-string",1,2,0,0,1,1,0 }
};
static void snapshot(const char *name,Tcl_Obj *o) {
    printf("%s\t%s\t%d\t%lld\t%lld\n",name,o->typePtr?o->typePtr->name:"none",o->bytes!=NULL,(long long)o->refCount,(long long)o->length);
}
static void bytes(const char *name,Tcl_Obj *o) {
    Tcl_Size n,k; const unsigned char *s=(const unsigned char*)Tcl_GetStringFromObj(o,&n);
    printf("%s\t",name); for(k=0;k<n;k++)printf("%02x",s[k]);printf("\n");
}
int main(int argc,char **argv) {
    static const unsigned char subject_bytes[]={0x41,0x00,0xff,0x5a};
    static const unsigned char replacement_bytes[]={0x51,0xfe};
    Tcl_Interp *interp; Tcl_Obj *subject,*replacement,*result; int n,flags;
    if(argc!=2)return 64;
    n=atoi(argv[1]);if(n<0||n>=(int)(sizeof(cases)/sizeof(cases[0])))return 65;
    Tcl_FindExecutable(argv[0]);interp=Tcl_CreateInterp();
    subject=Tcl_NewByteArrayObj(subject_bytes,sizeof(subject_bytes));Tcl_IncrRefCount(subject);
    replacement=Tcl_NewByteArrayObj(replacement_bytes,sizeof(replacement_bytes));Tcl_IncrRefCount(replacement);
    if(cases[n].shared)Tcl_IncrRefCount(subject);
    flags=cases[n].in_place?TCL_STRING_IN_PLACE:0;
    printf("case\t%d\t%s\n",n,cases[n].id);
    printf("purpose\tdirect-TclStringReplace\tparsed-first=%lld\tparsed-count=%lld\tflags=%d\tomit=%d\textra-subject-reference=%d\n",(long long)cases[n].first,(long long)cases[n].count,flags,cases[n].omit,cases[n].shared);
    printf("input-subject\t4100ff5a\ninput-replacement\t51fe\n");
    if(cases[n].resident_subject)bytes("setup-subject-string",subject);
    if(cases[n].resident_replacement)bytes("setup-replacement-string",replacement);
    snapshot("before-subject",subject);snapshot("before-replacement",replacement);
    result=TclStringReplace(interp,subject,cases[n].first,cases[n].count,cases[n].omit?NULL:replacement,flags);
    printf("returned\t%d\n",result!=NULL);
    snapshot("reached-subject",subject);snapshot("reached-replacement",replacement);
    if(result) {
        snapshot("reached-result",result);
        printf("result-alias\tsubject=%d\treplacement=%d\n",result==subject,result==replacement);
        bytes("result-bytes",result);snapshot("materialised-result",result);
        /* A genuine fresh-zero result not aliasing an owned input is retired
         * only after every public snapshot and result getter has completed. */
        if(result!=subject&&result!=replacement)Tcl_BounceRefCount(result);
    }
    bytes("subject-bytes",subject);
    if(cases[n].shared)Tcl_DecrRefCount(subject);
    Tcl_DecrRefCount(subject);Tcl_DecrRefCount(replacement);Tcl_DeleteInterp(interp);Tcl_Finalize();return 0;
}
