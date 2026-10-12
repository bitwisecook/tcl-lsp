/* tcl-lsp -- current original-header String replacement controls.
 * Copyright (C) 2026 James Deucker (bitwisecook).
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * Every source below is a counted C byte array, not Tcl source text.
 * No index/subject/result getter occurs between the original EvalObjv and
 * the "reached" snapshots. Result bytes are materialised only afterward.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef PROBE_JIM
#include "jim.h"
typedef Jim_Obj Object;
typedef Jim_Interp Interpreter;
static Object *new_bytes(Interpreter *i, const unsigned char *s, int n) { return Jim_NewStringObj(i, (const char *)s, n); }
static void hold(Object *o) { Jim_IncrRefCount(o); }
static void release(Interpreter *i, Object *o) { Jim_DecrRefCount(i, o); }
static void snapshot(const char *name, Object *o) {
    printf("%s\t%s\t%d\t%d\t%lld", name, o->typePtr ? o->typePtr->name : "none", o->bytes != NULL, o->refCount, (long long)o->length);
    if (o->typePtr && strcmp(o->typePtr->name, "string") == 0) printf("\tchars=%d", o->internalRep.strValue.charLength);
    printf("\n");
}
static void bytes(Interpreter *i, const char *name, Object *o) {
    int n, k; const unsigned char *s = (const unsigned char *)Jim_GetString(o, &n);
    (void)i; printf("%s\t", name); for (k = 0; k < n; k++) printf("%02x", s[k]); printf("\n");
}
#else
#include "tcl.h"
typedef Tcl_Obj Object;
typedef Tcl_Interp Interpreter;
static Object *new_bytes(Interpreter *i, const unsigned char *s, int n) { (void)i; return Tcl_NewStringObj((const char *)s, n); }
static void hold(Object *o) { Tcl_IncrRefCount(o); }
static void release(Interpreter *i, Object *o) { (void)i; Tcl_DecrRefCount(o); }
static void snapshot(const char *name, Object *o) {
    printf("%s\t%s\t%d\t%d\t%lld\n", name, o->typePtr ? o->typePtr->name : "none", o->bytes != NULL, o->refCount, (long long)o->length);
}
static void bytes(Interpreter *i, const char *name, Object *o) {
#if TCL_MAJOR_VERSION >= 9
    Tcl_Size n, k;
#else
    int n, k;
#endif
    const unsigned char *s = (const unsigned char *)Tcl_GetStringFromObj(o, &n);
    (void)i; printf("%s\t", name); for (k = 0; k < n; k++) printf("%02x", s[k]); printf("\n");
}
#endif
struct Input { const unsigned char *bytes; int length; };
struct Case { const char *id; struct Input subject, first, last, replacement; int omit, alias_first, alias_last, alias_replacement; };
#define B(s) { (const unsigned char *)(s), (int)(sizeof(s) - 1) }
static struct Case cases[] = {
    { "ascii-inverted", B("abcdef"), B("2"), B("1"), B("!"), 0, 0, 0, 0 },
    { "ascii-past-end", B("abcdef"), B("9"), B("end"), B("!"), 0, 0, 0, 0 },
    { "ascii-replaced", B("abcdef"), B("1"), B("2"), B("X"), 0, 0, 0, 0 },
    { "ascii-deleted", B("abcdef"), B("1"), B("2"), B(""), 1, 0, 0, 0 },
    { "utf-two-byte-replaced", B("A\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80Z"), B("1"), B("1"), B("X"), 0, 0, 0, 0 },
    { "utf-three-byte-replaced", B("A\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80Z"), B("2"), B("2"), B("X"), 0, 0, 0, 0 },
    { "utf-four-byte-deleted", B("A\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80Z"), B("3"), B("3"), B(""), 1, 0, 0, 0 },
    { "utf-inverted", B("A\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80Z"), B("4"), B("3"), B("!"), 0, 0, 0, 0 },
    { "utf-whole-replaced", B("A\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80Z"), B("0"), B("end"), B("\xce\xa9"), 0, 0, 0, 0 },
    { "counted-nul-before-replaced", B("A\0B\xc3\xa9Z"), B("0"), B("0"), B("X"), 0, 0, 0, 0 },
    { "counted-nul-replaced", B("A\0B\xc3\xa9Z"), B("1"), B("1"), B("X"), 0, 0, 0, 0 },
    { "counted-nul-after-replaced", B("A\0B\xc3\xa9Z"), B("2"), B("2"), B("X"), 0, 0, 0, 0 },
    { "counted-nul-replacement", B("abcdef"), B("1"), B("2"), B("Q\0R\xc3\xa9"), 0, 0, 0, 0 },
    { "counted-invalid-byte", B("A\xff" "B"), B("1"), B("1"), B("X"), 0, 0, 0, 0 },
    { "counted-first-index", B("abcdef"), B("1\0tail"), B("2"), B("X"), 0, 0, 0, 0 },
    { "counted-end-suffix-index", B("abcdef"), B("0"), B("end-1\0tail"), B("X"), 0, 0, 0, 0 },
    { "invalid-original-expression", B("abcdef"), B("$x"), B("2"), B("X"), 0, 0, 0, 0 },
    { "subject-is-first-index", B("2*1"), B(""), B("end"), B("X"), 0, 1, 0, 0 },
    { "same-first-last-header", B("abcdef"), B("1"), B(""), B("X"), 0, 0, 1, 0 },
    { "subject-is-replacement", B("abc"), B("1"), B("1"), B(""), 0, 0, 0, 1 },
    { "replacement-unreached", B("abcdef"), B("4"), B("2"), B("\xff\0Q"), 0, 0, 0, 0 }
};
static void input_hex(const char *name, struct Input input) {
    int k; printf("%s\t", name); for (k = 0; k < input.length; k++) printf("%02x", input.bytes[k]); printf("\n");
}
int main(int argc, char **argv) {
    Interpreter *i; Object *objects[6], *owners[6], *result; int ownerc = 0, objc, code, n, k;
    if (argc != 2) return 64;
    n = atoi(argv[1]); if (n < 0 || n >= (int)(sizeof(cases) / sizeof(cases[0]))) return 65;
#ifdef PROBE_JIM
    i = Jim_CreateInterp(); Jim_RegisterCoreCommands(i);
    printf("provider\tJim\n");
#else
    Tcl_FindExecutable(argv[0]); i = Tcl_CreateInterp();
    printf("provider\tC-Tcl\n");
#endif
    printf("case\t%d\t%s\n", n, cases[n].id);
    input_hex("input-subject", cases[n].subject);
    input_hex("input-first", cases[n].first);
    input_hex("input-last", cases[n].last);
    input_hex("input-replacement", cases[n].replacement);
    printf("input-roles\tomit=%d\tfirst=subject:%d\tlast=first:%d\treplacement=subject:%d\n",
        cases[n].omit, cases[n].alias_first, cases[n].alias_last, cases[n].alias_replacement);
#define MAKE(slot, data) do { objects[slot] = new_bytes(i, (data).bytes, (data).length); hold(objects[slot]); owners[ownerc++] = objects[slot]; } while (0)
    { struct Input command = B("string"), selector = B("replace"); MAKE(0, command); MAKE(1, selector); }
    MAKE(2, cases[n].subject);
    if (cases[n].alias_first) objects[3] = objects[2]; else MAKE(3, cases[n].first);
    if (cases[n].alias_last) objects[4] = objects[3]; else MAKE(4, cases[n].last);
    if (cases[n].alias_replacement) objects[5] = objects[2]; else MAKE(5, cases[n].replacement);
    objc = cases[n].omit ? 5 : 6;
    for (k = 2; k < 6; k++) { char label[32]; sprintf(label, "before-arg%d", k); snapshot(label, objects[k]); }
#ifdef PROBE_JIM
    Jim_SetResultString(i, "PRIOR", 5);
    code = Jim_EvalObjVector(i, objc, objects);
    result = i->result;
#else
    Tcl_SetObjResult(i, Tcl_NewStringObj("PRIOR", 5));
    code = Tcl_EvalObjv(i, objc, objects, TCL_EVAL_GLOBAL);
    result = Tcl_GetObjResult(i);
#endif
    printf("code\t%d\n", code);
    for (k = 2; k < 6; k++) { char label[32]; sprintf(label, "reached-arg%d", k); snapshot(label, objects[k]); }
    snapshot("reached-result", result);
    printf("result-alias\tsubject=%d\tfirst=%d\tlast=%d\treplacement=%d\n",
        result == objects[2], result == objects[3], result == objects[4], result == objects[5]);
    bytes(i, "result-bytes", result);
    snapshot("materialised-result", result);
    bytes(i, "subject-bytes", objects[2]);
    for (k = ownerc - 1; k >= 0; k--) release(i, owners[k]);
#ifdef PROBE_JIM
    Jim_FreeInterp(i);
#else
    Tcl_DeleteInterp(i); Tcl_Finalize();
#endif
    return 0;
}
