# naming.tcloo.stock-destroy-method-allocation

Kind: `native-observation`

## Problem statement

A fallback that advertises destroy independently of the actual inherited method allocation can keep reflection or dispatch alive after that allocation is removed.

## Question

For a fresh C class and O instance, is stock destroy inherited from the actual ::oo::object method allocation, and does removing that allocation withdraw both rosters and invocation?

## Conclusion

The three observed C providers initially advertise destroy on both C and O. Deleting ::oo::object destroy removes both advertisements, and O destroy fails with no visible methods. Each pinned objMethods table declares an exported intrinsic destroy method. This is one inherited allocation/removal control, with no arbitrary dispatch or lifecycle closure.

## Scope

Exact ASCII source, fresh interpreter per observed Tcl8.6.18/9.0.4/9.1.0, original stdout and exits plus separate same-binary version queries. No raw-zero/opaque method operands, private visibility, custom destructor callbacks, cache/header state, compiler admission, Normal or general object deletion equivalence measured. Other providers not launched for this question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded for this question. Build: not recorded. Channel: not launched for this finite question. Dialect: Tcl.

No observation from this finite probe. Availability or behaviour cannot be inferred from another provider.

### tcl8.5

Status: `not-tested`. Version: not recorded for this question. Build: not recorded. Channel: not launched for this finite question. Dialect: Tcl.

No observation from this finite probe. Availability or behaviour cannot be inferred from another provider.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual tclsh executable SHA retained in original capture and independently matched by version-query receipt; exact tclOO.c full SHA retained. Compiler flags, library and header digests unrecorded.. Channel: Exact ASCII LF source bytes passed via stdin to a fresh tclsh process; separate version query uses same executable.. Dialect: Tcl.

BEFORE class C and object O each report destroy. Deleting actual ::oo::object destroy removes it from both rosters; catch {O destroy} reports code1 and object "::O" has no visible methods. Same binary separately reports 8.6.18.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual tclsh executable SHA retained in original capture and independently matched by version-query receipt; exact tclOO.c full SHA retained. Compiler flags, library and header digests unrecorded.. Channel: Exact ASCII LF source bytes passed via stdin to a fresh tclsh process; separate version query uses same executable.. Dialect: Tcl.

BEFORE class C and object O each report destroy. Deleting actual ::oo::object destroy removes it from both rosters; catch {O destroy} reports code1 and object "::O" has no visible methods. Same binary separately reports 9.0.4.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual tclsh executable SHA retained in original capture and independently matched by version-query receipt; exact tclOO.c full SHA retained. Compiler flags, library and header digests unrecorded.. Channel: Exact ASCII LF source bytes passed via stdin to a fresh tclsh process; separate version query uses same executable.. Dialect: Tcl.

BEFORE class C and object O each report destroy. Deleting actual ::oo::object destroy removes it from both rosters; catch {O destroy} reports code1 and object "::O" has no visible methods. Same binary separately reports 9.1.0.

### jim

Status: `not-tested`. Version: not recorded for this question. Build: not recorded. Channel: not launched for this finite question. Dialect: Jim Tcl.

No observation from this finite probe. Availability or behaviour cannot be inferred from another provider.

### bigip

Status: `not-tested`. Version: not recorded for this question. Build: not recorded. Channel: not launched for this finite question. Dialect: F5 iRules.

