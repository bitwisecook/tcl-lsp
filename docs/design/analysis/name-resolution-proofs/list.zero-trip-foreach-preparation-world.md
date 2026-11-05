# naming.list.zero-trip-foreach-preparation-world

Kind: `native-observation`

## Problem statement

An unknown native scalar can execute an update-string or free-internal-representation hook while preparing a list operation. A known outer List or Dict can contain that same custom scalar without exposing the same callback during Length; a zero-iteration body does not suppress preparation. Class, cache, bytes and normal completion therefore cannot stand in for world-effect closure.

## Question

Can zero-trip foreach preparation execute a custom scalar hook before any body iteration?

## Conclusion

Actual C5 and Jim custom scalar preparation mutates keep before the empty foreach loop body can iterate; Jim independently shows updater-only and free-enabled modes. Empty-loop source shape and observed zero body iterations do not close conversion effects.

## Scope

Actual custom native object factories; ASCII source passed to Tcl_Eval/Jim_Eval. C compiled and alias-generic paths and Jim direct/alias paths are independently retained. The older hyphen-directory captures contain C version labels and process streams but no native library/executable attribution; the underscore corpus records independent sources/library/outputs. No equivalence of differing original probes or full header/cache/frame/Normal grant is assumed.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### jim

Status: `observed`. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks. Version: jim Jim0.84.

free=0 route=direct kind=scalar code=0 result=MUTATED; free=0 route=alias kind=scalar code=0 result=MUTATED; free=1 route=direct kind=scalar code=0 result=FREED; free=1 route=alias kind=scalar code=0 result=FREED

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Retained reference `e8`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Retained reference `e10`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Retained reference `e12`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Retained reference `e14`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Retained reference `e16`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 24–27. Retained reference `e18`.

```text
int main(void) {
    const char *kinds[]={"scalar"};int kind,route,free_hook;
    for(free_hook=0;free_hook<2;free_hook++)for(route=0;route<2;route++)for(kind=0;kind<1;kind++) {
        Jim_Interp *interp=Jim_CreateInterp();char script[512];int code;

```

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `SourceCommandBindings::prepare_object_callbacks`: Separately closes actual original object conversion world effects; bytes/cache do not donate class proof.
- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `command_binding::object_callbacks::tests::stock_conversion_effects_stay_open_on_unknown_zero_trip_inputs` (linked): Retains unknown incoming scalar conversion effects; ordinary current containers require independent selected method/class proof.

## Evidence

- [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks)
- [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks)
