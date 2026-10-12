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

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original native static library d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011 and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Tcl.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original native static library 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Tcl.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original native static library a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6 and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Tcl.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original native static library 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04 and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Tcl.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original native static library 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33 and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Tcl.

route=compiled kind=scalar code=0 result=MUTATED; route=generic kind=scalar code=0 result=MUTATED

### jim

Status: `observed`. Version: Jim0.84. Build: Original native static library 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff and exact source/output SHA; original executable/startup/compiler argv unavailable.. Channel: Custom native object factory; original C Tcl_Eval/Jim_Eval ASCII source; result read after native hooks.. Dialect: Jim Tcl.

free=0 route=direct kind=scalar code=0 result=MUTATED; free=0 route=alias kind=scalar code=0 result=MUTATED; free=1 route=direct kind=scalar code=0 result=FREED; free=1 route=alias kind=scalar code=0 result=FREED

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/manifest.json](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/manifest.json). SHA-256 `c8b35636c48f0330856e568d4e794ce82505aa43cf4b76b66f0d03a5b7d15056`. 17 original source/output/library rows for independent C/Jim object hook programs; no executable/argv/startup hashes recorded.
- `e1` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-native.json](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-native.json). SHA-256 `fe3f7eae797db7fbbe5e9e04f89ecc3887d9c35c37693dc11c1715c8087e8d32`. Older independent C labelled process outputs including exact stdout/stderr; no library/executable/build is borrowed from the newer corpus.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.4.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.4.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Byte-exact older C stdout projection for labelled 8.4; original precise build unavailable.
- `e3` (input): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Adjacent retained older factory input; the older JSON has no executed-source hash. This is context, not reconstruction of the executed program or equivalence to newer source.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.5.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.5.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Byte-exact older C stdout projection for labelled 8.5; original precise build unavailable.
- `e5` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.6.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-8.6.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Byte-exact older C stdout projection for labelled 8.6; original precise build unavailable.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-9.0.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-9.0.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Byte-exact older C stdout projection for labelled 9.0; original precise build unavailable.
- `e7` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-9.1.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string-callbacks/empty-foreach-9.1.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Byte-exact older C stdout projection for labelled 9.1; original precise build unavailable.
- `e8` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Exact independently executed custom updater/free factory and observation point.
- `e9` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.4.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.4.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Exact complete original stream; selected empty rows belong to this question.
- `e10` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Exact independently executed custom updater/free factory and observation point.
- `e11` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.5.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.5.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Exact complete original stream; selected empty rows belong to this question.
- `e12` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Exact independently executed custom updater/free factory and observation point.
- `e13` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.6.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-8.6.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Exact complete original stream; selected empty rows belong to this question.
- `e14` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Exact independently executed custom updater/free factory and observation point.
- `e15` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-9.0.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-9.0.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Exact complete original stream; selected empty rows belong to this question.
- `e16` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c). SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`. Exact independently executed custom updater/free factory and observation point.
- `e17` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-9.1.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-9.1.txt). SHA-256 `e3b750fbee8885e0ab6a1f32629932227ef6323afbeb498c54330a01fe01ccfa`. Exact complete original stream; selected empty rows belong to this question.
- `e18` (input): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-probe.c](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-probe.c). SHA-256 `52c7b83ea0c29348f286645ee96b28ee8de50944871e2be0575f2ea414e26694`. Exact independently executed custom updater/free factory and observation point.
- `e19` (observation): [rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-native.txt](../../../../rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-native.txt). SHA-256 `082c636323a0700a9c3aed8830be9fc7403645bccf6cc8aa57f36024a9c24002`. Exact complete original stream; selected empty rows belong to this question.

## Source inspection

tcl8.4 8.4.20, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Full-source SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`; snippet SHA-256 `7090ff56b21f5caec79eaec183211a7cb7043acf65dedbad7ac85fe9768c2e78`; retained evidence `e8`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl8.5 8.5.19, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Full-source SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`; snippet SHA-256 `7090ff56b21f5caec79eaec183211a7cb7043acf65dedbad7ac85fe9768c2e78`; retained evidence `e10`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl8.6 8.6.18, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Full-source SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`; snippet SHA-256 `7090ff56b21f5caec79eaec183211a7cb7043acf65dedbad7ac85fe9768c2e78`; retained evidence `e12`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl9.0 9.0.4, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Full-source SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`; snippet SHA-256 `7090ff56b21f5caec79eaec183211a7cb7043acf65dedbad7ac85fe9768c2e78`; retained evidence `e14`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

tcl9.1 9.1.0, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 27–30. Full-source SHA-256 `8879e5c69d2ed691012fea1a35f1f5ded4b8b2715c3a4a45d34b08ad29fa26d0`; snippet SHA-256 `7090ff56b21f5caec79eaec183211a7cb7043acf65dedbad7ac85fe9768c2e78`; retained evidence `e16`.

```text
int main(int argc,char **argv) {
    const char *kinds[]={"scalar"}; unsigned kind,route;
    if(argc!=2)return 2; Tcl_FindExecutable(argv[0]);
    for(route=0;route<2;route++) for(kind=0;kind<1;kind++) {

```

jim Jim0.84, revision `Exact retained executed probe; native engine source snapshot is not reconstructed from its library hash.`, `rust/tcl-syntax/tests/data/native_list_methods/string_callbacks/jim-empty-foreach-probe.c`, function `main: original source/object invocation and observation`, lines 24–27. Full-source SHA-256 `52c7b83ea0c29348f286645ee96b28ee8de50944871e2be0575f2ea414e26694`; snippet SHA-256 `3783f7330e725491864ed7e14a18208ecc49aff1f6177b5eb2d1df6e6073f009`; retained evidence `e18`.

```text
int main(void) {
    const char *kinds[]={"scalar"};int kind,route,free_hook;
    for(free_hook=0;free_hook<2;free_hook++)for(route=0;route<2;route++)for(kind=0;kind<1;kind++) {
        Jim_Interp *interp=Jim_CreateInterp();char script[512];int code;

```


## Consumer bindings

- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `SourceCommandBindings::prepare_object_callbacks`: Separately closes actual original object conversion world effects; bytes/cache do not donate class proof.
- [rust/tcl-compiler/src/command_binding/object_callbacks.rs](../../../../rust/tcl-compiler/src/command_binding/object_callbacks.rs), `command_binding::object_callbacks::tests::stock_conversion_effects_stay_open_on_unknown_zero_trip_inputs` (linked): Retains unknown incoming scalar conversion effects; ordinary current containers require independent selected method/class proof.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-native-list-object-probes.py",
  "--corpus",
  "callbacks",
  "--tcl-source-root",
  "${TCL_SOURCE_ROOT}",
  "--jim-source-root",
  "${JIM_SOURCE_ROOT}",
  "--output",
  "${NEW_OUTPUT}"
]
```

Requires exact original probe and library hashes; independently provision the matching C release or Jim static build. The runner records its fresh compiler arguments, executable hash, process status and streams and compares stdout byte for byte. Original stderr is absent from these manifests; the current replay separately requires empty stderr and does not claim retained stderr equality. Private C instruction probes require matching tclInt.h/tclCompile.h ABI. Missing originals and mismatched libraries are adapter refusals, not native guest answers. C library callbacks require the matching Tcl library directory. Every process has a60-second budget; run serially or obey a global maximum of two native processes. Reconfirmation of the oracle does not establish Rust parity or a native capability.