No observation from this finite probe. Availability or behaviour cannot be inferred from another provider.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/probe.tcl](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/probe.tcl). SHA-256 `2fce418c8bceb36c81645bebddff5dc9704c1ba797edde2f9d0d7105add422f5`. Exact ASCII LF script, passed to each fresh tclsh stdin.
- `capture` (provider): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/receipt.json). SHA-256 `1f032d641892798fd998a64db400285a4b2487a5f2699b80c7a2690a02b02bb8`. Original process exit, executable/source/output/anchor digests; build flags and library/header hashes are not retained by this script capture.
- `version-probe` (input): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/probe.tcl](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/probe.tcl). SHA-256 `f8183395c8b9ce7d2602a62c8fd67bd33e4698122592863a5bc47cb34e754c87`. Separate additive info patchlevel/tclversion script.
- `version-capture` (provider): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/receipt.json). SHA-256 `79352f5b605dbad9766d0fbe6274efc0aed93b1c639dc0fecb6ebe7aeb4fd730`. Same original executable hashes and original receipt hash; separate process0 version queries.
- `stdout-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/8.6.18.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/8.6.18.stdout). SHA-256 `a62ff3518ecddcb2f157c9eb829f0582b2dee3507b773d7cf93d35e885f02157`. Two original rows: inherited class/object rosters before removal and both rosters plus actual O destroy failure after deletion.
- `version-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/8.6.18.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/8.6.18.stdout). SHA-256 `29668c20e5d19ff4694f84937d4fa5ff89170e453eaf70dc693ed1d71353369d`. Same executable separately reports exact patchlevel and Tcl version.
- `source-tcl8.6` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json). SHA-256 `20cfa8c0f2a7826cce1eed6d9b904ca3769d216d3d0df59ba2b8978c47f9ac3f`. JSON pointer `/0/snippet`. Exact objMethods/DCM window retained from the full source whose digest is in original capture.
- `stdout-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/9.0.4.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/9.0.4.stdout). SHA-256 `a62ff3518ecddcb2f157c9eb829f0582b2dee3507b773d7cf93d35e885f02157`. Two original rows: inherited class/object rosters before removal and both rosters plus actual O destroy failure after deletion.
- `version-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/9.0.4.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/9.0.4.stdout). SHA-256 `4ca15c39f2af5d89227af7785878f742753ef3906f625a87dad669d4e80bf80a`. Same executable separately reports exact patchlevel and Tcl version.
- `source-tcl9.0` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json). SHA-256 `20cfa8c0f2a7826cce1eed6d9b904ca3769d216d3d0df59ba2b8978c47f9ac3f`. JSON pointer `/1/snippet`. Exact objMethods/DCM window retained from the full source whose digest is in original capture.
- `stdout-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/9.1.0.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/9.1.0.stdout). SHA-256 `a62ff3518ecddcb2f157c9eb829f0582b2dee3507b773d7cf93d35e885f02157`. Two original rows: inherited class/object rosters before removal and both rosters plus actual O destroy failure after deletion.
- `version-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/9.1.0.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/version-query/9.1.0.stdout). SHA-256 `aabb2775485d430c29816c1e2737adb1c004aa879efa3f8aee19634aac547f40`. Same executable separately reports exact patchlevel and Tcl version.
- `source-tcl9.1` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json](../../../../rust/tcl-registry/tests/data/native_tcloo_stock_destroy/source-windows.json). SHA-256 `20cfa8c0f2a7826cce1eed6d9b904ca3769d216d3d0df59ba2b8978c47f9ac3f`. JSON pointer `/2/snippet`. Exact objMethods/DCM window retained from the full source whose digest is in original capture.

## Source inspection

tcl8.6 8.6.18, revision `exact pinned source associated with original receipt; linked binary version queried separately`, `generic/tclOO.c`, function `objMethods`, lines 106–122. Full-source SHA-256 `e749370dcaeab6d214b811a246536c1cf06f92edf2c51c8f1dbd8f4a6f57b3ef`; snippet SHA-256 `bd7780841ca69a40a4b0e18d7bec1e2774291857ec4956c0f92baedb9ebf0b0d`; retained evidence `source-tcl8.6`.

```text
#define DCM(name,visibility,proc) \
    {name,visibility,\
	{TCL_OO_METHOD_VERSION_CURRENT,"core method: "#name,proc,NULL,NULL}}

static const DeclaredClassMethod objMethods[] = {
    DCM("destroy", 1,	TclOO_Object_Destroy),
    DCM("eval", 0,	TclOO_Object_Eval),
    DCM("unknown", 0,	TclOO_Object_Unknown),
    DCM("variable", 0,	TclOO_Object_LinkVar),
    DCM("varname", 0,	TclOO_Object_VarName),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("create", 1,	TclOO_Class_Create),
    DCM("new", 1,	TclOO_Class_New),
    DCM("createWithNamespace", 0, TclOO_Class_CreateNs),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
};

```

