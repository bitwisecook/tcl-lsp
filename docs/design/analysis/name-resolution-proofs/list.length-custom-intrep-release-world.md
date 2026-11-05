# naming.list.length-custom-intrep-release-world

Kind: `native-observation`

## Problem statement

An unknown native scalar can execute an update-string or free-internal-representation hook while preparing a list operation. A known outer List or Dict can contain that same custom scalar without exposing the same callback during Length; a zero-iteration body does not suppress preparation. Class, cache, bytes and normal completion therefore cannot stand in for world-effect closure.

## Question

Does Length release an unknown scalar internal representation while ordinary outer container Length avoids that hook?

## Conclusion

Actual custom scalar Length conversion invokes the free hook and writes FREED across C5 and Jim; ordinary outer container controls remain SAFE at the measured point. The later teardown hook is separate from the captured result and is disabled in the Jim harness after observation. This is a purpose-specific release observation, not general ordinary-object lifetime closure.

## Scope

Actual custom native object factories; ASCII source passed to Tcl_Eval/Jim_Eval. C compiled and alias-generic paths and Jim direct/alias paths are independently retained. The older hyphen-directory captures contain C version labels and process streams but no native library/executable attribution; the underscore corpus records independent sources/library/outputs. No equivalence of differing original probes or full header/cache/frame/Normal grant is assumed.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks. Version: tcl8.4 8.4.20.

route=compiled kind=scalar code=0 result=1 FREED custom-scalar list; route=compiled kind=list code=0 result=1 SAFE list list; route=generic kind=scalar code=0 result=1 FREED custom-scalar list; route=generic kind=list code=0 result=1 SAFE list list

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

route=compiled kind=scalar code=0 result=1 FREED custom-scalar list; route=compiled kind=list code=0 result=1 SAFE list list; route=compiled kind=dict code=0 result=2 SAFE dict list; route=generic kind=scalar code=0 result=1 FREED custom-scalar list; route=generic kind=list code=0 result=1 SAFE list list; route=generic kind=dict code=0 result=2 SAFE dict list

### jim

Status: `observed`. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks. Version: jim Jim0.84.

free=1 route=direct kind=scalar code=0 result=1 FREED; free=1 route=direct kind=list code=0 result=1 SAFE; free=1 route=direct kind=dict code=0 result=2 SAFE; free=1 route=alias kind=scalar code=0 result=1 FREED; free=1 route=alias kind=list code=0 result=1 SAFE; free=1 route=alias kind=dict code=0 result=2 SAFE

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/free-probe.c`, function `main: original source/object invocation and observation`, lines 31–34. Retained reference `e8`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar","list","dict"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<3;kind++) {

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/free-probe.c`, function `main: original source/object invocation and observation`, lines 31–34. Retained reference `e10`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar","list","dict"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<3;kind++) {

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/free-probe.c`, function `main: original source/object invocation and observation`, lines 31–34. Retained reference `e12`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar","list","dict"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<3;kind++) {

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/free-probe.c`, function `main: original source/object invocation and observation`, lines 31–34. Retained reference `e14`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar","list","dict"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<3;kind++) {

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/free-probe.c`, function `main: original source/object invocation and observation`, lines 31–34. Retained reference `e16`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar","list","dict"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<3;kind++) {

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-probe.c`, function `main: original source/object invocation and observation`, lines 24–27. Retained reference `e18`.

```text
int main(void) {
    const char *kinds[]={"scalar","list","dict"};int kind,route,free_hook;
    for(free_hook=0;free_hook<2;free_hook++)for(route=0;route<2;route++)for(kind=0;kind<3;kind++) {
        Jim_Interp *interp=Jim_CreateInterp();char script[512];int code;

```

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `SourceCommandBindings::prepare_object_callbacks`: Separately closes actual original object conversion world effects; bytes/cache do not donate class proof.
- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `command_binding::object_callbacks::tests::unknown_list_inputs_withdraw_callback_worlds_on_every_engine` (linked): Retains unknown incoming scalar conversion effects; ordinary current containers require independent selected method/class proof.

## Evidence

- [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks)
- [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks)