tcl9.0 9.0.4, revision `exact pinned source associated with original receipt; linked binary version queried separately`, `generic/tclOO.c`, function `objMethods`, lines 131–159. Full-source SHA-256 `2e26684094bbe01c988c20386de1a3bf53646f47ce84ad50a807414455d1d3a7`; snippet SHA-256 `e5bc66d9c081231b8aa32aba90cfbf1a8405d210bd6ca45b7429cbfeaf1de7a4`; retained evidence `source-tcl9.0`.

```text
#define DCM(name,visibility,proc) \
    {name,visibility,\
	{TCL_OO_METHOD_VERSION_CURRENT,"core method: "#name,proc,NULL,NULL}}

static const DeclaredClassMethod objMethods[] = {
    DCM("<cloned>", 0,	TclOO_Object_Cloned),
    DCM("destroy", 1,	TclOO_Object_Destroy),
    DCM("eval", 0,	TclOO_Object_Eval),
    DCM("unknown", 0,	TclOO_Object_Unknown),
    DCM("variable", 0,	TclOO_Object_LinkVar),
    DCM("varname", 0,	TclOO_Object_VarName),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("<cloned>", 0,	TclOO_Class_Cloned),
    DCM("create", 1,	TclOO_Class_Create),
    DCM("new", 1,	TclOO_Class_New),
    DCM("createWithNamespace", 0, TclOO_Class_CreateNs),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, cfgMethods[] = {
    DCM("configure", 1, TclOO_Configurable_Configure),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, singletonMethods[] = {
    DCM("new", 1,	TclOO_Singleton_New),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
}, singletonInstanceMethods[] = {
    DCM("<cloned>", 0,	TclOO_SingletonInstance_Cloned),
    DCM("destroy", 1,	TclOO_SingletonInstance_Destroy),
    {NULL, 0, {0, NULL, NULL, NULL, NULL}}
};

```

tcl9.1 9.1.0, revision `exact pinned source associated with original receipt; linked binary version queried separately`, `generic/tclOO.c`, function `objMethods`, lines 132–160. Full-source SHA-256 `f2abb0b2ca6fa8cf621f158f209a48ee20ec2a08073d2f2c4af541ddf0babbbd`; snippet SHA-256 `a59f7d98a7e68ba17412d19252c9a3a9eb629219c87e6b30517bdd22c7654b59`; retained evidence `source-tcl9.1`.

```text
#define DCM(name,visibility,proc) \
    {name,visibility,\
	{TCL_OO_METHOD_VERSION_2,"core method: "#name,proc,NULL,NULL}}

static const DeclaredClassMethod objMethods[] = {
    DCM("<cloned>", false,	TclOO_Object_Cloned),
    DCM("destroy", true,	TclOO_Object_Destroy),
    DCM("eval", false,		TclOO_Object_Eval),
    DCM("unknown", false,	TclOO_Object_Unknown),
    DCM("variable", false,	TclOO_Object_LinkVar),
    DCM("varname", false,	TclOO_Object_VarName),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, clsMethods[] = {
    DCM("<cloned>", false,	TclOO_Class_Cloned),
    DCM("create", true,		TclOO_Class_Create),
    DCM("new", true,		TclOO_Class_New),
    DCM("createWithNamespace", false, TclOO_Class_CreateNs),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, cfgMethods[] = {
    DCM("configure", true,	TclOO_Configurable_Configure),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, singletonMethods[] = {
    DCM("new", true,		TclOO_Singleton_New),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
}, singletonInstanceMethods[] = {
    DCM("<cloned>", false,	TclOO_SingletonInstance_Cloned),
    DCM("destroy", true,	TclOO_SingletonInstance_Destroy),
    {NULL, false, {0, NULL, NULL, NULL, NULL}}
};

```


## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs), `cmd_oo::native_method_info_tests::stock_destroy_allocation_controls_reflection_and_dispatch` (linked): Selected C8.6/9.0/9.1 stock root method allocation is visible in class/object rosters; deleting that same root method removes both and makes object destroy miss dispatch. This binding claims no Rust execution or broader TclOO parity.

A named test is a coverage binding, not a claim that it executed.

## Replay

Immutable original stdin script and receipts allow source/hash/result inspection. Original executable full hashes are retained, but executable files/build commands/libraries/headers are not copied into portable corpus; no automatic native replay is claimed.
