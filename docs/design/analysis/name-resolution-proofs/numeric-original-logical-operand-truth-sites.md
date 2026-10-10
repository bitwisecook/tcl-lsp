# naming.numeric.original-logical-operand-truth-sites

Kind: `native-observation`

## Problem statement

A logical operand, primitive Boolean getter and command condition can reach different conversion paths even for one original value. Operand position and generic versus held object-source entry are independent premises. This fixed experiment measures them with genuinely bound x/y originals and separates reached cache/result/error effects from source/compiler or application authority.

## Question

For each actual held original x constructor, how do primitive/expression controls and left/right &&/|| truth sites differ between generic Tcl_EvalEx source evaluation and independently held original Tcl_EvalObjEx compiled-script entry, with actual original y Int1 for && or Int0 for ||, across all six providers and their actual ABI? Jim independently repeats its genuine Jim_EvalObj route without assuming a second engine.

## Conclusion

Each of six original providers completes 648 independent constructor/site rows with 648 matching pre-call observations: 36 held x constructors across 18 separate primitive, expression API, generic-source and held object-source routes. All 3888 pre/post x variable identities remain the original; each pre-call x has refcount 2. Logical rows independently install actual y Int1 for AND or Int0 for OR, retaining its own binding/cache/residency/refcount fields. On C8.4 fresh String 4294967296, x in the left AND/OR position returns false with Boolean primary, while x in the reached right position returns true with integer primary; both generic and held object-source pairs reproduce this distinction. C8.4 cached Double NaN succeeds at the measured logical sites, but if fails in both generic and object-source routes. On C8.6, generic if on original String on leaves the x primary NULL, while object-source if installs booleanString; C9 object-source if installs boolean. C8.6/C9 cached NaN if reports a domain error through generic evaluation but floating point value is Not a Number through object-source evaluation, with their separately recorded error codes. C8.5 retains its own distinct cache/diagnostic behavior. Jim cached 4294967296 remains raw primitive Boolean 0 while all measured logical/expression sites return true; Jim cached NaN primitive refusal remains separate from successful logical/script routes and their String effects. All 1440 C object-source rows record an initially NULL script primary becoming bytecode; all 288 Jim object-source rows record script instead. These are actual held source-object primary observations, not arbitrary instruction or application admission.

Successful rows within each exact route are counted below; an exit0 host process includes all measured route errors. Object-source entries name actual API calls, with their script primary recorded independently.

| Worker and actual entry | C8.4 | C8.5 | C8.6 | C9.0 | C9.1 | Jim |
|---|---:|---:|---:|---:|---:|---:|
| 0 — Raw primitive API | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 19 / 36 |
| 1 — Expression Boolean API | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 2 — Two NOT, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 3 — Ternary, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 4 — if, generic | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 5 — while, generic | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 6 — AND x left, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 7 — AND x right, generic | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 8 — OR x left, generic | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 9 — OR x right, generic | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 10 — Two NOT, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 11 — Ternary, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 12 — if, object source | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 13 — while, object source | 30 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 14 — AND x left, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 15 — AND x right, object source | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 16 — OR x left, object source | 34 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |
| 17 — OR x right, object source | 31 / 36 | 31 / 36 | 31 / 36 | 32 / 36 | 32 / 36 | 34 / 36 |

## Scope

Exactly 36 constructors and 18 workers, each in a fresh fully initialized original interpreter, on the measured native LP64 ABI: int=4, long=8, wide=8, double=8, pointer=8, CHAR_BIT=8; C8 Tcl_UniChar=2 and C9=4, current Jim UTF8/ABI 102. The original x value is installed through the public variable API without source serialization. Workers 0/1 are the raw primitive/public expression API controls; worker 2 is TWO NOT operations and a final expression result, not an isolated inner NOT. Workers 2–9 use genuine generic Tcl_EvalEx source; workers 10–17 separately retain the same corresponding original script object and call Tcl_EvalObjEx(flags0). Jim uses Jim_EvalObj independently for both groups without an assumed second engine. Logical y=1/y=0 makes x reached in either operand position; this gives no statement about unexecuted short-circuit operands. Source objects remain host-held across every post-call observer; their primary and native refcount fields are measured before result/error/input String observers. Input and partner cache/residency/equality fields are also recorded at that bounded pre-observer window; input_after_observers is later rendered data, not initial storage. Direct API output starts sentinel 777; failed API output retains it, and script-route out 777 is unused because truth is in result bytes. errno is an ambient return observation, not a pure numeric-stage or freshness witness. C8.5+ errorCode uses public return-options/DictObjGet, while C8.4/Jim use their separately seeded public variable; no private error object identity follows. All 18 provider compile/version/main operations exit 0 with empty stderr, and the separate compiler version operation exits0; successful process closure includes every expected route error. All 504 input associations and whole original streams/receipts are independently verified. The Root partial unsealed 30-row and complete focused 180-row arrays are preserved as derived review only, never closure or eligibility evidence. No wasm32, LLP64, BIG-IP, null-interpreter route, default/application CABI issuer, runtime representation authority, physical frame/SSA/edit permission or Rust assertion is inferred. Bytecode/script primary observations do not certify exact opcodes, a generic-source compilation path, arbitrary source entry, worker or compiled artifact. Pinned source windows explain distinct entry/conversion branches without claiming unrecorded binary build history.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original observer ELF SHA c2d7e3fe6d7654a12045d53f3954476575c1e3a6076244d83b038ea0173e44a9; archive SHA 532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. Fresh String 4294967296 is false/Boolean in left logical position and true/int on the reached right, through both generic and object-source entry. Cached NaN logical sites succeed while generic/object if fails. All 288 held script objects acquire bytecode; original x/y identities and separate result/error/cache windows remain exact.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original observer ELF SHA 7b5fc13de769419738a6845311fec7c2d3c87f29d63a78987d230bf404e6d14d; archive SHA 94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. All 18 routes have 31 successful constructors each. Generic/object if on on both install booleanString, and cached NaN if retains its domain-error presentation in both entries. All 288 held script objects acquire bytecode; full raw rows retain other purpose-specific diagnostics and cache effects.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original observer ELF SHA eace7c1f04684392ebe79ec29c868d66d0c39f040ea69bb67ca147d8062be873; archive SHA 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. All 18 routes have 31 successful constructors each. Generic if on on leaves x primary NULL; held object-source if installs booleanString. Cached NaN if has domain error in generic entry versus floating point value is Not a Number and its typed code in object-source entry. All 288 script objects acquire bytecode.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original observer ELF SHA 7343ec38b085bd4cd31cb7ef8731d0dfd78b8f1fc93728dc3185d6f671a053dd; archive SHA dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. All 18 routes have 32 successful constructors each. Generic if on on leaves x primary NULL; object-source if installs boolean. Cached NaN generic/object if diagnostics and error codes differ. All 288 script objects acquire bytecode; all other raw result/cache/String-effect fields stay independently observed.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original observer ELF SHA f2d9070dc4e4070b63c00c8332696702c6446401a8c785a2a8b6105b39225dbc; archive SHA 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. All 18 routes have 32 successful constructors each. Generic if on on leaves x primary NULL; object-source if installs boolean. Cached NaN generic/object if diagnostics and error codes differ. All 288 script objects acquire bytecode, without arbitrary instruction or source-entry authority.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original observer ELF SHA d9deef9f3345d5f552f2633869004c645e0cd2a879fe250c9e3e222dfd15ac99; archive SHA a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da. Exact GCC14 command/version, independent CLI version and actual runtime/header metadata retained. Native LP64 int=4/long=8/wide=8/double=8/pointer=8/CHAR_BIT=8; full source/config/archive/dependency/CLI pins checked before and after. No unrecorded configure/build history.. Channel: Held original constructor x and independent actual y, public variable installation, exact fixed ASCII script; separate generic versus retained object-source entry. Dialect: Actual original C release/current Jim distribution and observed LP64 ABI; each site/entry keeps its own conversion and source-object primary purpose.

648 paired rows. Primitive succeeds 19 of 36, each other route 34 of 36. Cached integer 4294967296 raw primitive 0 differs from true at every logical/expression site. Cached NaN primitive fails while logical/script routes succeed with distinct String/result effects. All 288 held source objects acquire script; both groups use actual Jim_EvalObj, not two assumed engines.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Original logical operands/entry/ABI.

No original appliance operand, object-source entry or ABI process was requested or executed.

## Exact evidence

- `native_logical_operand_truth_sites_original-8.4.20-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/cli-version.stdout). SHA-256 `ede2f9015aff14bc8edd1fd1b2ec6ecc3dc7b6a649207bbb9c4c2f7aa18b31a9`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/execute-direct.stdout). SHA-256 `de52feaec6db2efa577af0253d79f1497a6a9fba66cf92421f76b8a2d4dfc2c9`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/probe.elf). SHA-256 `c2d7e3fe6d7654a12045d53f3954476575c1e3a6076244d83b038ea0173e44a9`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.4.20/receipt.json). SHA-256 `0a92bce78100ac13b0b8df1015014466fef913b7919c6b97bd3f814767c565b4`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/cli-version.stdout). SHA-256 `510e2d09d60dc006dfc5d5878788779520c3f1bc62afb4fe7e48870f9de74fe1`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/execute-direct.stdout). SHA-256 `4220f806a39c3db3572d764ad718891340e346c927216ff1e90226752ee30a9e`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/probe.elf). SHA-256 `7b5fc13de769419738a6845311fec7c2d3c87f29d63a78987d230bf404e6d14d`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.5.19/receipt.json). SHA-256 `455351d44073645b3adb314743a40c1fe9bf0009e119e45bd3d725de347abcd5`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/cli-version.stdout). SHA-256 `7ca7e2ae25a10e99a9945869f21ac489508c93f296803490941ae66587c10ec0`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/execute-direct.stdout). SHA-256 `2bf426d14b81141079994d0de7fb2b8f1769fd2e1e86b6965ed21c60153a278d`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/probe.elf). SHA-256 `eace7c1f04684392ebe79ec29c868d66d0c39f040ea69bb67ca147d8062be873`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/8.6.18/receipt.json). SHA-256 `6d30d6e4f3dd67ecdd929cfab8428c8ee4213a097f6e5726cac2a73f9c1b6e6c`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/cli-version.stdout). SHA-256 `32ada7140f59d3bb7c967706b61cdf727ee4051c1f011f7477694f5b232d1ec9`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/execute-direct.stdout). SHA-256 `d09ad252051bc0019403f2bf52d7ed3a86625e3e8a989708ef114fe430f4a560`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/probe.elf). SHA-256 `7343ec38b085bd4cd31cb7ef8731d0dfd78b8f1fc93728dc3185d6f671a053dd`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.0.4/receipt.json). SHA-256 `9a788720eee4080be87fcf17f3f7ea1df4a1cba0e49bb61f3a09a8ebb2ec753c`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/cli-version.stdout). SHA-256 `a88879ffa7ac4f34863959ff7f314ea863fd002511e61073c6fb8f586394d310`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/execute-direct.stdout). SHA-256 `cf0e63819a036651b7d3e69568d2beacbc98e1b9ef76defa0644470f7d5b7af8`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/probe.elf). SHA-256 `f2d9070dc4e4070b63c00c8332696702c6446401a8c785a2a8b6105b39225dbc`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/9.1.0/receipt.json). SHA-256 `fd54d48a651dbc7bdddd56f98aaeef82e07719e337f2c283f10e47be0a4caeb6`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-closure.json` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/closure.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/closure.json). SHA-256 `f7a3ee2db11c0177934d846a9f88c3bad404069c6c675bd8a0d20d12cc8ce5d4`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-compiler-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-compiler-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler-version.stdout). SHA-256 `2ae7618a3dfad6644f70dbf191f205882a96d3aac6350978839c4c19389f95a5`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-compiler.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/compiler.json). SHA-256 `f6a281d3e8bc19aafa9f2b665ab1ced9ccdb3a3a07fd64de7722cdb7bcbc8a5c`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-focused-logical-and-route-observations-complete.json` (reconfirmation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/focused-logical-and-route-observations-complete.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/focused-logical-and-route-observations-complete.json). SHA-256 `8a4839e402e195ac622f7622eaf659528a0afcb0893cba02d3ef1b66d28e4aea`. Unchanged Root complete180-row focused derived array; not an independent execution or authority for matrix closure. Complete raw original streams/receipts are verified separately.
- `native_logical_operand_truth_sites_original-jim-cli-version.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/cli-version.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/cli-version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-cli-version.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/cli-version.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/cli-version.stdout). SHA-256 `f061a4141f0c4f07220cc8a8656910dd43801c0e2f4672f43bb50aa93ca31f73`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-execute-direct.stderr` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/execute-direct.stderr](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/execute-direct.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-execute-direct.stdout` (observation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/execute-direct.stdout](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/execute-direct.stdout). SHA-256 `dd8d05ce7b7d68b73b5c4c26d230fa6d7aecf636683ec4234dbbb47fa6faaed2`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-probe.elf` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/probe.elf](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/probe.elf). SHA-256 `d9deef9f3345d5f552f2633869004c645e0cd2a879fe250c9e3e222dfd15ac99`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/jim/receipt.json). SHA-256 `f4c29e0aafaed8a4eb174e4050928b93e2e282f055822d317d350a9a92f1416f`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-partial-derived-first-provider-observations-unsealed.json` (reconfirmation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/partial-derived-first-provider-observations-unsealed.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/partial-derived-first-provider-observations-unsealed.json). SHA-256 `cead5d8fc9ad6f7bd5f182cff196674ea027509f3bdbd2ceee53cda28dba1025`. Unchanged Root partial unsealed30-row first-provider derived array, retained only as separate review provenance. It proves no matrix closure, eligibility or process outcome. Raw original streams and receipts remain authoritative.
- `native_logical_operand_truth_sites_original-probe.c` (input): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/probe.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/probe.c). SHA-256 `a903921b852280ac8b37cec36b0d6fb8e6928d22c2589449ecc39e2d455e696f`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_logical_operand_truth_sites_original-request.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/request.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/request.json). SHA-256 `b554cf39546d1978dfe3311329f3f7e58e3275e4121abf52aab5dc4ceccabee9`. Unchanged original fixed request/probe, exact compiler/version/process stream, generated observer executable or closed receipt.
- `native_jim_class_source269-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.4.20/receipt.json). SHA-256 `0f7234e7d052215ad3586d3f98dbfe640c84868c10d915cda35c612b11dfcf63`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tcl.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tcl.h). SHA-256 `824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-unix-libtcl8.4.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/libtcl8.4.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/libtcl8.4.a). SHA-256 `532be0a794ca8277c5ad979a227c38600f27c3c91d436e5d3731d661c271fe47`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/unix/Makefile). SHA-256 `0fb0c580d2402093bb212ad340ea87f88822aa5844a8114e112bec4c990411b1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-8.4.20-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c). SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInterp.c). SHA-256 `76c22a84c35e960f7936999f0f19a1aef60c83756a4f3d26d885cfdbd4288230`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclPkg.c). SHA-256 `b2cfb209be52a7caff1fec7cbcdea3970559646555c5a9081e89daecacd57ce9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-usr--lib--x86_64-linux-gnu--libssl.so.3` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libssl.so.3](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libssl.so.3). SHA-256 `4b34859318555bf96d518131bed5fc4d5e35fefa4e1729f49eaa26bd05263850`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-usr--lib--x86_64-linux-gnu--libcrypto.so.3` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libcrypto.so.3](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libcrypto.so.3). SHA-256 `8bb5f3fdffe280d4453eb79a4663c2c47af70b7c247fe2e94e2da703cee1fd3d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-usr--lib--x86_64-linux-gnu--libz.so.1.3.1` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libz.so.1.3.1](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--lib--x86_64-linux-gnu--libz.so.1.3.1). SHA-256 `85590dd58edf5445e18bc7193e5ebc01ac5841f1ae187e97705a662e90c6421e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-8.4.20-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.4.20/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.4.20/provider.elf). SHA-256 `f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-usr--bin--x86_64-linux-gnu-gcc-14` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--bin--x86_64-linux-gnu-gcc-14](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/usr--bin--x86_64-linux-gnu-gcc-14). SHA-256 `a23ecab8ff08f09ad8c80602c2c5df7f49e09c25905cb8975902e101bf72635f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-original-inputs-tmp--receiver-logical-operand-truth-request465--request.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--request.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--request.json). SHA-256 `b554cf39546d1978dfe3311329f3f7e58e3275e4121abf52aab5dc4ceccabee9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-tmp--receiver-capi-scalar-getter-request456--c-version.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/tmp--receiver-capi-scalar-getter-request456--c-version.tcl](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/tmp--receiver-capi-scalar-getter-request456--c-version.tcl). SHA-256 `94435c94329c585b6c1aeab67588b472dfce542e43c68cfc63a214ccdc6e72ea`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--unix--tclConfig.sh` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--unix--tclConfig.sh](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--unix--tclConfig.sh). SHA-256 `e67ced9d9a920741fd6ed13a7462acf7f7ccd75283e7ce1daa0ef8f3e7f712ca`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclRegexp.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclRegexp.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclRegexp.h). SHA-256 `53b4d59c5931f4a2cd101376c6e48567d8cdc4e7e5b1395bba5e6d3e12b1b8b5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--regerrs.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regerrs.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regerrs.h). SHA-256 `bdd9d0e5c9487617e1a104f42a5f943c8f4036e6db5b345412d6a2caa66f5a19`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--regcustom.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regcustom.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regcustom.h). SHA-256 `f5b61fb0879175e32efddf29366a1129871009b568a7577a9f439b7639956063`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--regguts.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regguts.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regguts.h). SHA-256 `65deb75d5f2ecb45e5c390366d5147f1422c4cc175316cd6f55747fae9c0794e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclIntDecls.h). SHA-256 `ef14c8b968a80d9d4b08b70419afbfcef4dc484d03fb5c3f6c729350e0c45a1e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--regex.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regex.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--regex.h). SHA-256 `1622265ce515dda616c930b0946bdf18f7271e9bbedab3d1d5657d13b67fdd7d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclInitScript.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclInitScript.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclInitScript.h). SHA-256 `4865d9e8aede714646e436164a4b6fec113ec3d79caf268abe13fe621d7f3e84`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPort.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPort.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPort.h). SHA-256 `9e061bf968300003b27bb6e4ad1b6e38dedc4146aad55d72786676c6831cb647`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclInt.h). SHA-256 `f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIntPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIntPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIntPlatDecls.h). SHA-256 `9e164bf2d564c2290f5ca3539150f992fae55a3751afb59cbd9dbe680e7eba44`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclCompile.h). SHA-256 `3d7d3b604522a9c5e673076fab8df53316951da1406892d03fe52dd678e2d0ec`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclDecls.h). SHA-256 `3e89c14101bbf1969da34f6a5fecc53c57fbb0cbd69137c0fe00a2750fcdb27e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclIO.h). SHA-256 `d85663574c38df6715b598957b2dfd31c713f8e99410842006519f5e3568d8da`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclPlatDecls.h). SHA-256 `8d7d5a1c6319aeed472f670ae0fb67b41d6595751079c6b67dc40cc9521f8e89`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclMath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclMath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclMath.h). SHA-256 `230e59a213702ebf098d9794e17b9b998db1d64712c17fd683d82a45bcf651c9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_map_prefix312-map-source-8.4.20-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.4.20/tclObj.c). SHA-256 `6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclUtil.c). SHA-256 `26a0bb5644999077e8a6a82a7ab1bbcf73787f889a35d7ede5369788c66d2e4c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclBasic.c). SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.4.20/generic/tclExecute.c). SHA-256 `970975c51cf8b78a291b92289cfeac8f023bd126c5d227ae5fa3c7d426a60c77`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclCompExpr.c` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclCompExpr.c](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--generic--tclCompExpr.c). SHA-256 `c4138e547b833c39d0c6da3bb4042fc7a7052a6146a2e7635aedcd53ac492514`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-original-inputs-tmp--receiver-logical-operand-truth-request465--expr-and-left.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-and-left.tcl](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-and-left.tcl). SHA-256 `4416144aef3d938549bd3dfe187115bff35296f61254eaaca53328afce705846`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-original-inputs-tmp--receiver-logical-operand-truth-request465--expr-and-right.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-and-right.tcl](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-and-right.tcl). SHA-256 `03f73e7e2523ea6ce21bcdea8e2266816bca96212eb00b799a7cdcbfb61768ad`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-tmp--receiver-expression-truth-request459--expr-conditional.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expr-conditional.tcl](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expr-conditional.tcl). SHA-256 `73870a9985ea4613d4ec20ddccaf3419b96f4f25f90479295b127f3e430ae19d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-tmp--receiver-expression-truth-request459--expr-not.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expr-not.tcl](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expr-not.tcl). SHA-256 `8b51ac1aa2cb3651d723780b7fbc4e9fb417f95965d423d163d73a16febd6fe3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-original-inputs-tmp--receiver-logical-operand-truth-request465--expr-or-left.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-or-left.tcl](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-or-left.tcl). SHA-256 `7df315aed9ca3277a7b4f34ecb361703563c5b24a21c541787aa5d315b33d004`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-original-inputs-tmp--receiver-logical-operand-truth-request465--expr-or-right.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-or-right.tcl](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/original-inputs/tmp--receiver-logical-operand-truth-request465--expr-or-right.tcl). SHA-256 `545d2551b84c7f65b027fd810bb34b8972c317bed76a4e4c037f10648e0f6b89`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-tmp--receiver-expression-truth-request459--expression-source.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expression-source.tcl](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--expression-source.tcl). SHA-256 `266a697aba660c60c2f35b01b3438ba3f9fee2f3c2108858ff22d84c7dec7517`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-tmp--receiver-expression-truth-request459--if.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--if.tcl](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--if.tcl). SHA-256 `ae874b930a6a01aede1cb532cd0cd04b76a313946372b30cfc172127bd2c28cb`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-tmp--receiver-capi-scalar-getter-request456--jim-version.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/tmp--receiver-capi-scalar-getter-request456--jim-version.tcl](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/tmp--receiver-capi-scalar-getter-request456--jim-version.tcl). SHA-256 `67c43f03e646543de96a1a97809fc0ab06a21cf2db5be74389b56938db38ec21`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_primitive_boolean_expression_truth_original-original-inputs-tmp--receiver-expression-truth-request459--while.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--while.tcl](../../../../rust/tcl-registry/tests/data/native_primitive_boolean_expression_truth_original/original-inputs/tmp--receiver-expression-truth-request459--while.tcl). SHA-256 `c5c3b0b07dc61b5589c25cf65f6c31f86fe26cecdbbbb948d197ce0212fe64bf`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclExecute.c). SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl8.5-tclCompExpr.c` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompExpr.c). SHA-256 `0a6d97e0800151eeb85e60635c9862ac111022a56d8535d38d8d2fd34fd779f5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c). SHA-256 `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl8.6-tclCompExpr.c` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompExpr.c). SHA-256 `970b54cc3b24299ff47e64ec6dccc14fdbffd6fdc9f0904170c2071bfd3f8ca0`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c). SHA-256 `0c42699cdcf610813ce77e2a6fec505f12d82a87b110950c92824fdf30d3c42a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl9.0-tclCompExpr.c` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl9.0/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl9.0/tclCompExpr.c). SHA-256 `f82f94056112b9c76292b384ddb0f84b1d4889d1c14c4776fa0929e02767ded1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c). SHA-256 `9dd4e9a080136f2ce36ee8f9acf4a30e770cc5fc417519468c785d76013151f4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl9.1-tclCompExpr.c` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl9.1/tclCompExpr.c](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl9.1/tclCompExpr.c). SHA-256 `16191591d5dd04ac05c3e3200799bf27167ec1d90c9985c3dbbed04e3c07d1b0`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_alias_error_extent279-request-jim.c` (source-anchor): [rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c](../../../../rust/tcl-registry/tests/data/native_jim_alias_error_extent279/request/jim.c). SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclBasic.c). SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/8.6.18/tclBasic.c). SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.0.4/tclBasic.c). SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c` (source-anchor): [rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c](../../../../rust/tcl-registry/tests/data/native_oo_original_collision249/request/source-anchors/9.1.0/tclBasic.c). SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--init.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--init.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--init.tcl). SHA-256 `a7f4388d1989f005872a47ca5d698aa06680aca21407462301972c5a6048d92b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--ldAout.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--ldAout.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--ldAout.tcl). SHA-256 `695157c85d3a27c435abd2998ba06fb24cad7b1e7ecb280fc1c60986749634f0`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--auto.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--auto.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--auto.tcl). SHA-256 `a04176d99af9264972f25f115bf3a354a47d6791b0fb65dc956698147454fd13`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--word.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--word.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--word.tcl). SHA-256 `c4b31e1a407db7566ab0f0b1e36eecc5e5d72dd9eac1916c5d69304e0df4c26e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--parray.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--parray.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--parray.tcl). SHA-256 `cff00c9471fa6941bca5a934ea331cc63ecf750c93c320f3cb378cd27c77b89a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--history.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--history.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--history.tcl). SHA-256 `6c7262b1ed5b376d84776d347e1d3ade7d565fabf1ac956a61649b95318a5d5b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--package.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--package.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--package.tcl). SHA-256 `3373e053b6b698e9aaf66542b97be199b59beb9a7eb73878238c0915a844343f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--safe.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--safe.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--safe.tcl). SHA-256 `c975b8db8a368d4eee415eaa2d9cfca0cf98445c314772f82a64ebde07bce906`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--platform.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--platform.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--platform.tcl). SHA-256 `d2c0e68e0b9d53faf7d9a34ecc49560b14eaae108eb25681335f51e231755c6e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--shell.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--shell.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--shell.tcl). SHA-256 `6611d96f69fff00d09b625786298b001c7b794147057a1f57163d910cf2b5d0a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--platform--pkgIndex.tcl). SHA-256 `fbb2ece354ab60a262509773cc9c28b77940e26af005963ea1402547d43fae8a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--http--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http--pkgIndex.tcl). SHA-256 `0e8c796d23795489ca9f710ee963f685a67ce6a0052ebcde4eefe40295221779`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--http--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http--http.tcl). SHA-256 `d9ce3119271af29ac600a3cbec9866be750a306c01c3d28897f655e3e9b35c46`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--msgcat.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--msgcat.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--msgcat.tcl). SHA-256 `21ed0584bad0589bad98c5684cea7c113ef0d0339599ee20fa45d835683eb64e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--msgcat--pkgIndex.tcl). SHA-256 `df9750e9a99622ca48727f45f32db3313d1d6487c3f92de60d730209708622c7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--reg--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--reg--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--reg--pkgIndex.tcl). SHA-256 `24561d73e6fced367e6a09b824afd107ef20de2d476425b15019dfa6130662df`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--pkgIndex.tcl). SHA-256 `420c4b3088c9dacd21bc348011cac61d7cb283b9bee78ae72eed764ab094651c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--http1.0--http.tcl). SHA-256 `88220b059956d3f331b29c514f0d4ad77fbd840efb27f0c2621510800a9b9094`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--dde--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--dde--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--dde--pkgIndex.tcl). SHA-256 `21668d84700b94eb650c902000b9490106267b444c632148dfcf5b60e3621fcd`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--optparse.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--optparse.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--optparse.tcl). SHA-256 `a7665fe9abcc7340f34e6470b94001c67d6c68824f214ed736dd27e8d62a8d32`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--opt--pkgIndex.tcl). SHA-256 `c4a5c0dc93486d7ac086313410983f34717e431128212a4e2c4dec82ae834f77`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--tcltest.tcl). SHA-256 `edda8af864f40c3d92c318472213497477616cc18089028c72c03852ecc9f587`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.4.20--library--tcltest--pkgIndex.tcl). SHA-256 `30d8ec3404ae35102274f991e84f7cdda6bb84181d60328ae50c2c6143ccd4de`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_class_source269-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.5.19/receipt.json). SHA-256 `5c357fa1bd8f44e02b0b08fca76640fecbe85297493b116cb8e8e0a78aa617e7`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tcl.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tcl.h). SHA-256 `c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-unix-libtcl8.5.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/libtcl8.5.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/libtcl8.5.a). SHA-256 `94313727ea507b1b2b00018062a0f57587458a3509c37f2e4b351dd58d0592dc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/unix/Makefile). SHA-256 `26ae775d2e4657ecfcb45421cc77c4c26170cb2a21985fbb002eb4791bc766a5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-8.5.19-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c). SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_event_original-source-8.5.19-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.5.19/tclInterp.c). SHA-256 `958e2492c5afe9e57a7776cd774ed995224d9c93402b02708ef8c6611588cc04`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclPkg.c). SHA-256 `2365436253e7777a7773f3fa00d9099929fae8406590cd2daebb2914a0a4f0b3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-8.5.19-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.5.19/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.5.19/provider.elf). SHA-256 `e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--unix--tclConfig.sh` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--unix--tclConfig.sh](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--unix--tclConfig.sh). SHA-256 `305b56696cc9b02162fef44ffcaa8f2a99b096aede7e53986d2a53acf7d8cac7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclFileSystem.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclFileSystem.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclFileSystem.h). SHA-256 `9a19f213feb6767646041a2c69da48e6c135215d9393360df195d0f1fe515f68`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclRegexp.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclRegexp.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclRegexp.h). SHA-256 `a2ad300cd59a4bda23994cb46c4ae253b62779391909209b3e52d9f9c52cabe3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMath.h). SHA-256 `97cbde63356a65eee09a9882279cb329876a92ec5995f079d574499b89655314`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathInt.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathInt.h). SHA-256 `035937f02a5bd42500ac7cc678a462af7178657a6932f890c4d63396f28869ca`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclTomMathDecls.h). SHA-256 `2fd434bd593b85b781272fbbd4efba6eafe53a3a7aeb5d3903aa09133d0bc642`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--regcustom.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regcustom.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regcustom.h). SHA-256 `77d71001cdb39fce6fecf99c1f48b53d006695aa11f116d013400f92514ac6fa`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--regguts.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regguts.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regguts.h). SHA-256 `f52f3ca1e5b0970312aa4005e03e0cdbc1e9512764889556b292029137639037`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tommath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tommath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tommath.h). SHA-256 `eb0576b12bd5a63d95f4b12706118c472d7a33ab70c92aa711a9deb193eab068`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclIntDecls.h). SHA-256 `e4efb75e3a479b8d816cba3eb4633a449276dc2ac2f1a86d2125cac32dfc0243`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--regex.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regex.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--regex.h). SHA-256 `edcc0d91cd9bce85078695c6ddb06a629b36933e06e079b550378fb7688a34e2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPort.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPort.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPort.h). SHA-256 `f0985f5ac3804a9ca460df2f788889343e09ef55a8cd4f6d305d8e717398b41f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_holder_routing205-source-inspection208-8.5.19-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.5.19/tclInt.h). SHA-256 `72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIntPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIntPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIntPlatDecls.h). SHA-256 `4e86797863c1beced0f250ef5d74119a36a2558e21528125195426eaa904806f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl8.5-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.5/tclCompile.h). SHA-256 `9ef1b2690de80b9c193d866d8ef8eb3271e57802c91a96f0f0a01f87c1d1a645`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclDecls.h). SHA-256 `3affefe8c9c5399ebdc9fcab7b2765aac0cfcb9d35c7f170e19f0c611e17b6bf`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclIO.h). SHA-256 `bb181558a40a0b942469db59b50b53828f6007626ad94ab9b30373b725eb606f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--generic--tclPlatDecls.h). SHA-256 `1d6d63ee65d5f149099fdd24589b061b788b6eca0238ae6383274a138c4999ce`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_write_error_lifetime226-failed-request225-original-source-tclObj.c` (observation): [rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_write_error_lifetime226/failed-request225/original-source/tclObj.c). SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.5.19/generic/tclUtil.c). SHA-256 `67be2e4aed576288b31069a98a1ddacf8354c87ca1a8c8e85e53421364a989cd`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--init.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--init.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--init.tcl). SHA-256 `21a5aad2ed6d69e15c032be72da55dcca8b56580c869e863d87caf2848e5c2b1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--auto.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--auto.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--auto.tcl). SHA-256 `f33b4e6ca8ac8a86ace39ad57628d7588ef04ec3d8d86c700f54ccba77b242fc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--word.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--word.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--word.tcl). SHA-256 `64585c5327b0710d31bff61c14564ff289acaad8743174f95544d8c04306d8c7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--clock.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--clock.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--clock.tcl). SHA-256 `71966a6cecd4d718b8b6286573bf50539c1d4bffad26a1126d056a5da48a66e4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tm.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tm.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tm.tcl). SHA-256 `5d86054b2ce7ecb7ad39a6a2ee7afc98816a837e9819ce7b7c31c19ba0b123cf`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--parray.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--parray.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--parray.tcl). SHA-256 `ebe5a2b4cbbcd7fd3f7a6f76d68d7856301db01b350c040942a7b806a46e0014`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--history.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--history.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--history.tcl). SHA-256 `f136e0db9e71468e4d9d93200cd2d04e6915d5546681bfeca6cb9a620ba648ba`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--package.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--package.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--package.tcl). SHA-256 `be3df25f0cf653c20b69784aee0fd719634d5421746b5b1141bc0592a59841c9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--safe.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--safe.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--safe.tcl). SHA-256 `953bc6cbf03a7ff492de59828c6d31a12d80b45873d85c03cb62a6099fed976c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--platform.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--platform.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--platform.tcl). SHA-256 `d4b3d9601454ea4828dff3be426c33fb845d005e98d2cc139dbb0d69cad3168b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--shell.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--shell.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--shell.tcl). SHA-256 `2f812a0550716b88930174a8ca245698427cd286680c0968558ae269ab52440d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--platform--pkgIndex.tcl). SHA-256 `1d808c59aaf46b7a24ea360246a5fa3824a510d703750eb234b702f8106c708c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--http--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--http--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--http--pkgIndex.tcl). SHA-256 `f33a19dafa35af4765cd1a6dd8557047a73a21b2b8a7288d52cec6bdf7cea6cc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--http--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--http--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--http--http.tcl). SHA-256 `425f41cbee04fe48b5803a77f1649257e4dcb475491c01f6d58704c3af4399c1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--msgcat.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--msgcat.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--msgcat.tcl). SHA-256 `2efa43679969abd5748f6e7d64408617396300b469e1ec9d0b6c1a7764c886e5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--msgcat--pkgIndex.tcl). SHA-256 `8989f8e8d7f96da5fd51f5271cd32aadc4fa5c23d7c4010838378d921c3d8d1e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--optparse.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--optparse.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--optparse.tcl). SHA-256 `217074e45fc877ceddb0eb10fca94fcf43dc235dd8dc4bd1c9b6ec3121ae726c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--opt--pkgIndex.tcl). SHA-256 `a088e549d18ade683273e31c004daa7e614642fe801afb3861eb85445250186b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--tcltest.tcl). SHA-256 `db4e5898291e7fd3cf522e9c9c5b714d81d6d148ba53ed2052073d12327f8d8d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.5.19--library--tcltest--pkgIndex.tcl). SHA-256 `29c57ad6fe369a9fed7d378471aec5a7bdd5bd153c57222f83485245a3f23e44`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_class_source269-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/8.6.18/receipt.json). SHA-256 `9a89b544c51fe11d8dc302119e7047e0cdc0dceb183572c743c430a88c155af1`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h). SHA-256 `aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a). SHA-256 `980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile). SHA-256 `8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-8.6.18-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c). SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-8.6.18-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/8.6.18/tclOOInfo.c). SHA-256 `309603344c3aeebe95832928b473a84ebc001b3d85c8489e15675448035de02b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_event_original-source-8.6.18-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/8.6.18/tclInterp.c). SHA-256 `b2597dfae723709bd21c6dfe9dd0cd4eb6d38f90ea082419b59a2aa2c00881ca`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclPkg.c). SHA-256 `83845b75b4da27d7ae67afd5b15135153d68557681cd9659f75d4d4e027135a2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-8.6.18-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.6.18/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/8.6.18/provider.elf). SHA-256 `9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--unix--tclConfig.sh` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--unix--tclConfig.sh](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--unix--tclConfig.sh). SHA-256 `beddc87cb1aa1dd414e2be975085bfa07d46e4a9822710e8974da16e8868dfd9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclFileSystem.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclFileSystem.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclFileSystem.h). SHA-256 `f2fd640c77879de707b770a95634d8899b6f1212d63a90c316dde64de9a78c97`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclRegexp.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclRegexp.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclRegexp.h). SHA-256 `e1a93d9734b8082903bd3de63d7750a170d5aea7444d81ae9703232c2e55b920`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMath.h). SHA-256 `31101dcbfb0f8943f4286f3ed2b3cc3dfe837573eca267aec3486951f89b5ac7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--regerrs.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regerrs.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regerrs.h). SHA-256 `43d6d65745e3849d564c4240adacec18139513709f782a778b4d56195096742f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathInt.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathInt.h). SHA-256 `5f209890cff6721e66373827126b1da389b4f5be97f850fdc2940abe34abdee9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclTomMathDecls.h). SHA-256 `385be9640cd9ecb085ce3fad840047a2aef54df464f815090e560536f04c8c9d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringRep.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringRep.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringRep.h). SHA-256 `13780ca356d16dbad4e773516ca27bbc5bf3689bb90193455d0061b2d12258e2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOInt.h). SHA-256 `9abdcd59f73d78b81c17edb2562686adbcda0afd44aa1dfa0ec5f9b2b7746f98`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--regcustom.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regcustom.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regcustom.h). SHA-256 `efd15b71c48740a47c31a7ccd546a07ab2e0aeee242e5d8594a3a9a96b52e69a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOODecls.h). SHA-256 `2ee41615f2f0eaa65d58825a8768ea011dc8491d33ee245f3a5062e721d41e54`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclParse.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclParse.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclParse.h). SHA-256 `60213f5d42a983347fbb4ada32e7210b19c911afbfe354cdf17273c53a078a90`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--regguts.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regguts.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regguts.h). SHA-256 `dd28b58cddb9367fe49cd45795f076dd4c85473a3afbc504969597fe91c5e809`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclIntDecls.h). SHA-256 `dd303cab02a101f96109c96f626eb91fa5576fe21294a9d80816b06a8014ca26`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--regex.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regex.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--regex.h). SHA-256 `4009629c7a63503b0203600246214ea3ac4bde36bb05bc7c93156cd2f63201d7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPort.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPort.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPort.h). SHA-256 `b79fd3ba585ea82ad991ccc994a2bb2f284720ef055560936aa6cbab7b9e9e25`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_holder_routing205-source-inspection208-8.6.18-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.6.18/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/8.6.18/tclInt.h). SHA-256 `e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIntPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIntPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIntPlatDecls.h). SHA-256 `94469685c0c8bbb6cf1c63bfa50af29b66a8801096c4e18a760a50e76f779d00`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_expression_compiler_name_effects-sources-tcl8.6-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_expression_compiler_name_effects/sources/tcl8.6/tclCompile.h). SHA-256 `be854eab265b25091f3e5a9b8d268d3aeca520d12382f135e3b3ddede49e6ed9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOIntDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOOIntDecls.h). SHA-256 `09bf8c566b159f35b2a1f9a4850fadf12a950bf8467ed44135a16342239f347b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclDecls.h). SHA-256 `b4072e658d9026bd81a0706e9ff02bf40f8f1f137bcd882d2a23dd25b335a144`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclIO.h). SHA-256 `aad5e9f1875708c1c98bdf3e85559604b9bde2e12920b44620d99188337baa5a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringTrim.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringTrim.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclStringTrim.h). SHA-256 `4dd41d4d4ff4195dcacbeedb033b8c4eb7f919c1e2047ad070eb01764bfc70fc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclOO.h). SHA-256 `64ebe853d4ce50f54292c95e47ed91dc6fb006fba335006a15df2fc2cec58ea6`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--generic--tclPlatDecls.h). SHA-256 `9498557f03c1a6fa8b6f6c3b16988cba9efda86bbed8d1c6b4851b09db0da7de`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.6.18/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/8.6.18/tclObj.c). SHA-256 `7a7ef8ec85c74581129e8f3bef939e3191a432dcb021e0a16f1cf9971049f90d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-8.6.18-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/8.6.18/generic/tclUtil.c). SHA-256 `452b96a108330de9bf79c52a6266634127b468b704512ffb93e0dc66685df372`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--init.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--init.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--init.tcl). SHA-256 `513985a7db39509e30f1bec641284dff77db22aa44fe9a0e6191eb63dcc28f35`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--auto.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--auto.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--auto.tcl). SHA-256 `67be04b5aa4c0845f8c5143171acf31ad6b3f23eb1a0be98133133e8abc9822b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--word.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--word.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--word.tcl). SHA-256 `f18e8ae29aa8ed2be2cbb5568b4243f654f7fb0aa2066e16977863ee1415b9bc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--clock.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--clock.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--clock.tcl). SHA-256 `3fd236d0d8cd613dee7e4ece236396bd424bc81673e9bd403bee4695d0ff5c75`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tm.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tm.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tm.tcl). SHA-256 `0b102f22297356a9873a81224962fde4cbd4b59889c54b1d0632695701eb10fe`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--history.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--history.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--history.tcl). SHA-256 `e2eb8bdf39732ca48173e61d9ada4f0d095ddaa3ed1fcce21c0a520c75493d8f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--package.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--package.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--package.tcl). SHA-256 `0227624ef7ab99d60f0aa76e74419c34267a322db36cf945e66ed705525eedc0`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--safe.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--safe.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--safe.tcl). SHA-256 `bd70548ea2090135918f2e61a39bfdd6163ec8cef82f400ce220a1edc4bedd6c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--platform.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--platform.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--platform.tcl). SHA-256 `ec8cca09e045ecc3d61d7aa2a336f4c6366a5bc0e8207feca37eab38fc0cd292`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--shell.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--shell.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--shell.tcl). SHA-256 `3e9a139b7d2d768a8b94327feb402c4fc7afeda156e46ab2d22e8c40266142a5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--platform--pkgIndex.tcl). SHA-256 `e744fb8cd80fa0d40eb7fd96b0190e4d441376eeb4dca23efc16ae1b5152c0ad`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--http--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http--pkgIndex.tcl). SHA-256 `0345d47af9786eb20eab98519140306b924f32cd7c685d34bc40d3b6f1f6f27e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--http--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http--http.tcl). SHA-256 `803ca646b09304bc21b79f3c5618ac1359a0bc34a2762d40dea933c23e4f7ae6`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--msgcat.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--msgcat.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--msgcat.tcl). SHA-256 `5732951b02b45c3702c2f9ee7c43deabfcfe0aaae893f89bcc59e32bd9444ce3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--msgcat--pkgIndex.tcl). SHA-256 `3d391b5c9219666880499e6e5c87edb05e64c5d25fe2c0e66874fb0271f2e3d1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--reg--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--reg--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--reg--pkgIndex.tcl). SHA-256 `5a3a93234a418f3a8e1814dd8e6d95344045a6d3a4e62af6a8fad2c48d2aa1ce`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--http1.0--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http1.0--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--http1.0--http.tcl). SHA-256 `f115d9d2be5886175fe610484eaac8e17d9feb1c71d419af66c6e31b0c215523`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--dde--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--dde--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--dde--pkgIndex.tcl). SHA-256 `c2e7fd1110cf83cf9b8c5dcc823e02955576615dd67d8f912c151ced735c0d8f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--optparse.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--optparse.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--optparse.tcl). SHA-256 `04f4b2f34d959cff47c6502a137d15ac9dc4f54d4a6e78478f64c8c7e14849a8`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--opt--pkgIndex.tcl). SHA-256 `c6ef00c55362782a3b8c185e84413225dde64cec690fb324158d7028c53c3745`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--tcltest.tcl). SHA-256 `03da6ebf2e314e6c931430761cc47cf586ded9210f9c5afed804858a78034e67`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl8.6.18--library--tcltest--pkgIndex.tcl). SHA-256 `26cbf5ae8928a273e8404ddb5d92c0d086eb1cb64369db9365930388feecd199`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_class_source269-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.0.4/receipt.json). SHA-256 `adcdc0557f5279db1a62ff25ff750400f64280c9c6d0151614ed663aa8fe1fc3`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h). SHA-256 `eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a). SHA-256 `dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile). SHA-256 `69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-9.0.4-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c). SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-9.0.4-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.0.4/tclOOInfo.c). SHA-256 `a1ed9fa857c89ba0cc5eea5a6f4e3be82783130823d5182c8e4a4abc5073158b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_event_original-source-9.0.4-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.0.4/tclInterp.c). SHA-256 `5c944998c2f89689ef9f591983ea7b0e322dd40756fe5baca641e2258969b125`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclPkg.c). SHA-256 `c89585f9d38a913ce03b9a700987cfe18cb39dec2a24293336eb85b67cd67c8c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-9.0.4-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.0.4/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.0.4/provider.elf). SHA-256 `f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--unix--tclConfig.sh` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--unix--tclConfig.sh](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--unix--tclConfig.sh). SHA-256 `7e808c1d9a006c98ca7b819a43a206ef9e0527ec2ea8dfbb9a1d348659a2ebfe`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclFileSystem.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclFileSystem.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclFileSystem.h). SHA-256 `1695ab68cb8ebdb9d83b7a3beefdfd585a75d2d17aab2ded3d09f6061b2b985a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclRegexp.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclRegexp.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclRegexp.h). SHA-256 `89c6bdb4c27fad330d4ae893593c7f79d21cf30bcaee082e73d12fe78adb24f4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMath.h). SHA-256 `3729c75b90b82672c19f432cbd127abbd01692797bfbd79f5ca8003d7e38bc12`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--regerrs.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regerrs.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regerrs.h). SHA-256 `9ab668ff9045ca52586613bc5a3a427e56350c1a87023420cc49dd5e8441f3b4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDate.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDate.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDate.h). SHA-256 `4d9dd452eb3504583ebc05eeae6c2a3e4097ff6e11f230220f39b2a2adbd03e8`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMathDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMathDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclTomMathDecls.h). SHA-256 `29e1731c2962563276361069b818052d0937f8b0b302333b1fa7789ee4f421d4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStringRep.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStringRep.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStringRep.h). SHA-256 `9c1ad75f64b876f1018e3e06a613d00e62d404ef68285ce46438340cf80f6da2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOInt.h). SHA-256 `a386548a74e6c88d002cb137bdf323e1f72f20ad4290988ce98b90355dade929`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--regcustom.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regcustom.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regcustom.h). SHA-256 `02b27b3af5bdacb1113bf7eea5439fdcff8d87884cbb2ee9a4a0425d7d52a016`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOODecls.h). SHA-256 `d7c75dce4c4fc02ef5ba85c49933113dc8bcdcd49fde00a982f7394adee80ab2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclParse.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclParse.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclParse.h). SHA-256 `e0293a2067998484d13e7f3911781eac96d63fa96b1ec2743e259c970e1c9212`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--regguts.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regguts.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regguts.h). SHA-256 `d338568fe56ba78d0431b60582bc0d1c4e7ee21eb4bb6164d7e274c7bcea6eda`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclIntDecls.h). SHA-256 `96897750658753b43b3de554682bec7249c92e858dbe437eab7de0e26aba9a6a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--regex.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regex.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--regex.h). SHA-256 `c8d2b752b8acc928d1e8fb11b8f58e3b7e18dbd623c50de98c573cf754198e7c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPort.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPort.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPort.h). SHA-256 `db26aa99449e53c7cc9700adfc4283b0b84428e6fd2712d391bff30d910076bf`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_holder_routing205-source-inspection208-9.0.4-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.0.4/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.0.4/tclInt.h). SHA-256 `f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIntPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIntPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIntPlatDecls.h). SHA-256 `0e38502772ecdf2d3bacad02365f06517f45627eef388068bd70645283895d02`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclCompile.h). SHA-256 `22f512199d2e57370496d8a6713921d9ac57538497a3a64966fa1dee7a6a5dc5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStrIdxTree.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStrIdxTree.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclStrIdxTree.h). SHA-256 `f7fdc28afc3bb32a2f78711837f5230a1808fa5d9e32d75bd9032185304998c5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOIntDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOOIntDecls.h). SHA-256 `736bf32cb065781e28fab4c041c28626a723ef045579584ea98e289523043369`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclDecls.h). SHA-256 `28ad708ea0738fbf8bfb13dcde87ea8064220982d29b8c082df87c7987ff2c70`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclIO.h). SHA-256 `6e670762905b13ada9295938a3a80a61d503468fe0fb4752ddbee5b9223ead56`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclOO.h). SHA-256 `8381d5980a521ae1907d6d17423e6e45f3f37f75634dc20f1214b7e1ea1170f3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--generic--tclPlatDecls.h). SHA-256 `013fb34b4b876732a421dcd56009292e9b71b768be4b57a392599e347ea8f80d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.0.4/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.0.4/tclObj.c). SHA-256 `91a390bd24fbe71108108eeccf9955df0389f0f81dff24f303c32344469e8d4a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.0.4-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.0.4/generic/tclUtil.c). SHA-256 `13df44c0c262d3445b5c5db138d14c6e45e4993b076387f6e4f272f89d62bcf4`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--readfile.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--readfile.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--readfile.tcl). SHA-256 `eff41b30c70251d068e24d1500ad504404d692f8d48ab74cf8f99bb4dba64414`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--init.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--init.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--init.tcl). SHA-256 `8c5907793b2b8aa05e8d8cf39c0cb7f033c01ca1b8be683e7159774d1a01357a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--auto.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--auto.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--auto.tcl). SHA-256 `629dc86958aa1c5f8697a638f296c706458e7a3418d2d81a2e81ce945f3828c6`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--word.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--word.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--word.tcl). SHA-256 `3a481eacbcca1c611843cdc387ac51be0b8ac98a994cbe8fa43091fee35b207e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--clock.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--clock.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--clock.tcl). SHA-256 `6b286804649120c7c249d8c2941cee8e1da59211132d985ad97b8ab196073546`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--tm.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--tm.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--tm.tcl). SHA-256 `7c1de0bc512b362c26d489bd332f3d883041d98c2ed1cc99b4d64cd97e646786`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--icu.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--icu.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--icu.tcl). SHA-256 `bc125ef0732af6e67367e324c0b3de87e03f04f48c72c42fbb8f77b7b5849a8d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--parray.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--parray.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--parray.tcl). SHA-256 `b8677236d4e687e686426718e78d3984146d2f318e5b34cbd43824206e85a40a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--writefile.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--writefile.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--writefile.tcl). SHA-256 `b454723873db2e587ac1323bcebff2e8929f5dc98a6fe001ed34fcaf0c2aa320`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--install.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--install.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--install.tcl). SHA-256 `5508d98832c4f3cf52b297511206c5dd020111f1dfbe358eee023ee1bff3e012`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--history.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--history.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--history.tcl). SHA-256 `ca38bb6652bbc562a3a82d2919f4e900cc5deedfb3582fa4503c573754ccf007`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--package.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--package.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--package.tcl). SHA-256 `473bf551d3156a144d6483ce730c72add640168a3ab426b3ee0b122f6d06a96e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--safe.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--safe.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--safe.tcl). SHA-256 `0d3e8ddb147d713206139b7c4c915f11d0a53aebeee3a5ca3e7aebf26854e238`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--foreachline.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--foreachline.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--foreachline.tcl). SHA-256 `5073f9015583f00b25354a7877f55c68d93674ae11384ee042e3b094a8a900e8`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--platform.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--platform.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--platform.tcl). SHA-256 `4af798475ffe6a0af32f0202a7cbc7b07b59de5ddcd7f89fcbdd1b07ffe5f6e1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--shell.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--shell.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--platform--shell.tcl). SHA-256 `54337e9db387e5f3c0a714ae225d3d915da83751d5be4a9e177e9b726d3f92d6`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--http--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--http--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--http--pkgIndex.tcl). SHA-256 `83bb9de4d5ce47be75668bd196b945407bfd0ec8a3fbf425f137fee723d25742`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--http--http.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--http--http.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--http--http.tcl). SHA-256 `56372f15e8f73234b7fb2b68f597d5a56a42d995416cdf65fe0c2da1c114e5a9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--msgcat.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--msgcat.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--msgcat.tcl). SHA-256 `80301481addb70362ffd4eb1c301471a41e59987d39dbbc2a6096ddfb415071e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--msgcat--pkgIndex.tcl). SHA-256 `6b7f3bf260c40ecf823f6fb7246ef3b833597e97bfd0e6963aa117ae196cd602`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--idna.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--idna.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--idna.tcl). SHA-256 `fa329853cd186ea405f4bec122913f1e4286c54488d0ffd445f7f120c71583fe`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--pkgIndex.tcl). SHA-256 `7d6f5e0236fad3b60cf5b7b10fa5629effcebead382083a7d9543c9526095d1f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--cookiejar.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--cookiejar.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--cookiejar--cookiejar.tcl). SHA-256 `9d0811415b98c15bbb5c8f86ec3b92e812dac61e3f4d564f7e0dd0b066ba632a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--registry--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--registry--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--registry--pkgIndex.tcl). SHA-256 `f485327543f043f62c1ad12416d3c959b98aa2b77320503cb2db0e16e773fb78`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--dde--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--dde--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--dde--pkgIndex.tcl). SHA-256 `f140827f677c522b3dce28b61609142da79c565e43ce176141c36766bc95265e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--optparse.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--optparse.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--optparse.tcl). SHA-256 `3d14ecf300f9fdd21875e22ce348988f852c177c4bf25fb6a7f37ba4be9fca73`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.0.4--library--opt--pkgIndex.tcl). SHA-256 `9eaa82aeb3d308b655b69660380829c54a6bbc011280667ac8cf0e5882132846`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_class_source269-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/9.1.0/receipt.json). SHA-256 `496e47fce5ab0c75d2c56654c7b242652d257dfd2f049869934e5190c6671bb8`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h). SHA-256 `30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a). SHA-256 `513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile). SHA-256 `c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-9.1.0-tclCmdIL.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c). SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-9.1.0-tclOOInfo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclOOInfo.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/9.1.0/tclOOInfo.c). SHA-256 `5f46472c0039bfd89a5ed06aba1df376942136c6ddf02d5c6a0b2cf0bce2a5a9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_event_original-source-9.1.0-tclInterp.c` (source-anchor): [rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c](../../../../rust/tcl-registry/tests/data/native_event_original/source/9.1.0/tclInterp.c). SHA-256 `f7880cfc3cd9787502134b22832475df5ebad0432e66c8fae2281ddd98abe59d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclPkg.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclPkg.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclPkg.c). SHA-256 `b21d3d4578ac6ad532f53782a8c2aa6d08087e9ecc6638d1b6850b1ca2aca684`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-9.1.0-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.1.0/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/9.1.0/provider.elf). SHA-256 `2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--unix--tclConfig.sh` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--unix--tclConfig.sh](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--unix--tclConfig.sh). SHA-256 `3151ba756040c3f86a08eb4a249c26f71d6f2d600b3f7425385c9434c1d41eb2`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclFileSystem.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclFileSystem.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclFileSystem.h). SHA-256 `30b8c2e274920082eaf973ac56bb42db809987d7faf7fcb463cfeb3db282f701`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclRegexp.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclRegexp.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclRegexp.h). SHA-256 `38413b65994af164c0280a81b975f2f1ef0c843b3c962faab16ef551b904fce0`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMath.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMath.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMath.h). SHA-256 `39983e6580739ec6a1e488dcfcbd869b75fbe4f9b6dfa3d091f79fa621ff2314`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDate.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDate.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDate.h). SHA-256 `9c8dd3591520af6de23baf9922701d88cf94db5aef398493342878131da7ed95`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMathDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMathDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclTomMathDecls.h). SHA-256 `901d07c0c7b414bde524d27b312fb1ca1083798d89ede1a68b6d01c33dcba7df`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringRep.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringRep.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringRep.h). SHA-256 `eec19ce4eb14b12bd27bbc3e4c593ee5f4936f795ef72cdb6991835fb1104729`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOInt.h). SHA-256 `d05b67a6bf4832f3b94013caab7c6b7d6939982f077e53ff98b7c40c51ac40e7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--regcustom.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regcustom.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regcustom.h). SHA-256 `0f86e38102205b0f8c4b56c202768c1442248fe0d988e3d3182ba9dce1e80d77`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOODecls.h). SHA-256 `7a5a73ea4029bb71922d4ee130d912e57057d4dee3408dfa8e0650f38e62333b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--regguts.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regguts.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regguts.h). SHA-256 `880e2b8e47aca98f31a4dea8f40df0132d6980bebe85e6695238c62f9b3cc268`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclIntDecls.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclIntDecls.h). SHA-256 `78d549e7b85ed9e91df2d44e62db9f2a376b55deb98c55edbbbe53396d45b502`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--regex.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regex.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--regex.h). SHA-256 `7695b69c11216847cc76af0663d26a9428f7cf0e9a60bb2c1adb9fa3aab858db`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPort.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPort.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPort.h). SHA-256 `aa7bee90fe17e0b4785d0909fd89529214c1c7b5b552fb88ef4513f61fd30f06`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_holder_routing205-source-inspection208-9.1.0-tclInt.h` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.1.0/tclInt.h](../../../../rust/tcl-registry/tests/data/native_ensemble_holder_routing205/source-inspection208/9.1.0/tclInt.h). SHA-256 `fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIntPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIntPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIntPlatDecls.h). SHA-256 `096fed7182d04d1a0fc192548fb8754eb0a218096205b685454f1f95d3903e39`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclCompile.h` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.h](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclCompile.h). SHA-256 `70203ece61377d4cf22a1eae4348d6216bf77372fd7f8a49274f05d8c546d44e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStrIdxTree.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStrIdxTree.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStrIdxTree.h). SHA-256 `56e4f5758d77f67bc95feb768a602d1269edad5ac51e35722d65ce8ef3299300`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOIntDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOIntDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOOIntDecls.h). SHA-256 `dd464d695945eb9890ae434f8598dd0ba20ce06cd04ae1d4fd19fd77f1ba9f99`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclDecls.h). SHA-256 `590a2f27f095e7150bfb648f43d00c9a9ec5dbb2b936e413b551f663dc972142`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclIO.h). SHA-256 `800db2da1c1a41081e664ab761cd1d620642ce8c4669edf10c68df3cc0927e24`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringTrim.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringTrim.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclStringTrim.h). SHA-256 `347a563d9391cc50913117d97f25c3a3a9049d8f86916c5cedfa038439fbe507`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclOO.h). SHA-256 `8fa650a6dea982840ad237867e1e69b581b0ab9445bcd82c80bb1b25f2d3260b`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPlatDecls.h` (source-anchor): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPlatDecls.h](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--generic--tclPlatDecls.h). SHA-256 `09b4a4fef3c23a6af2a55c2864736587ecfd9e8ccebe28cb75e95b42417e007c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.1.0/tclObj.c](../../../../rust/tcl-registry/tests/data/native_ensemble_map_prefix312/map-source/9.1.0/tclObj.c). SHA-256 `58c041dbf20bba3c8fef5782969c661d4d60a6ee0bc915f30e55c19cedb31976`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-9.1.0-generic-tclUtil.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclUtil.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/9.1.0/generic/tclUtil.c). SHA-256 `509c893e8aef8f1ae70d0d1e279bbefc60baa758a56d1be184421794f6868e64`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--init.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--init.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--init.tcl). SHA-256 `636818d3e4071d28ffb610c86d895cb4f0cff57ddf7d8523dabf1feed821ca9f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--autoexecok.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--autoexecok.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--autoexecok.tcl). SHA-256 `92c9f7d88e023844fd51d400c41144ad8d71f99099c896044d5693b190e3e3fa`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--auto.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--auto.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--auto.tcl). SHA-256 `1953e3830f90106aa322cfc66644cc36ae39f8350bf214f07fa4d75afd80da02`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--clock.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--clock.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--clock.tcl). SHA-256 `f1dc4c6b3bdac23e08b34af6a0fb4b04103d875d2698064d7490151617275d59`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--icu.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--icu.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--icu.tcl). SHA-256 `4ef7e224231e5430270ae03eebf54d3d0e8ca6b976cb8d8b2e7e6bf9147eaefe`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--writefile.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--writefile.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--writefile.tcl). SHA-256 `b5e9def32c99ee9b2199b46d3e37e7d075091d7e3c42921c754ee9e18cfff233`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--package.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--package.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--package.tcl). SHA-256 `a1b01ef4a09299dc30efcf3cfd647a3b99efc945bf8f1980a43d9fc43f075b26`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_deep_import_chain_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--safe.tcl` (provider): [rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--safe.tcl](../../../../rust/tcl-registry/tests/data/native_deep_import_chain_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--safe.tcl). SHA-256 `02af9afd6f876e6432bd67084b2e8983213ba33cba036bb4d78cca9072b3de96`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--platform.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--platform.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--platform.tcl). SHA-256 `c1a9ceabcd60f33cd80a1187916ded8df6e319ded07347bbc7c7255f08c87442`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--platform--pkgIndex.tcl). SHA-256 `8dd8f2d040863b062283f9e569502e014cac14a0989175e553ad5bc181fd9361`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--msgcat.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--msgcat.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--msgcat.tcl). SHA-256 `446a2dad2b197c17cd6ff4b638b1fbc5f93a5337548330c6bc6ac6886058b7eb`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--msgcat--pkgIndex.tcl). SHA-256 `17c82510641fa0048bc1e8b3cc47e2f18cac46e594f0171bdc3100c6c09a5221`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--idna.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--idna.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--idna.tcl). SHA-256 `c0a3df8e2d0f4681be9e4078e56d294406073131d2be0cd3d0b63b20fa0c4feb`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--cookiejar--pkgIndex.tcl). SHA-256 `9a13ce96018100685b26aa450f5a8cbce925ac70d5c1bdb2b323259c9956a5f7`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--registry--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--registry--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--registry--pkgIndex.tcl). SHA-256 `c61f803073c3183a49d5524221c05f295dee6cbef57272ceacc8ee5cc0d54c2c`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--dde--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--dde--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--dde--pkgIndex.tcl). SHA-256 `c3a195332dd743956eb38ebc22f1873eeea06501c2cd3ba91cee3838eb62da5f`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--opt--optparse.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--opt--optparse.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--opt--optparse.tcl). SHA-256 `49a8824f7d1d44e1ae5f1a0b9f3e8c363d89014e2e31bcdf1ae7045d698ff512`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--tcltest.tcl). SHA-256 `0113aee4c0edbd2d6f0235693d24dc896a7ad30f8eb30f255ef24a1d2982d311`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_tcltest_alias_caller_frame_original-original-inputs-workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--pkgIndex.tcl` (source-anchor): [rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--pkgIndex.tcl](../../../../rust/tcl-registry/tests/data/native_tcltest_alias_caller_frame_original/original-inputs/workspace--tcl-lsp--tmp--tcl9.1.0--library--tcltest--pkgIndex.tcl). SHA-256 `7372fd028e8165994e6c1e0ee2e3f3f7551ff75def535a414443e9e6c0138c65`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_jim_class_source269-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_jim_class_source269/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_class_source269/jim/receipt.json). SHA-256 `006a4501536c0ae6e1919ef33f8cfcad38ebd8698b965f90abfcd6d364e32b54`. Unchanged prior build input receipt; exact current CLI and complete required input pins match, with no invented build history.
- `native_jim_core_alias_prefix273-jim.h` (source-anchor): [rust/tcl-registry/tests/data/native_jim_core_alias_prefix273/jim.h](../../../../rust/tcl-registry/tests/data/native_jim_core_alias_prefix273/jim.h). SHA-256 `d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-jim-libjim.a` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/libjim.a](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/libjim.a). SHA-256 `a692b833f5b8f3351c1dc5911810aadf4c5f0ff4ca795ee6b802e6550e57b3da`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-jim-Makefile` (provider): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/Makefile](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/Makefile). SHA-256 `983469f71a073b757f3ce8869ebf846e7fbb358abd0e5d5d73b07796e366ce77`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_inventory_original-sources-jim-jim-subcmd.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim-subcmd.c](../../../../rust/tcl-registry/tests/data/native_info_inventory_original/sources/jim/jim-subcmd.c). SHA-256 `80f274c72d403c5d7906e22da58cc8f79fd7541e42f34e90e34cc7c5c5248ecb`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_child_alias_inventory298-jim-interp.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_alias_inventory298/jim-interp.c](../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim-interp.c). SHA-256 `6913a5abf6518b9bdf6b953130ae006e4369207baab2d70bd58b4a55bd0edee9`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-jim-oo.tcl` (input): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/oo.tcl](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/oo.tcl). SHA-256 `42f6b0150881a8d286c1ee9a9b30bdf1b06cf39850c576d9dfc104ef3c0eeef5`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_info_commands_literal_original-original-inputs-jim-_oo.c` (source-anchor): [rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_oo.c](../../../../rust/tcl-registry/tests/data/native_info_commands_literal_original/original-inputs/jim/_oo.c). SHA-256 `a3f26330db47cdfd64377059db2a11b5fcc53d3ae112dfb390686b4f7836eca3`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_child_bootstrap_context303-source-_load-static-exts.c` (source-anchor): [rust/tcl-registry/tests/data/native_child_bootstrap_context303/source/_load-static-exts.c](../../../../rust/tcl-registry/tests/data/native_child_bootstrap_context303/source/_load-static-exts.c). SHA-256 `69742674664cfa4ca43495d61075ee681b1c55ed9215eb27ebd2902b69d66979`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_return_provider_executable_audit387-jim-provider.elf` (provider): [rust/tcl-registry/tests/data/native_return_provider_executable_audit387/jim/provider.elf](../../../../rust/tcl-registry/tests/data/native_return_provider_executable_audit387/jim/provider.elf). SHA-256 `e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `rust-tcl-test-support-tests-data-native_resolution_reference_builds-jim-config.h` (input): [rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-config.h](../../../../rust/tcl-test-support/tests/data/native_resolution_reference_builds/jim-config.h). SHA-256 `d092046fe07c2ec14b77da74304135da24725d766e1c9fd74ee32eeb07702221`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_capi_scalar_publication_original-original-inputs-workspace--.proofs--native-providers--jimtcl--config.log` (provider): [rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--.proofs--native-providers--jimtcl--config.log](../../../../rust/tcl-registry/tests/data/native_capi_scalar_publication_original/original-inputs/workspace--.proofs--native-providers--jimtcl--config.log). SHA-256 `050187a330047724acddbcb98543b7b5e2168b3c3ea5fdf4b6fe8c0b6a245b7e`. Exact required source/header/configuration/archive/compiler/dependency/CLI input recorded checked before and after; current full bytes independently verified. Byte-identical official evidence is reused.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-Tcl_GetBooleanFromObj-1422-1439.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_GetBooleanFromObj-1422-1439.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_GetBooleanFromObj-1422-1439.c). SHA-256 `f7d0675d35b15c64cd961b8d01eb0a8c3d0daab6c4f088fc1b5a94ceb6b9358f`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-Tcl_ExprBooleanObj-5621-5642.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_ExprBooleanObj-5621-5642.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_ExprBooleanObj-5621-5642.c). SHA-256 `7664e6948ecb726bccb6e774b435a0d1cd7ac50f439f2d73057f2f44790b2111`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-Tcl_IfObjCmd-188-303.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_IfObjCmd-188-303.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_IfObjCmd-188-303.c). SHA-256 `64b1767c16fe5bda50c2994232f2d0634d85c64b66c5a49382ecb11a4133d21d`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-Tcl_EvalEx-3925-3939.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_EvalEx-3925-3939.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_EvalEx-3925-3939.c). SHA-256 `a5b49685933d9b08ab91986de95912ed02f1a4d109e8ceb7a85c1c1ccfc0c129`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-Tcl_EvalObjEx-4988-5002.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_EvalObjEx-4988-5002.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/Tcl_EvalObjEx-4988-5002.c). SHA-256 `40d28485d47f0e93583873079da669382b7ab049c05e243f866ef4dce9ce830e`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-conditional-jump-source-window-2270-2369.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/conditional-jump-source-window-2270-2369.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/conditional-jump-source-window-2270-2369.c). SHA-256 `09a4085311dcd6d29bd53ca94bf605799ac088f1e19cdf9b939a82b62dc9790e`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-logical-not-source-window-3765-3864.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/logical-not-source-window-3765-3864.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/logical-not-source-window-3765-3864.c). SHA-256 `9437d9d8659b1ea31859a655fa632e0ad318d68b9ea38bc2997462f86d81adf9`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.4.20-logical-AND-OR-source-window-2332-2481.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/logical-AND-OR-source-window-2332-2481.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.4.20/logical-AND-OR-source-window-2332-2481.c). SHA-256 `ce0254c53912ce24467494ff711edd834a6e343010ed72cdab264df1c57bc75b`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-Tcl_GetBooleanFromObj-1871-1915.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_GetBooleanFromObj-1871-1915.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_GetBooleanFromObj-1871-1915.c). SHA-256 `bdb2aa20ed599e2defef802f7b5e1c812c412dbddec2dba2009f98eaa595e91b`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-Tcl_ExprBooleanObj-5585-5601.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_ExprBooleanObj-5585-5601.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_ExprBooleanObj-5585-5601.c). SHA-256 `7098a0fa7a6cf0b659622ba24066f1590c8b6265d1a68ecd9ec7ad41c137fb33`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-Tcl_IfObjCmd-203-309.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_IfObjCmd-203-309.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_IfObjCmd-203-309.c). SHA-256 `e1c22a43638aa885ed8950fc0b783766a6e55e59ac8a03a92ef0b902ff411cb2`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-Tcl_EvalEx-4079-4091.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_EvalEx-4079-4091.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_EvalEx-4079-4091.c). SHA-256 `b2d8a2c66f34819f51d7f07a57d34111d47895effa1144b84038710502d52700`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-Tcl_EvalObjEx-5076-5086.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_EvalObjEx-5076-5086.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/Tcl_EvalObjEx-5076-5086.c). SHA-256 `b5306fbe56f0b6a0a1c28bb8843b0d1bfb00d22107bb1225d62f3035e8bdc5bc`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-conditional-jump-source-window-3595-3694.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/conditional-jump-source-window-3595-3694.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/conditional-jump-source-window-3595-3694.c). SHA-256 `cbc7d8bc02497dbd1569daa2bfe23da9c62d67a5de16645b13a88a37dead0436`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-logical-not-source-window-6275-6374.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/logical-not-source-window-6275-6374.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/logical-not-source-window-6275-6374.c). SHA-256 `c381e2a10b93c58ceaa440b408be22739e5b0519f78a6b515eb1926a96cc30ea`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.5.19-logical-AND-OR-source-window-3680-3829.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/logical-AND-OR-source-window-3680-3829.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.5.19/logical-AND-OR-source-window-3680-3829.c). SHA-256 `3d3fc92b282dae892438d9d526ee340f4b057b48deb0011cd55218d56774f8e3`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-Tcl_GetBooleanFromObj-1885-1929.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_GetBooleanFromObj-1885-1929.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_GetBooleanFromObj-1885-1929.c). SHA-256 `23411a5f88b660770178b2a3bf77ecb4482ddce156edc5fef7688c862a95de29`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-Tcl_ExprBooleanObj-6608-6624.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_ExprBooleanObj-6608-6624.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_ExprBooleanObj-6608-6624.c). SHA-256 `cbd302a40f3b1c68f7f190ae578da2731f8ed38195ebba35eb02e207c710d2c1`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-Tcl_IfObjCmd-205-212.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_IfObjCmd-205-212.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_IfObjCmd-205-212.c). SHA-256 `faccb69addf75dfcdb98230ab8b8a237d9c66ef76f185f3e05d0d4c4e8066ef8`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-Tcl_EvalEx-5065-5077.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_EvalEx-5065-5077.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_EvalEx-5065-5077.c). SHA-256 `0abaf933d18625dee4a5918aad3aa1fe8d9282c5874be165f95e440c496f2b41`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-Tcl_EvalObjEx-6077-6087.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_EvalObjEx-6077-6087.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/Tcl_EvalObjEx-6077-6087.c). SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-conditional-jump-source-window-4574-4673.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/conditional-jump-source-window-4574-4673.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/conditional-jump-source-window-4574-4673.c). SHA-256 `10c0df04b591090e6ed0bd24329bcf638da8930cff148d37ce079bc78b45cd8c`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-logical-not-source-window-6612-6711.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/logical-not-source-window-6612-6711.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/logical-not-source-window-6612-6711.c). SHA-256 `17c0b35f5a21ecd2e7c195b8a022a853a48241f8fef27b70b8c71d5132dc0e72`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-8.6.18-logical-AND-OR-source-window-4657-4806.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/logical-AND-OR-source-window-4657-4806.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/8.6.18/logical-AND-OR-source-window-4657-4806.c). SHA-256 `4abc8999ab515f745168a7e0f4015f83b86163da3b88f0656a9641999edd61df`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-Tcl_GetBooleanFromObj-2071-2077.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_GetBooleanFromObj-2071-2077.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_GetBooleanFromObj-2071-2077.c). SHA-256 `106c04a7d70bd8b7f0d08235453196e07e0194b9fcf6c0ee8c3cf32bce301237`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-Tcl_ExprBooleanObj-6696-6712.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_ExprBooleanObj-6696-6712.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_ExprBooleanObj-6696-6712.c). SHA-256 `9d16595cb1b5b136aa431c2ffb3d7e45098b4a3c342da411b04bfabf09bded85`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-Tcl_IfObjCmd-204-211.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_IfObjCmd-204-211.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_IfObjCmd-204-211.c). SHA-256 `6d83eb23c15aeb8e75a94823981455cb32bbd8bd00a5b3029f85f5152c57c094`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-Tcl_EvalEx-5228-5240.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_EvalEx-5228-5240.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_EvalEx-5228-5240.c). SHA-256 `74ac20e4dfc9df16fb5ee43dd556be27a704ebd8bef53c390173ac5c9957f353`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-Tcl_EvalObjEx-6180-6190.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_EvalObjEx-6180-6190.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/Tcl_EvalObjEx-6180-6190.c). SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-conditional-jump-source-window-4217-4316.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/conditional-jump-source-window-4217-4316.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/conditional-jump-source-window-4217-4316.c). SHA-256 `d88299d5b34a72c54bfe68372a409a3d0c53ea5182c1de095a408d9c423ebdde`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.0.4-logical-not-source-window-6214-6313.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/logical-not-source-window-6214-6313.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.0.4/logical-not-source-window-6214-6313.c). SHA-256 `2e9710560cae3f078b10843661fc96b3bd9fc1e7e12812ebb3d398c301526754`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-Tcl_GetBooleanFromObj-2021-2028.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_GetBooleanFromObj-2021-2028.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_GetBooleanFromObj-2021-2028.c). SHA-256 `af23019b697eadc02e4f6718536c6febed2b30d329ee256784ef8c45f4a43cbb`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-Tcl_ExprBooleanObj-6653-6669.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_ExprBooleanObj-6653-6669.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_ExprBooleanObj-6653-6669.c). SHA-256 `9d16595cb1b5b136aa431c2ffb3d7e45098b4a3c342da411b04bfabf09bded85`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-Tcl_IfObjCmd-205-212.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_IfObjCmd-205-212.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_IfObjCmd-205-212.c). SHA-256 `e00e402cea66935442f14b8b3e414d70e102d806e77747feb36fdadbd4574227`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-Tcl_EvalEx-5196-5208.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_EvalEx-5196-5208.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_EvalEx-5196-5208.c). SHA-256 `74ac20e4dfc9df16fb5ee43dd556be27a704ebd8bef53c390173ac5c9957f353`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-Tcl_EvalObjEx-6137-6147.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_EvalObjEx-6137-6147.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/Tcl_EvalObjEx-6137-6147.c). SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-conditional-jump-source-window-4507-4606.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/conditional-jump-source-window-4507-4606.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/conditional-jump-source-window-4507-4606.c). SHA-256 `c9e73c1cd86b3b87c18e4bfadd36125fec768f28f4ce28861a2bd5c220e1d1da`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-9.1.0-logical-not-source-window-6532-6631.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/logical-not-source-window-6532-6631.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/9.1.0/logical-not-source-window-6532-6631.c). SHA-256 `65ed077602338ff623cf2c5c5a87a4d39e05599bdcd6a36d98b8ef27fd475777`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_GetBoolean-6576-6582.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_GetBoolean-6576-6582.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_GetBoolean-6576-6582.c). SHA-256 `ed9f49f644ecfd68c81dfe2ade9a852d71e07d25adc80ef99b035d5eb3394705`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-ExprBool-9147-9169.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/ExprBool-9147-9169.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/ExprBool-9147-9169.c). SHA-256 `572bbcd17f6f9d823ab7d7fe2c9c72f47ed4e28715ff615d37de21adf4b2a738`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_GetBoolFromExpr-10314-10334.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_GetBoolFromExpr-10314-10334.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_GetBoolFromExpr-10314-10334.c). SHA-256 `16fca59dcfd687b61cecda487ef783d07eef961cab2f184580b0f3dfd922226c`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_ExprCoreCommand-14255-14271.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_ExprCoreCommand-14255-14271.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_ExprCoreCommand-14255-14271.c). SHA-256 `c1eda7376d7f41024ae641af165218384ea4b772a6ab95a9fb676503bb0afad8`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_IfCoreCommand-13273-13321.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_IfCoreCommand-13273-13321.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_IfCoreCommand-13273-13321.c). SHA-256 `c240060512ebe372e324c443fb1c986ca6b530861861b096029ada53ae387c51`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_WhileCoreCommand-12801-12829.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_WhileCoreCommand-12801-12829.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_WhileCoreCommand-12801-12829.c). SHA-256 `9312c37ddf1741b5e7f0302de2728cf8a2c9a9b667f4902b2949b2b6fafdbed3`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-JimExprOpAnd-9172-9186.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/JimExprOpAnd-9172-9186.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/JimExprOpAnd-9172-9186.c). SHA-256 `fdb779c4efd7562d938c87150fef00b08d5695d967f5f2e47b99704a72b2efae`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-JimExprOpOr-9189-9203.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/JimExprOpOr-9189-9203.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/JimExprOpOr-9189-9203.c). SHA-256 `0020e4eee25b569ac1101e299f1acd6daf69e92a4ffba872d48aa12f02be568f`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-source-windows-jim-Jim_EvalObj-11554-11796.c` (source-anchor): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_EvalObj-11554-11796.c](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/source-windows/jim/Jim_EvalObj-11554-11796.c). SHA-256 `5934c52fe03eeba0daafaa878ba31626f4d197dc72400efb41795e75374b3626`. Derived exact pinned original source window; explanatory branch text is independent of observed driver rows and supplies no extra execution or opcode admission.
- `native_logical_operand_truth_sites_original-observations.json` (reconfirmation): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/observations.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/observations.json). SHA-256 `809a1b9a362c4074089f73f333f4894602789f55f8e95e4a9bb5c68cacd53dfe`. Derived paired constructor/cache/identity/completion/output/result/error and selected route review; no new process or software assertion.
- `native_logical_operand_truth_sites_original-recorded-path-map.json` (provider): [rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/recorded-path-map.json](../../../../rust/tcl-registry/tests/data/native_logical_operand_truth_sites_original/recorded-path-map.json). SHA-256 `8ae75fb2d4480fa2b5025a176297bcd53a55434a21ccdb119e7d9823c1c968fe`. 504 exact original required input associations with published restore paths, independently full-byte verified.

## Source inspection

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1422–1439. Full-source SHA-256 `6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c`; snippet SHA-256 `f7d0675d35b15c64cd961b8d01eb0a8c3d0daab6c4f088fc1b5a94ceb6b9358f`; retained evidence `native_ensemble_map_prefix312-map-source-8.4.20-tclObj.c`.

```text
Tcl_GetBooleanFromObj(interp, objPtr, boolPtr)
    Tcl_Interp *interp; 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr;	/* The object from which to get boolean. */
    register int *boolPtr;	/* Place to store resulting boolean. */
{
    register int result;

    if (objPtr->typePtr == &tclBooleanType) {
	result = TCL_OK;
    } else {
	result = SetBooleanFromAny(interp, objPtr);
    }

    if (result == TCL_OK) {
	*boolPtr = (int) objPtr->internalRep.longValue;
    }
    return result;
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 5621–5642. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `7664e6948ecb726bccb6e774b435a0d1cd7ac50f439f2d73057f2f44790b2111`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_ExprBooleanObj(interp, objPtr, ptr)
    Tcl_Interp *interp;			/* Context in which to evaluate the
					 * expression. */
    register Tcl_Obj *objPtr;		/* Expression to evaluate. */
    int *ptr;				/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	if (resultPtr->typePtr == &tclIntType) {
	    *ptr = (resultPtr->internalRep.longValue != 0);
	} else if (resultPtr->typePtr == &tclDoubleType) {
	    *ptr = (resultPtr->internalRep.doubleValue != 0.0);
	} else {
	    result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	}
	Tcl_DecrRefCount(resultPtr);  /* discard the result object */
    }
    return result;
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 188–303. Full-source SHA-256 `92695cd7731ce21ab5441fb6ce2d23e5cb44f020256a607397ebe836b5a9b14a`; snippet SHA-256 `64b1767c16fe5bda50c2994232f2d0634d85c64b66c5a49382ecb11a4133d21d`; retained evidence `native_info_inventory_original-sources-8.4.20-tclCmdIL.c`.

```text
Tcl_IfObjCmd(dummy, interp, objc, objv)
    ClientData dummy;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument objects. */
{
    int thenScriptIndex = 0;	/* then script to be evaled after syntax check */
#ifdef TCL_TIP280
    Interp* iPtr = (Interp*) interp;
#endif
    int i, result, value;
    char *clause;
    i = 1;
    while (1) {
	/*
	 * At this point in the loop, objv and objc refer to an expression
	 * to test, either for the main expression or an expression
	 * following an "elseif".  The arguments after the expression must
	 * be "then" (optional) and a script to execute if the expression is
	 * true.
	 */

	if (i >= objc) {
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no expression after \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	if (!thenScriptIndex) {
	    result = Tcl_ExprBooleanObj(interp, objv[i], &value);
	    if (result != TCL_OK) {
		return result;
	    }
	}
	i++;
	if (i >= objc) {
	    missingScript:
	    clause = Tcl_GetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: no script following \"",
		    clause, "\" argument", (char *) NULL);
	    return TCL_ERROR;
	}
	clause = Tcl_GetString(objv[i]);
	if ((i < objc) && (strcmp(clause, "then") == 0)) {
	    i++;
	}
	if (i >= objc) {
	    goto missingScript;
	}
	if (value) {
	    thenScriptIndex = i;
	    value = 0;
	}
	
	/*
	 * The expression evaluated to false.  Skip the command, then
	 * see if there is an "else" or "elseif" clause.
	 */

	i++;
	if (i >= objc) {
	    if (thenScriptIndex) {
#ifndef TCL_TIP280
		return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
		/* TIP #280. Make invoking context available to branch */
		return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
				    iPtr->cmdFramePtr,thenScriptIndex);
#endif
	    }
	    return TCL_OK;
	}
	clause = Tcl_GetString(objv[i]);
	if ((clause[0] == 'e') && (strcmp(clause, "elseif") == 0)) {
	    i++;
	    continue;
	}
	break;
    }

    /*
     * Couldn't find a "then" or "elseif" clause to execute.  Check now
     * for an "else" clause.  We know that there's at least one more
     * argument when we get here.
     */

    if (strcmp(clause, "else") == 0) {
	i++;
	if (i >= objc) {
	    Tcl_AppendResult(interp,
		    "wrong # args: no script following \"else\" argument",
		    (char *) NULL);
	    return TCL_ERROR;
	}
    }
    if (i < objc - 1) {
	Tcl_AppendResult(interp,
		"wrong # args: extra words after \"else\" clause in \"if\" command",
		(char *) NULL);
	return TCL_ERROR;
    }
    if (thenScriptIndex) {
#ifndef TCL_TIP280
	return Tcl_EvalObjEx(interp, objv[thenScriptIndex], 0);
#else
	/* TIP #280. Make invoking context available to branch/else */
	return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
			    iPtr->cmdFramePtr,thenScriptIndex);
#endif
    }
#ifndef TCL_TIP280
    return Tcl_EvalObjEx(interp, objv[i], 0);
#else
    return TclEvalObjEx(interp, objv[i], 0, iPtr->cmdFramePtr,i);
#endif
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 3925–3939. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `a5b49685933d9b08ab91986de95912ed02f1a4d109e8ceb7a85c1c1ccfc0c129`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_EvalEx(interp, script, numBytes, flags)
    Tcl_Interp *interp;		/* Interpreter in which to evaluate the
				 * script.  Also used for error reporting. */
    CONST char *script;		/* First character of script to evaluate. */
    int numBytes;		/* Number of bytes in script.  If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags;			/* Collection of OR-ed bits that control
				 * the evaluation of the script.  Only
				 * TCL_EVAL_GLOBAL is currently
				 * supported. */
{
#ifdef TCL_TIP280
  return EvalEx (interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 4988–5002. Full-source SHA-256 `cab6d4decd1a3e365fe3cef49a2d5430dfef01cd775dd7d34f98064032f3a81a`; snippet SHA-256 `40d28485d47f0e93583873079da669382b7ab049c05e243f866ef4dce9ce830e`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclBasic.c`.

```text
Tcl_EvalObjEx(interp, objPtr, flags)
    Tcl_Interp *interp;			/* Token for command interpreter
					 * (returned by a previous call to
					 * Tcl_CreateInterp). */
    register Tcl_Obj *objPtr;		/* Pointer to object containing
					 * commands to execute. */
    int flags;				/* Collection of OR-ed bits that
					 * control the evaluation of the
					 * script.  Supported values are
					 * TCL_EVAL_GLOBAL and
					 * TCL_EVAL_DIRECT. */
{
#ifdef TCL_TIP280
  return TclEvalObjEx (interp, objPtr, flags, NULL, 0);
}

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 2270–2369. Full-source SHA-256 `970975c51cf8b78a291b92289cfeac8f023bd126c5d227ae5fa3c7d426a60c77`; snippet SHA-256 `09a4085311dcd6d29bd53ca94bf605799ac088f1e19cdf9b939a82b62dc9790e`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	opnd = 5;                             /* TRUE */
	pcAdjustment = TclGetInt4AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE4:
	opnd = TclGetInt4AtPtr(pc+1);         /* TRUE */
	pcAdjustment = 5;                     /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_FALSE1:
	opnd = 2;                             /* TRUE */
	pcAdjustment = TclGetInt1AtPtr(pc+1); /* FALSE */
	goto doJumpTrue;

    case INST_JUMP_TRUE1:
	opnd = TclGetInt1AtPtr(pc+1);          /* TRUE */
	pcAdjustment = 2;                      /* FALSE */
	    
    doJumpTrue:
	{
	    int b;
		
	    valuePtr = stackPtr[stackTop];
	    if (valuePtr->typePtr == &tclIntType) {
		b = (valuePtr->internalRep.longValue != 0);
	    } else if (valuePtr->typePtr == &tclDoubleType) {
		b = (valuePtr->internalRep.doubleValue != 0.0);
	    } else if (valuePtr->typePtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		b = (w != W0);
	    } else {
		result = Tcl_GetBooleanFromObj(interp, valuePtr, &b);
		if (result != TCL_OK) {
		    TRACE_WITH_OBJ(("%d => ERROR: ", opnd), Tcl_GetObjResult(interp));
		    goto checkForCatch;
		}
	    }
#ifndef TCL_COMPILE_DEBUG
	    NEXT_INST_F((b? opnd : pcAdjustment), 1, 0);
#else
	    if (b) {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s true, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc+opnd - codePtr->codeStart)));
		} else {
		    TRACE(("%d => %.20s true\n", pcAdjustment, O2S(valuePtr)));
		}
		NEXT_INST_F(opnd, 1, 0);
	    } else {
		if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE1)) {
		    TRACE(("%d => %.20s false\n", opnd, O2S(valuePtr)));
		} else {
		    opnd = pcAdjustment;
		    TRACE(("%d => %.20s false, new pc %u\n", opnd, O2S(valuePtr),
		            (unsigned int)(pc + opnd - codePtr->codeStart)));
		}
		NEXT_INST_F(pcAdjustment, 1, 0);
	    }
#endif
	}
	    	    
    case INST_LOR:
    case INST_LAND:
    {
	/*
	 * Operands must be boolean or numeric. No int->double
	 * conversions are performed.
	 */
		
	int i1, i2;
	int iResult;
	char *s;
	Tcl_ObjType *t1Ptr, *t2Ptr;

	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop - 1];;
	t1Ptr = valuePtr->typePtr;
	t2Ptr = value2Ptr->typePtr;

	if ((t1Ptr == &tclIntType) || (t1Ptr == &tclBooleanType)) {
	    i1 = (valuePtr->internalRep.longValue != 0);
	} else if (t1Ptr == &tclWideIntType) {
	    TclGetWide(w,valuePtr);
	    i1 = (w != W0);
	} else if (t1Ptr == &tclDoubleType) {
	    i1 = (valuePtr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(valuePtr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, valuePtr, i, w);
		if (valuePtr->typePtr == &tclIntType) {
		    i1 = (i != 0);
		} else {
		    i1 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL,
					       valuePtr, &i1);
		i1 = (i1 != 0);

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `logical-not-source-window`, lines 3765–3864. Full-source SHA-256 `970975c51cf8b78a291b92289cfeac8f023bd126c5d227ae5fa3c7d426a60c77`; snippet SHA-256 `9437d9d8659b1ea31859a655fa632e0ad318d68b9ea38bc2997462f86d81adf9`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_LNOT:
    {
	/*
	 * The operand must be numeric or a boolean string as
	 * accepted by Tcl_GetBooleanFromObj(). If the operand
	 * object is unshared modify it directly, otherwise
	 * create a copy to modify: this is "copy on write".
	 * Free any old string representation since it is now
	 * invalid.
	 */

	double d;
	int boolvar;
	Tcl_ObjType *tPtr;

	valuePtr = stackPtr[stackTop];
	tPtr = valuePtr->typePtr;
	if (!IS_INTEGER_TYPE(tPtr) && ((tPtr != &tclDoubleType)
	        || (valuePtr->bytes != NULL))) {
	    if ((tPtr == &tclBooleanType) && (valuePtr->bytes == NULL)) {
		valuePtr->typePtr = &tclIntType;
	    } else {
		char *s = Tcl_GetStringFromObj(valuePtr, &length);
		if (TclLooksLikeInt(s, length)) {
		    GET_WIDE_OR_INT(result, valuePtr, i, w);
		} else {
		    result = Tcl_GetDoubleFromObj((Tcl_Interp *) NULL,
		            valuePtr, &d);
		}
		if (result == TCL_ERROR && *pc == INST_LNOT) {
		    result = Tcl_GetBooleanFromObj((Tcl_Interp *)NULL,
		            valuePtr, &boolvar);
		    i = (long)boolvar; /* i is long, not int! */
		}
		if (result != TCL_OK) {
		    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n",
		            s, (tPtr? tPtr->name : "null")));
		    DECACHE_STACK_INFO();
		    IllegalExprOperandType(interp, pc, valuePtr);
		    CACHE_STACK_INFO();
		    goto checkForCatch;
		}
	    }
	    tPtr = valuePtr->typePtr;
	}

	if (Tcl_IsShared(valuePtr)) {
	    /*
	     * Create a new object.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		objResultPtr = Tcl_NewLongObj(
		    (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), objResultPtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewWideIntObj(-w);
		} else {
		    objResultPtr = Tcl_NewLongObj(w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), objResultPtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    objResultPtr = Tcl_NewDoubleObj(-d);
		} else {
		    /*
		     * Should be able to use "!d", but apparently
		     * some compilers can't handle it.
		     */
		    objResultPtr = Tcl_NewLongObj((d==0.0)? 1 : 0);
		}
		TRACE_WITH_OBJ(("%.6g => ", d), objResultPtr);
	    }
	    NEXT_INST_F(1, 1, 1);
	} else {
	    /*
	     * valuePtr is unshared. Modify it directly.
	     */
	    if ((tPtr == &tclIntType) || (tPtr == &tclBooleanType)) {
		i = valuePtr->internalRep.longValue;
		Tcl_SetLongObj(valuePtr,
	                (*pc == INST_UMINUS)? -i : !i);
		TRACE_WITH_OBJ(("%ld => ", i), valuePtr);
	    } else if (tPtr == &tclWideIntType) {
		TclGetWide(w,valuePtr);
		if (*pc == INST_UMINUS) {
		    Tcl_SetWideIntObj(valuePtr, -w);
		} else {
		    Tcl_SetLongObj(valuePtr, w == W0);
		}
		TRACE_WITH_OBJ((LLD" => ", w), valuePtr);
	    } else {
		d = valuePtr->internalRep.doubleValue;
		if (*pc == INST_UMINUS) {
		    Tcl_SetDoubleObj(valuePtr, -d);
		} else {
		    /*

```

tcl8.4 8.4.20, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 2332–2481. Full-source SHA-256 `970975c51cf8b78a291b92289cfeac8f023bd126c5d227ae5fa3c7d426a60c77`; snippet SHA-256 `ce0254c53912ce24467494ff711edd834a6e343010ed72cdab264df1c57bc75b`; retained evidence `native_info_commands_literal_original-original-inputs-8.4.20-generic-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND:
    {
	/*
	 * Operands must be boolean or numeric. No int->double
	 * conversions are performed.
	 */
		
	int i1, i2;
	int iResult;
	char *s;
	Tcl_ObjType *t1Ptr, *t2Ptr;

	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop - 1];;
	t1Ptr = valuePtr->typePtr;
	t2Ptr = value2Ptr->typePtr;

	if ((t1Ptr == &tclIntType) || (t1Ptr == &tclBooleanType)) {
	    i1 = (valuePtr->internalRep.longValue != 0);
	} else if (t1Ptr == &tclWideIntType) {
	    TclGetWide(w,valuePtr);
	    i1 = (w != W0);
	} else if (t1Ptr == &tclDoubleType) {
	    i1 = (valuePtr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(valuePtr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, valuePtr, i, w);
		if (valuePtr->typePtr == &tclIntType) {
		    i1 = (i != 0);
		} else {
		    i1 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL,
					       valuePtr, &i1);
		i1 = (i1 != 0);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		        (t1Ptr? t1Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, valuePtr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}
		
	if ((t2Ptr == &tclIntType) || (t2Ptr == &tclBooleanType)) {
	    i2 = (value2Ptr->internalRep.longValue != 0);
	} else if (t2Ptr == &tclWideIntType) {
	    TclGetWide(w,value2Ptr);
	    i2 = (w != W0);
	} else if (t2Ptr == &tclDoubleType) {
	    i2 = (value2Ptr->internalRep.doubleValue != 0.0);
	} else {
	    s = Tcl_GetStringFromObj(value2Ptr, &length);
	    if (TclLooksLikeInt(s, length)) {
		GET_WIDE_OR_INT(result, value2Ptr, i, w);
		if (value2Ptr->typePtr == &tclIntType) {
		    i2 = (i != 0);
		} else {
		    i2 = (w != W0);
		}
	    } else {
		result = Tcl_GetBooleanFromObj((Tcl_Interp *) NULL, value2Ptr, &i2);
	    }
	    if (result != TCL_OK) {
		TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		        (t2Ptr? t2Ptr->name : "null")));
		DECACHE_STACK_INFO();
		IllegalExprOperandType(interp, pc, value2Ptr);
		CACHE_STACK_INFO();
		goto checkForCatch;
	    }
	}

	/*
	 * Reuse the valuePtr object already on stack if possible.
	 */
	
	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewLongObj(iResult);
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    NEXT_INST_F(1, 2, 1);
	} else {	/* reuse the valuePtr object */
	    TRACE(("%.20s %.20s => %d\n", O2S(valuePtr), O2S(value2Ptr), iResult));
	    Tcl_SetLongObj(valuePtr, iResult);
	    NEXT_INST_F(1, 1, 0);
	}
    }

    /*
     * ---------------------------------------------------------
     *     Start of INST_LIST and related instructions.
     */

    case INST_LIST:
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj
	 * and then decrement their ref counts. 
	 */

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &(stackPtr[stackTop - (opnd-1)]));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);

    case INST_LIST_LENGTH:
	valuePtr = stackPtr[stackTop];

	result = Tcl_ListObjLength(interp, valuePtr, &length);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
	            Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
	objResultPtr = Tcl_NewIntObj(length);
	TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	NEXT_INST_F(1, 1, 1);
	    
    case INST_LIST_INDEX:
	/*** lindex with objc == 3 ***/
		
	/*
	 * Pop the two operands
	 */
	value2Ptr = stackPtr[stackTop];
	valuePtr  = stackPtr[stackTop- 1];

	/*
	 * Extract the desired list element
	 */
	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr == NULL) {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr), O2S(value2Ptr)),
	            Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

	/*
	 * Stash the list element on the stack
	 */

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1871–1915. Full-source SHA-256 `25136b8ad5a833dfecd5e45a2f8f54e52d78e10ff516a5abc61fbdfb7e6ee014`; snippet SHA-256 `bdb2aa20ed599e2defef802f7b5e1c812c412dbddec2dba2009f98eaa595e91b`; retained evidence `native_dict_write_error_lifetime226-failed-request225-original-source-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp, 	/* Used for error reporting if not NULL. */
    register Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    register int *boolPtr)	/* Place to store resulting boolean. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *boolPtr = (objPtr->internalRep.longValue != 0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBooleanType) {
	    *boolPtr = (int) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    /*
	     * Caution: Don't be tempted to check directly for the "double"
	     * Tcl_ObjType and then compare the intrep to 0.0. This isn't
	     * reliable because a "double" Tcl_ObjType can hold the NaN value.
	     * Use the API Tcl_GetDoubleFromObj, which does the checking and
	     * sets the proper error message for us.
	     */

            double d;

	    if (Tcl_GetDoubleFromObj(interp, objPtr, &d) != TCL_OK) {
		return TCL_ERROR;
	    }
	    *boolPtr = (d != 0.0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    *boolPtr = 1;
	    return TCL_OK;
	}
#ifndef NO_WIDE_TYPE
	if (objPtr->typePtr == &tclWideIntType) {
	    *boolPtr = (objPtr->internalRep.wideValue != 0);
	    return TCL_OK;
	}
#endif
    } while ((ParseBoolean(objPtr) == TCL_OK) || (TCL_OK ==
	    TclParseNumber(interp, objPtr, "boolean value", NULL,-1,NULL,0)));
    return TCL_ERROR;
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 5585–5601. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `7098a0fa7a6cf0b659622ba24066f1590c8b6265d1a68ecd9ec7ad41c137fb33`; retained evidence `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    register Tcl_Obj *objPtr,	/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 203–309. Full-source SHA-256 `51be49bd7e373aa319cb2affb341853473cbbf089694650bfff7689d3c925d8e`; snippet SHA-256 `e1c22a43638aa885ed8950fc0b783766a6e55e59ac8a03a92ef0b902ff411cb2`; retained evidence `native_info_inventory_original-sources-8.5.19-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    int thenScriptIndex = 0;	/* "then" script to be evaled after syntax
				 * check. */
    Interp *iPtr = (Interp *) interp;
    int i, result, value;
    char *clause;

    i = 1;
    while (1) {
	/*
	 * At this point in the loop, objv and objc refer to an expression to
	 * test, either for the main expression or an expression following an
	 * "elseif". The arguments after the expression must be "then"
	 * (optional) and a script to execute if the expression is true.
	 */

	if (i >= objc) {
	    clause = TclGetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no expression after \"", clause, "\" argument", NULL);
	    return TCL_ERROR;
	}
	if (!thenScriptIndex) {
	    result = Tcl_ExprBooleanObj(interp, objv[i], &value);
	    if (result != TCL_OK) {
		return result;
	    }
	}
	i++;
	if (i >= objc) {
	missingScript:
	    clause = TclGetString(objv[i-1]);
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no script following \"", clause, "\" argument", NULL);
	    return TCL_ERROR;
	}
	clause = TclGetString(objv[i]);
	if ((i < objc) && (strcmp(clause, "then") == 0)) {
	    i++;
	}
	if (i >= objc) {
	    goto missingScript;
	}
	if (value) {
	    thenScriptIndex = i;
	    value = 0;
	}

	/*
	 * The expression evaluated to false. Skip the command, then see if
	 * there is an "else" or "elseif" clause.
	 */

	i++;
	if (i >= objc) {
	    if (thenScriptIndex) {
		/*
		 * TIP #280. Make invoking context available to branch.
		 */

		return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
			iPtr->cmdFramePtr, thenScriptIndex);
	    }
	    return TCL_OK;
	}
	clause = TclGetString(objv[i]);
	if ((clause[0] == 'e') && (strcmp(clause, "elseif") == 0)) {
	    i++;
	    continue;
	}
	break;
    }

    /*
     * Couldn't find a "then" or "elseif" clause to execute. Check now for an
     * "else" clause. We know that there's at least one more argument when we
     * get here.
     */

    if (strcmp(clause, "else") == 0) {
	i++;
	if (i >= objc) {
	    Tcl_AppendResult(interp, "wrong # args: ",
		    "no script following \"else\" argument", NULL);
	    return TCL_ERROR;
	}
    }
    if (i < objc - 1) {
	Tcl_AppendResult(interp, "wrong # args: ",
		"extra words after \"else\" clause in \"if\" command", NULL);
	return TCL_ERROR;
    }
    if (thenScriptIndex) {
	/*
	 * TIP #280. Make invoking context available to branch/else.
	 */

	return TclEvalObjEx(interp, objv[thenScriptIndex], 0,
		iPtr->cmdFramePtr, thenScriptIndex);
    }
    return TclEvalObjEx(interp, objv[i], 0, iPtr->cmdFramePtr, i);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 4079–4091. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `b2d8a2c66f34819f51d7f07a57d34111d47895effa1144b84038710502d52700`; retained evidence `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    int numBytes,		/* Number of bytes in script. If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
  return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 5076–5086. Full-source SHA-256 `d09c16386ea0376dc3590a74ded2c6e2bcef5079f2b1a3785545bad37c7ead63`; snippet SHA-256 `b5306fbe56f0b6a0a1c28bb8843b0d1bfb00d22107bb1225d62f3035e8bdc5bc`; retained evidence `native_info_commands_literal_original-original-inputs-8.5.19-generic-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    register Tcl_Obj *objPtr,	/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 3595–3694. Full-source SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`; snippet SHA-256 `cbc7d8bc02497dbd1569daa2bfe23da9c62d67a5de16645b13a88a37dead0436`; retained evidence `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc+1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset*/
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc+1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc+1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc+1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(interp, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE_WITH_OBJ(("%d => ERROR: ", jmpOffset[
		    ((*pc == INST_JUMP_FALSE1) || (*pc == INST_JUMP_FALSE4))
		    ? 0 : 1]), Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s true, new pc %u\n", jmpOffset[1],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE(("%d => %.20s true\n", jmpOffset[0], O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE(("%d => %.20s false\n", jmpOffset[0], O2S(valuePtr)));
	    } else {
		TRACE(("%d => %.20s false, new pc %u\n", jmpOffset[0],
			O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	int opnd;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc+1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d => %.20s ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    int jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %u\n",
		    (unsigned)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }

    /*
     * These two instructions are now redundant: the complete logic of the LOR
     * and LAND is now handled by the expression compiler.
     */

    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;
	Tcl_Obj *value2Ptr = OBJ_AT_TOS;
	Tcl_Obj *valuePtr = OBJ_UNDER_TOS;

	result = TclGetBooleanFromObj(NULL, valuePtr, &i1);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `logical-not-source-window`, lines 6275–6374. Full-source SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`; snippet SHA-256 `c381e2a10b93c58ceaa440b408be22739e5b0519f78a6b515eb1926a96cc30ea`; retained evidence `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	result = TclGetBooleanFromObj(NULL, valuePtr, &b);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = constants[!b];
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT: {
	mp_int big;
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
		|| (type == TCL_NUMBER_NAN) || (type == TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	if (type == TCL_NUMBER_LONG) {
	    long l = *((const long *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l);
	    NEXT_INST_F(1, 0, 0);
	}
#ifndef NO_WIDE_TYPE
	if (type == TCL_NUMBER_WIDE) {
	    Tcl_WideInt w = *((const Tcl_WideInt *)ptr);

	    if (Tcl_IsShared(valuePtr)) {
		objResultPtr = Tcl_NewWideIntObj(~w);
		NEXT_INST_F(1, 1, 1);
	    }
	    Tcl_SetWideIntObj(valuePtr, ~w);
	    NEXT_INST_F(1, 0, 0);
	}
#endif
	Tcl_TakeBignumFromObj(NULL, valuePtr, &big);
	/* ~a = - a - 1 */
	mp_neg(&big, &big);
	mp_sub_d(&big, 1, &big);
	if (Tcl_IsShared(valuePtr)) {
	    objResultPtr = Tcl_NewBignumObj(&big);
	    NEXT_INST_F(1, 1, 1);
	}
	Tcl_SetBignumObj(valuePtr, &big);
	NEXT_INST_F(1, 0, 0);
    }

    case INST_UMINUS: {
	ClientData ptr;
	int type;
	Tcl_Obj *valuePtr = OBJ_AT_TOS;

	result = GetNumberFromObj(NULL, valuePtr, &ptr, &type);
	if ((result != TCL_OK)
#ifndef ACCEPT_NAN
		|| (type == TCL_NUMBER_NAN)
#endif
		) {
	    result = TCL_ERROR;
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}
	switch (type) {
	case TCL_NUMBER_DOUBLE: {
	    double d;

	    if (Tcl_IsShared(valuePtr)) {
		TclNewDoubleObj(objResultPtr, -(*((const double *)ptr)));

```

tcl8.5 8.5.19, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 3680–3829. Full-source SHA-256 `de5706aa9022663636927515066d4630cbfe8ded754dceaca7ae61ee8612b47d`; snippet SHA-256 `3d3fc92b282dae892438d9d526ee340f4b057b48deb0011cd55218d56774f8e3`; retained evidence `native_dict_write_error_lifetime226-failed-request225-original-source-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;
	Tcl_Obj *value2Ptr = OBJ_AT_TOS;
	Tcl_Obj *valuePtr = OBJ_UNDER_TOS;

	result = TclGetBooleanFromObj(NULL, valuePtr, &i1);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	result = TclGetBooleanFromObj(NULL, value2Ptr, &i2);
	if (result != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto checkForCatch;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = constants[iResult];
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }

    /*
     * ---------------------------------------------------------
     *	   Start of INST_LIST and related instructions.
     */

    case INST_LIST: {
	/*
	 * Pop the opnd (objc) top stack elements into a new list obj and then
	 * decrement their ref counts.
	 */

	int opnd;

	opnd = TclGetUInt4AtPtr(pc+1);
	objResultPtr = Tcl_NewListObj(opnd, &OBJ_AT_DEPTH(opnd-1));
	TRACE_WITH_OBJ(("%u => ", opnd), objResultPtr);
	NEXT_INST_V(5, opnd, 1);
    }

    case INST_LIST_LENGTH: {
	Tcl_Obj *valuePtr;
	int length;

	valuePtr = OBJ_AT_TOS;

	result = TclListObjLength(interp, valuePtr, &length);
	if (result == TCL_OK) {
	    TclNewIntObj(objResultPtr, length);
	    TRACE(("%.20s => %d\n", O2S(valuePtr), length));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_WITH_OBJ(("%.30s => ERROR: ", O2S(valuePtr)),
		    Tcl_GetObjResult(interp));
	    goto checkForCatch;
	}
    }

    case INST_LIST_INDEX: {
	/*** lindex with objc == 3 ***/

	/* Variables also for INST_LIST_INDEX_IMM */

	int listc, idx, opnd, pcAdjustment;
	Tcl_Obj **listv;
	Tcl_Obj *valuePtr, *value2Ptr;

	/*
	 * Pop the two operands.
	 */

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;

	/*
	 * Extract the desired list element.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);
	if ((result == TCL_OK) && (value2Ptr->typePtr != &tclListType)
		&& (TclGetIntForIndexM(NULL , value2Ptr, listc-1,
			&idx) == TCL_OK)) {
	    TclDecrRefCount(value2Ptr);
	    tosPtr--;
	    pcAdjustment = 1;
	    goto lindexFastPath;
	}

	objResultPtr = TclLindexList(interp, valuePtr, value2Ptr);
	if (objResultPtr) {
	    /*
	     * Stash the list element on the stack.
	     */

	    TRACE(("%.20s %.20s => %s\n",
		    O2S(valuePtr), O2S(value2Ptr), O2S(objResultPtr)));
	    NEXT_INST_F(1, 2, -1);	/* Already has the correct refCount */
	} else {
	    TRACE_WITH_OBJ(("%.30s %.30s => ERROR: ", O2S(valuePtr),
		    O2S(value2Ptr)), Tcl_GetObjResult(interp));
	    result = TCL_ERROR;
	    goto checkForCatch;
	}

    case INST_LIST_INDEX_IMM:
	/*** lindex with objc==3 and index in bytecode stream ***/

	pcAdjustment = 5;

	/*
	 * Pop the list and get the index.
	 */

	valuePtr = OBJ_AT_TOS;
	opnd = TclGetInt4AtPtr(pc+1);

	/*
	 * Get the contents of the list, making sure that it really is a list
	 * in the process.
	 */

	result = TclListObjGetElements(interp, valuePtr, &listc, &listv);

	if (result == TCL_OK) {
	    /*
	     * Select the list item based on the index. Negative operand means
	     * end-based indexing.
	     */

	    if (opnd < -1) {

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 1885–1929. Full-source SHA-256 `7a7ef8ec85c74581129e8f3bef939e3191a432dcb021e0a16f1cf9971049f90d`; snippet SHA-256 `23411a5f88b660770178b2a3bf77ecb4482ddce156edc5fef7688c862a95de29`; retained evidence `native_ensemble_map_prefix312-map-source-8.6.18-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    do {
	if (objPtr->typePtr == &tclIntType) {
	    *intPtr = (objPtr->internalRep.longValue != 0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBooleanType) {
	    *intPtr = (int) objPtr->internalRep.longValue;
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclDoubleType) {
	    /*
	     * Caution: Don't be tempted to check directly for the "double"
	     * Tcl_ObjType and then compare the internalrep to 0.0. This isn't
	     * reliable because a "double" Tcl_ObjType can hold the NaN value.
	     * Use the API Tcl_GetDoubleFromObj, which does the checking and
	     * sets the proper error message for us.
	     */

	    double d;

	    if (Tcl_GetDoubleFromObj(interp, objPtr, &d) != TCL_OK) {
		return TCL_ERROR;
	    }
	    *intPtr = (d != 0.0);
	    return TCL_OK;
	}
	if (objPtr->typePtr == &tclBignumType) {
	    *intPtr = 1;
	    return TCL_OK;
	}
#ifndef TCL_WIDE_INT_IS_LONG
	if (objPtr->typePtr == &tclWideIntType) {
	    *intPtr = (objPtr->internalRep.wideValue != 0);
	    return TCL_OK;
	}
#endif
    } while ((ParseBoolean(objPtr) == TCL_OK) || (TCL_OK ==
	    TclParseNumber(interp, objPtr, "boolean value", NULL,-1,NULL,0)));
    return TCL_ERROR;
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6608–6624. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `cbd302a40f3b1c68f7f190ae578da2731f8ed38195ebba35eb02e207c710d2c1`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,	/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 205–212. Full-source SHA-256 `26d3d423720df253aefd7ad412e939165282a1f80fd6cd8c42900875d9627a23`; snippet SHA-256 `faccb69addf75dfcdb98230ab8b8a237d9c66ef76f185f3e05d0d4c4e8066ef8`; retained evidence `native_info_inventory_original-sources-8.6.18-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    ClientData dummy,		/* Not used. */
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNRIfObjCmd, dummy, objc, objv);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5065–5077. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `0abaf933d18625dee4a5918aad3aa1fe8d9282c5874be165f95e440c496f2b41`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    int numBytes,		/* Number of bytes in script. If < 0, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6077–6087. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`; retained evidence `native_oo_original_collision249-request-source-anchors-8.6.18-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4574–4673. Full-source SHA-256 `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491`; snippet SHA-256 `10c0df04b591090e6ed0bd24329bcf638da8930cff148d37ce079bc78b45cd8c`; retained evidence `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc+1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc+1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc+1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc+1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;
	TRACE(("%d => ", jmpOffset[
		(*pc==INST_JUMP_FALSE1 || *pc==INST_JUMP_FALSE4) ? 0 : 1]));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s true, new pc %u\n", O2S(valuePtr),
			(unsigned)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE_APPEND(("%.20s true\n", O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s false\n", O2S(valuePtr)));
	    } else {
		TRACE_APPEND(("%.20s false, new pc %u\n", O2S(valuePtr),
			(unsigned)(pc + jmpOffset[0] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }
    break;

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc+1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d \"%.20s\" => ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    int jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %u\n",
		    (unsigned)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }
    break;

    /*
     * These two instructions are now redundant: the complete logic of the LOR
     * and LAND is now handled by the expression compiler.
     */

    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	if (TclGetBooleanFromObj(NULL, valuePtr, &i1) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `logical-not-source-window`, lines 6612–6711. Full-source SHA-256 `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491`; snippet SHA-256 `17c0b35f5a21ecd2e7c195b8a022a853a48241f8fef27b70b8c71d5132dc0e72`; retained evidence `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;

	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE(("\"%.20s\" => ERROR: illegal type %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_WITH_OBJ(("%s => ", O2S(valuePtr)), objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND(("ERROR: illegal type %s\n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_LONG) {
	    l1 = *((const long *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewLongObj(objResultPtr, ~l1);
		TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetLongObj(valuePtr, ~l1);
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND(("ERROR: illegal type %s \n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	break;
	case TCL_NUMBER_LONG:
	    l1 = *((const long *) ptr1);
	    if (l1 != LONG_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewLongObj(objResultPtr, -l1);
		    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetLongObj(valuePtr, -l1);
		TRACE_APPEND(("%s\n", O2S(valuePtr)));
		NEXT_INST_F(1, 0, 0);
	    }
	    /* FALLTHROUGH */
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:

```

tcl8.6 8.6.18, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclExecute.c`, function `logical-AND-OR-source-window`, lines 4657–4806. Full-source SHA-256 `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491`; snippet SHA-256 `4abc8999ab515f745168a7e0f4015f83b86163da3b88f0656a9641999edd61df`; retained evidence `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
    case INST_LOR:
    case INST_LAND: {
	/*
	 * Operands must be boolean or numeric. No int->double conversions are
	 * performed.
	 */

	int i1, i2, iResult;

	value2Ptr = OBJ_AT_TOS;
	valuePtr = OBJ_UNDER_TOS;
	if (TclGetBooleanFromObj(NULL, valuePtr, &i1) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (TclGetBooleanFromObj(NULL, value2Ptr, &i2) != TCL_OK) {
	    TRACE(("\"%.20s\" => ILLEGAL TYPE %s \n", O2S(value2Ptr),
		    (value2Ptr->typePtr? value2Ptr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, pc, value2Ptr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}

	if (*pc == INST_LOR) {
	    iResult = (i1 || i2);
	} else {
	    iResult = (i1 && i2);
	}
	objResultPtr = TCONST(iResult);
	TRACE(("%.20s %.20s => %d\n", O2S(valuePtr),O2S(value2Ptr),iResult));
	NEXT_INST_F(1, 2, 1);
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT: {
	Namespace *currNsPtr = (Namespace *) TclGetCurrentNamespace(interp);

	if (currNsPtr == (Namespace *) TclGetGlobalNamespace(interp)) {
	    TclNewLiteralStringObj(objResultPtr, "::");
	} else {
	    TclNewStringObj(objResultPtr, currNsPtr->fullName,
		    strlen(currNsPtr->fullName));
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_IS_DELETED)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;
    case INST_INFO_LEVEL_NUM:
	TclNewIntObj(objResultPtr, iPtr->varFramePtr->level);
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_INFO_LEVEL_ARGS: {
	int level;
	CallFrame *framePtr = iPtr->varFramePtr;
	CallFrame *rootFramePtr = iPtr->rootFramePtr;

	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	if (TclGetIntFromObj(interp, OBJ_AT_TOS, &level) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	if (level <= 0) {
	    level += framePtr->level;
	}
	for (; (framePtr->level!=level) && (framePtr!=rootFramePtr) ;
		framePtr = framePtr->callerVarPtr) {
	    /* Empty loop body */
	}
	if (framePtr == rootFramePtr) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "bad level \"%s\"", TclGetString(OBJ_AT_TOS)));
	    TRACE_ERROR(interp);
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "STACK_LEVEL",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	objResultPtr = Tcl_NewListObj(framePtr->objc, framePtr->objv);
	TRACE_APPEND(("%.30s\n", O2S(objResultPtr)));
	NEXT_INST_F(1, 1, 1);
    }
    {
	Tcl_Command cmd, origCmd;

    case INST_RESOLVE_COMMAND:
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	TclNewObj(objResultPtr);
	if (cmd != NULL) {
	    Tcl_GetCommandFullName(interp, cmd, objResultPtr);
	}
	TRACE_WITH_OBJ(("\"%.20s\" => ", O2S(OBJ_AT_TOS)), objResultPtr);
	NEXT_INST_F(1, 1, 1);

    case INST_ORIGIN_COMMAND:
	TRACE(("\"%.30s\" => ", O2S(OBJ_AT_TOS)));
	cmd = Tcl_GetCommandFromObj(interp, OBJ_AT_TOS);
	if (cmd == NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "invalid command name \"%s\"", TclGetString(OBJ_AT_TOS)));
	    DECACHE_STACK_INFO();
	    Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "COMMAND",
		    TclGetString(OBJ_AT_TOS), (char *)NULL);
	    CACHE_STACK_INFO();
	    TRACE_APPEND(("ERROR: not command\n"));
	    goto gotError;
	}
	origCmd = TclGetOriginalCommand(cmd);
	if (origCmd == NULL) {
	    origCmd = cmd;
	}
	TclNewObj(objResultPtr);
	Tcl_GetCommandFullName(interp, origCmd, objResultPtr);
	TRACE_APPEND(("\"%.30s\"", O2S(OBJ_AT_TOS)));
	NEXT_INST_F(1, 1, 1);
    }

    /*
     * -----------------------------------------------------------------
     *	   Start of TclOO support instructions.
     */

    {
	Object *oPtr;
	CallFrame *framePtr;
	CallContext *contextPtr;

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 2071–2077. Full-source SHA-256 `91a390bd24fbe71108108eeccf9955df0389f0f81dff24f303c32344469e8d4a`; snippet SHA-256 `106c04a7d70bd8b7f0d08235453196e07e0194b9fcf6c0ee8c3cf32bce301237`; retained evidence `native_ensemble_map_prefix312-map-source-9.0.4-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,         /* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,	/* The object from which to get boolean. */
    int *intPtr)	/* Place to store resulting boolean. */
{
    return Tcl_GetBoolFromObj(interp, objPtr, (TCL_NULL_OK-2)&(int)sizeof(int), (char *)(void *)intPtr);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6696–6712. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `9d16595cb1b5b136aa431c2ffb3d7e45098b4a3c342da411b04bfabf09bded85`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,		/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 204–211. Full-source SHA-256 `af564d5d77f8f70cf120dbd49509a8580130cc89352f52424a01920fb0da4fa7`; snippet SHA-256 `6d83eb23c15aeb8e75a94823981455cb32bbd8bd00a5b3029f85f5152c57c094`; retained evidence `native_info_inventory_original-sources-9.0.4-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    return Tcl_NRCallObjProc(interp, TclNRIfObjCmd, clientData, objc, objv);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5228–5240. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `74ac20e4dfc9df16fb5ee43dd556be27a704ebd8bef53c390173ac5c9957f353`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    Tcl_Size numBytes,		/* Number of bytes in script. If -1, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6180–6190. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`; retained evidence `native_oo_original_collision249-request-source-anchors-9.0.4-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4217–4316. Full-source SHA-256 `0c42699cdcf610813ce77e2a6fec505f12d82a87b110950c92824fdf30d3c42a`; snippet SHA-256 `d88299d5b34a72c54bfe68372a409a3d0c53ea5182c1de095a408d9c423ebdde`; retained evidence `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE4:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	goto doCondJump;

    case INST_JUMP_TRUE4:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	goto doCondJump;

    case INST_JUMP_FALSE1:
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	goto doCondJump;

    case INST_JUMP_TRUE1:
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);

    doCondJump:
	valuePtr = OBJ_AT_TOS;
	TRACE(("%d => ", jmpOffset[
		(*pc==INST_JUMP_FALSE1 || *pc==INST_JUMP_FALSE4) ? 0 : 1]));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s true, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[1] - codePtr->codeStart)));
	    } else {
		TRACE_APPEND(("%.20s true\n", O2S(valuePtr)));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE1) || (*pc == INST_JUMP_TRUE4)) {
		TRACE_APPEND(("%.20s false\n", O2S(valuePtr)));
	    } else {
		TRACE_APPEND(("%.20s false, new pc %" TCL_Z_MODIFIER "u\n", O2S(valuePtr),
			(size_t)(pc + jmpOffset[0] - codePtr->codeStart)));
	    }
	}
#endif
	NEXT_INST_F(jmpOffset[b], 1, 0);
    }
    break;

    case INST_JUMP_TABLE: {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails.
	 */

	opnd = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *) codePtr->auxDataArrayPtr[opnd].clientData;
	TRACE(("%d \"%.20s\" => ", opnd, O2S(OBJ_AT_TOS)));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	if (hPtr != NULL) {
	    Tcl_Size jumpOffset = PTR2INT(Tcl_GetHashValue(hPtr));

	    TRACE_APPEND(("found in table, new pc %" TCL_Z_MODIFIER "u\n",
		    (size_t)(pc - codePtr->codeStart + jumpOffset)));
	    NEXT_INST_F(jumpOffset, 1, 0);
	} else {
	    TRACE_APPEND(("not found in table\n"));
	    NEXT_INST_F(5, 1, 0);
	}
    }
    break;

    /*
     * -----------------------------------------------------------------
     *	   Start of general introspector instructions.
     */

    case INST_NS_CURRENT:
	objResultPtr = TclNewNamespaceObj(TclGetCurrentNamespace(interp));
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    break;
    case INST_COROUTINE_NAME: {
	CoroutineData *corPtr = iPtr->execEnvPtr->corPtr;

	TclNewObj(objResultPtr);
	if (corPtr && !(corPtr->cmdPtr->flags & CMD_DYING)) {
	    Tcl_GetCommandFullName(interp, (Tcl_Command) corPtr->cmdPtr,
		    objResultPtr);
	}
	TRACE_WITH_OBJ(("=> "), objResultPtr);
	NEXT_INST_F(1, 0, 1);
    }
    break;

```

tcl9.0 9.0.4, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclExecute.c`, function `logical-not-source-window`, lines 6214–6313. Full-source SHA-256 `0c42699cdcf610813ce77e2a6fec505f12d82a87b110950c92824fdf30d3c42a`; snippet SHA-256 `2e9710560cae3f078b10843661fc96b3bd9fc1e7e12812ebb3d398c301526754`; retained evidence `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
    case INST_LNOT: {
	int b;

	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE(("\"%.20s\" => ERROR: illegal type %s\n", O2S(valuePtr),
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_WITH_OBJ(("%s => ", O2S(valuePtr)), objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND(("ERROR: illegal type %s\n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_INT) {
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewIntObj(objResultPtr, ~w1);
		TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetIntObj(valuePtr, ~w1);
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE(("\"%.20s\" => ", O2S(valuePtr)));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND(("ERROR: illegal type %s \n",
		    (valuePtr->typePtr? valuePtr->typePtr->name : "null")));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	break;
	case TCL_NUMBER_INT:
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (w1 != WIDE_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewIntObj(objResultPtr, -w1);
		    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetIntObj(valuePtr, -w1);
		TRACE_APPEND(("%s\n", O2S(valuePtr)));
		NEXT_INST_F(1, 0, 0);
	    }
	    /* FALLTHROUGH */
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND(("%s\n", O2S(objResultPtr)));
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND(("%s\n", O2S(valuePtr)));
	    NEXT_INST_F(1, 0, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclObj.c`, function `Tcl_GetBooleanFromObj`, lines 2021–2028. Full-source SHA-256 `58c041dbf20bba3c8fef5782969c661d4d60a6ee0bc915f30e55c19cedb31976`; snippet SHA-256 `af23019b697eadc02e4f6718536c6febed2b30d329ee256784ef8c45f4a43cbb`; retained evidence `native_ensemble_map_prefix312-map-source-9.1.0-tclObj.c`.

```text
Tcl_GetBooleanFromObj(
    Tcl_Interp *interp,		/* Used for error reporting if not NULL. */
    Tcl_Obj *objPtr,		/* The object from which to get boolean. */
    int *intPtr)		/* Place to store resulting boolean. */
{
    return Tcl_GetBoolFromObj(interp, objPtr, (TCL_NULL_OK-2)&(int)sizeof(int),
	    (char *)(void *)intPtr);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_ExprBooleanObj`, lines 6653–6669. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `9d16595cb1b5b136aa431c2ffb3d7e45098b4a3c342da411b04bfabf09bded85`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_ExprBooleanObj(
    Tcl_Interp *interp,		/* Context in which to evaluate the
				 * expression. */
    Tcl_Obj *objPtr,		/* Expression to evaluate. */
    int *ptr)			/* Where to store 0/1 result. */
{
    Tcl_Obj *resultPtr;
    int result;

    result = Tcl_ExprObj(interp, objPtr, &resultPtr);
    if (result == TCL_OK) {
	result = Tcl_GetBooleanFromObj(interp, resultPtr, ptr);
	Tcl_DecrRefCount(resultPtr);
				/* Discard the result object. */
    }
    return result;
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `Tcl_IfObjCmd`, lines 205–212. Full-source SHA-256 `d5b92a5b9c8c923142db6b76f816cbe6ca857255861a3c3dc53787fa9b335e5a`; snippet SHA-256 `e00e402cea66935442f14b8b3e414d70e102d806e77747feb36fdadbd4574227`; retained evidence `native_info_inventory_original-sources-9.1.0-tclCmdIL.c`.

```text
Tcl_IfObjCmd(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    return Tcl_NRCallObjProc2(interp, TclNRIfObjCmd, clientData, objc, objv);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_EvalEx`, lines 5196–5208. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `74ac20e4dfc9df16fb5ee43dd556be27a704ebd8bef53c390173ac5c9957f353`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    Tcl_Size numBytes,		/* Number of bytes in script. If -1, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `Tcl_EvalObjEx`, lines 6137–6147. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `08da5d75b1501350b0dece515f726aa0d3875fcbcbc992a92ea0b5530afccce3`; retained evidence `native_oo_original_collision249-request-source-anchors-9.1.0-tclBasic.c`.

```text
Tcl_EvalObjEx(
    Tcl_Interp *interp,		/* Token for command interpreter (returned by
				 * a previous call to Tcl_CreateInterp). */
    Tcl_Obj *objPtr,		/* Pointer to object containing commands to
				 * execute. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Supported values
				 * are TCL_EVAL_GLOBAL and TCL_EVAL_DIRECT. */
{
    return TclEvalObjEx(interp, objPtr, flags, NULL, 0);
}

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclExecute.c`, function `conditional-jump-source-window`, lines 4507–4606. Full-source SHA-256 `9dd4e9a080136f2ce36ee8f9acf4a30e770cc5fc417519468c785d76013151f4`; snippet SHA-256 `c9e73c1cd86b3b87c18e4bfadd36125fec768f28f4ce28861a2bd5c220e1d1da`; retained evidence `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
    case INST_JUMP_FALSE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_FALSE1);
	jmpOffset[0] = TclGetInt1AtPtr(pc + 1);
	jmpOffset[1] = 2;
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE1:
	DEPRECATED_OPCODE_MARK(INST_JUMP_TRUE1);
	jmpOffset[0] = 2;
	jmpOffset[1] = TclGetInt1AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);
	goto doCondJump;
#endif

    case INST_JUMP_FALSE:
	jmpOffset[0] = TclGetInt4AtPtr(pc + 1);	/* FALSE offset */
	jmpOffset[1] = 5;			/* TRUE offset */
	TRACE("%d => ", jmpOffset[0]);
	goto doCondJump;

    case INST_JUMP_TRUE:
	jmpOffset[0] = 5;
	jmpOffset[1] = TclGetInt4AtPtr(pc + 1);
	TRACE("%d => ", jmpOffset[1]);

    doCondJump:
	valuePtr = OBJ_AT_TOS;

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	if (TclGetBooleanFromObj(interp, valuePtr, &b) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}

#ifdef TCL_COMPILE_DEBUG
	if (b) {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    ||  (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s true, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[1]);
	    } else {
		TRACE_APPEND("%.20s true\n", O2S(valuePtr));
	    }
	} else {
	    if ((*pc == INST_JUMP_TRUE)
#ifndef REMOVE_DEPRECATED_OPCODES
		    || (*pc == INST_JUMP_TRUE1)
#endif
		    ) {
		TRACE_APPEND("%.20s false\n", O2S(valuePtr));
	    } else {
		TRACE_APPEND("%.20s false, new pc %" SIZEd "\n", O2S(valuePtr),
			PC_REL + jmpOffset[0]);
	    }
	}
#endif
	NEXT_INST_F0(jmpOffset[b], 1);
    }

    {
	Tcl_HashEntry *hPtr;
	JumptableInfo *jtPtr;
	JumptableNumInfo *jtnPtr;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails. Lookup by string.
	 */

    case INST_JUMP_TABLE:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtPtr = (JumptableInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	hPtr = Tcl_FindHashEntry(&jtPtr->hashTable, TclGetString(OBJ_AT_TOS));
	goto processJumpTableEntry;

	/*
	 * Jump to location looked up in a hashtable; fall through to next
	 * instr if lookup fails or key is non-integer. Lookup by integer.
	 */

    case INST_JUMP_TABLE_NUM:
	tblIdx = TclGetInt4AtPtr(pc + 1);
	jtnPtr = (JumptableNumInfo *)
		codePtr->auxDataArrayPtr[tblIdx].clientData;
	TRACE("%u \"%.20s\" => ", tblIdx, O2S(OBJ_AT_TOS));
	DECACHE_STACK_INFO();
	Tcl_WideInt key;
	if (Tcl_GetWideIntFromObj(interp, OBJ_AT_TOS, &key) != TCL_OK) {
	    TRACE_ERROR(interp);
	    goto gotError;
	}
	CACHE_STACK_INFO();
	hPtr = Tcl_FindHashEntry(&jtnPtr->hashTable, INT2PTR(key));

```

tcl9.1 9.1.0, revision `Exact original release bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclExecute.c`, function `logical-not-source-window`, lines 6532–6631. Full-source SHA-256 `9dd4e9a080136f2ce36ee8f9acf4a30e770cc5fc417519468c785d76013151f4`; snippet SHA-256 `65ed077602338ff623cf2c5c5a87a4d39e05599bdcd6a36d98b8ef27fd475777`; retained evidence `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
    case INST_LNOT: {
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));

	/* TODO - check claim that taking address of b harms performance */
	/* TODO - consider optimization search for constants */
	int b;
	if (TclGetBooleanFromObj(NULL, valuePtr, &b) != TCL_OK) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	/* TODO: Consider peephole opt. */
	objResultPtr = TCONST(!b);
	TRACE_APPEND_OBJ(objResultPtr);
	NEXT_INST_F(1, 1, 1);
    }

    case INST_BITNOT:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| (type1==TCL_NUMBER_NAN) || (type1==TCL_NUMBER_DOUBLE)) {
	    /*
	     * ... ~$NonInteger => raise an error.
	     */

	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	if (type1 == TCL_NUMBER_INT) {
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (Tcl_IsShared(valuePtr)) {
		TclNewIntObj(objResultPtr, ~w1);
		TRACE_APPEND_NUM_OBJ(objResultPtr);
		NEXT_INST_F(1, 1, 1);
	    }
	    TclSetIntObj(valuePtr, ~w1);
	    TRACE_APPEND("%s\n", O2S(valuePtr));
	    NEXT_INST_F0(1, 0);
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UMINUS:
	valuePtr = OBJ_AT_TOS;
	TRACE("\"%.20s\" => ", O2S(valuePtr));
	if ((GetNumberFromObj(NULL, valuePtr, &ptr1, &type1) != TCL_OK)
		|| IsErroringNaNType(type1)) {
	    TRACE_APPEND("ERROR: illegal type %s\n", TY2S(valuePtr->typePtr));
	    DECACHE_STACK_INFO();
	    IllegalExprOperandType(interp, "", pc, valuePtr);
	    CACHE_STACK_INFO();
	    goto gotError;
	}
	switch (type1) {
	case TCL_NUMBER_NAN:
	    /* -NaN => NaN */
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	case TCL_NUMBER_INT:
	    w1 = *((const Tcl_WideInt *) ptr1);
	    if (w1 != WIDE_MIN) {
		if (Tcl_IsShared(valuePtr)) {
		    TclNewIntObj(objResultPtr, -w1);
		    TRACE_APPEND_NUM_OBJ(objResultPtr);
		    NEXT_INST_F(1, 1, 1);
		}
		TclSetIntObj(valuePtr, -w1);
		TRACE_APPEND_NUM_OBJ(valuePtr);
		NEXT_INST_F0(1, 0);
	    }
	    TCL_FALLTHROUGH();
	default:
	    break;
	}
	objResultPtr = ExecuteExtendedUnaryMathOp(*pc, valuePtr);
	if (objResultPtr != NULL) {
	    TRACE_APPEND_NUM_OBJ(objResultPtr);
	    NEXT_INST_F(1, 1, 1);
	} else {
	    TRACE_APPEND_NUM_OBJ(valuePtr);
	    NEXT_INST_F0(1, 0);
	}

    case INST_UPLUS:
    case INST_TRY_CVT_TO_NUMERIC:
	/*
	 * Try to convert the topmost stack object to numeric object. This is

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetBoolean`, lines 6576–6582. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `ed9f49f644ecfd68c81dfe2ade9a852d71e07d25adc80ef99b035d5eb3394705`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_GetBoolean(Jim_Interp *interp, Jim_Obj *objPtr, int * booleanPtr)
{
    if (objPtr->typePtr != &intObjType && SetBooleanFromAny(interp, objPtr, JIM_ERRMSG) == JIM_ERR)
        return JIM_ERR;
    *booleanPtr = (int) JimWideValue(objPtr);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `ExprBool`, lines 9147–9169. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `572bbcd17f6f9d823ab7d7fe2c9c72f47ed4e28715ff615d37de21adf4b2a738`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int ExprBool(Jim_Interp *interp, Jim_Obj *obj)
{
    long l;
    double d;
    int b;
    int ret = -1;

    /* In case the object is interp->result with refcount 1*/
    Jim_IncrRefCount(obj);

    if (Jim_GetLong(interp, obj, &l) == JIM_OK) {
        ret = (l != 0);
    }
    else if (Jim_GetDouble(interp, obj, &d) == JIM_OK) {
        ret = (d != 0);
    }
    else if (Jim_GetBoolean(interp, obj, &b) == JIM_OK) {
        ret = (b != 0);
    }

    Jim_DecrRefCount(interp, obj);
    return ret;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_GetBoolFromExpr`, lines 10314–10334. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `16fca59dcfd687b61cecda487ef783d07eef961cab2f184580b0f3dfd922226c`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_GetBoolFromExpr(Jim_Interp *interp, Jim_Obj *exprObjPtr, int *boolPtr)
{
    int retcode = Jim_EvalExpression(interp, exprObjPtr);

    if (retcode == JIM_OK) {
        switch (ExprBool(interp, Jim_GetResult(interp))) {
            case 0:
                *boolPtr = 0;
                break;

            case 1:
                *boolPtr = 1;
                break;

            case -1:
                retcode = JIM_ERR;
                break;
        }
    }
    return retcode;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_ExprCoreCommand`, lines 14255–14271. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `c1eda7376d7f41024ae641af165218384ea4b772a6ab95a9fb676503bb0afad8`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_ExprCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
#ifdef JIM_COMPAT
    if (argc > 2) {
        int retcode;
        Jim_Obj *objPtr;

        objPtr = Jim_ConcatObj(interp, argc - 1, argv + 1);
        Jim_IncrRefCount(objPtr);
        retcode = Jim_EvalExpression(interp, objPtr);
        Jim_DecrRefCount(interp, objPtr);

        return retcode;
    }
#endif
    return Jim_EvalExpression(interp, argv[1]);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_IfCoreCommand`, lines 13273–13321. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `c240060512ebe372e324c443fb1c986ca6b530861861b096029ada53ae387c51`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_IfCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int boolean, retval, current = 1, falsebody = 0;

    while (1) {
        /* not enough arguments given! */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if ((retval = Jim_GetBoolFromExpr(interp, argv[current++], &boolean))
            != JIM_OK)
            return retval;
        /* There lacks something, isn't it? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (Jim_CompareStringImmediate(interp, argv[current], "then"))
            current++;
        /* Tsk tsk, no then-clause? */
        if (current >= argc) {
            return JIM_USAGE;
        }
        if (boolean)
            return Jim_EvalObj(interp, argv[current]);
        /* Ok: no else-clause follows */
        if (++current >= argc) {
            Jim_SetResult(interp, Jim_NewEmptyStringObj(interp));
            return JIM_OK;
        }
        falsebody = current++;
        if (Jim_CompareStringImmediate(interp, argv[falsebody], "else")) {
            /* IIICKS - else-clause isn't last cmd? */
            if (current != argc - 1) {
                return JIM_USAGE;
            }
            return Jim_EvalObj(interp, argv[current]);
        }
        else if (Jim_CompareStringImmediate(interp, argv[falsebody], "elseif"))
            /* Ok: elseif follows meaning all the stuff
             * again (how boring...) */
            continue;
        /* OOPS - else-clause is not last cmd? */
        else if (falsebody != argc - 1) {
            return JIM_USAGE;
        }
        return Jim_EvalObj(interp, argv[falsebody]);
    }
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_WhileCoreCommand`, lines 12801–12829. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `9312c37ddf1741b5e7f0302de2728cf8a2c9a9b667f4902b2949b2b6fafdbed3`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int Jim_WhileCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    /* The general purpose implementation of while starts here */
    while (1) {
        int boolean = 0, retval;

        if ((retval = Jim_GetBoolFromExpr(interp, argv[1], &boolean)) != JIM_OK)
            return retval;
        if (!boolean)
            break;

        if ((retval = Jim_EvalObj(interp, argv[2])) != JIM_OK) {
            if (JimCheckLoopRetcode(interp, retval)) {
                return retval;
            }
            switch (retval) {
                case JIM_BREAK:
                    goto out;
                case JIM_CONTINUE:
                    continue;
                default:
                    return retval;
            }
        }
    }
  out:
    Jim_SetEmptyResult(interp);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimExprOpAnd`, lines 9172–9186. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `fdb779c4efd7562d938c87150fef00b08d5695d967f5f2e47b99704a72b2efae`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int JimExprOpAnd(Jim_Interp *interp, struct JimExprNode *node)
{
    /* evaluate left */
    int result = JimExprGetTermBoolean(interp, node->left);

    if (result == 1) {
        /* true so evaluate right */
        result = JimExprGetTermBoolean(interp, node->right);
    }
    if (result == -1) {
        return JIM_ERR;
    }
    Jim_SetResultInt(interp, result);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `JimExprOpOr`, lines 9189–9203. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `0020e4eee25b569ac1101e299f1acd6daf69e92a4ffba872d48aa12f02be568f`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
static int JimExprOpOr(Jim_Interp *interp, struct JimExprNode *node)
{
    /* evaluate left */
    int result = JimExprGetTermBoolean(interp, node->left);

    if (result == 0) {
        /* false so evaluate right */
        result = JimExprGetTermBoolean(interp, node->right);
    }
    if (result == -1) {
        return JIM_ERR;
    }
    Jim_SetResultInt(interp, result);
    return JIM_OK;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03; exact pinned current distribution bytes`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalObj`, lines 11554–11796. Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `5934c52fe03eeba0daafaa878ba31626f4d197dc72400efb41795e75374b3626`; retained evidence `native_jim_alias_error_extent279-request-jim.c`.

```text
int Jim_EvalObj(Jim_Interp *interp, Jim_Obj *scriptObjPtr)
{
    int i;
    ScriptObj *script;
    ScriptToken *token;
    int retcode = JIM_OK;
    Jim_Obj *sargv[JIM_EVAL_SARGV_LEN], **argv = NULL;
    Jim_EvalFrame frame;

    /* If the object is of type "list", with no string rep we can call
     * a specialized version of Jim_EvalObj() */
    if (Jim_IsList(scriptObjPtr) && scriptObjPtr->bytes == NULL) {
        return JimEvalObjList(interp, scriptObjPtr);
    }

    Jim_IncrRefCount(scriptObjPtr);     /* Make sure it's shared. */
    script = JimGetScript(interp, scriptObjPtr);
    if (JimParseCheckMissing(interp, script->missing) == JIM_ERR) {
        JimSetErrorStack(interp, script);
        Jim_DecrRefCount(interp, scriptObjPtr);
        return JIM_ERR;
    }

    /* Reset the interpreter result. This is useful to
     * return the empty result in the case of empty program. */
    Jim_SetEmptyResult(interp);

    token = script->token;

#ifdef JIM_OPTIMIZATION
    /* Check for one of the following common scripts used by for, while
     *
     *   {}
     *   incr a
     */
    if (script->len == 0) {
        Jim_DecrRefCount(interp, scriptObjPtr);
        /* Allow a signal to interrupt loops that dispatch no command. */
        if (Jim_CheckSignal(interp))
            return JIM_SIGNAL;
        return JIM_OK;
    }
    if (script->len == 3
        && token[1].objPtr->typePtr == &commandObjType
        && (token[1].objPtr->internalRep.cmdValue.cmdPtr->flags & JIM_CMD_ISPROC) == 0
        && token[1].objPtr->internalRep.cmdValue.cmdPtr->u.native.cmdProc == Jim_IncrCoreCommand
        && token[2].objPtr->typePtr == &variableObjType) {

        Jim_Obj *objPtr = Jim_GetVariable(interp, token[2].objPtr, JIM_NONE);

        if (objPtr && !Jim_IsShared(objPtr) && objPtr->typePtr == &intObjType) {
            JimWideValue(objPtr)++;
            Jim_InvalidateStringRep(objPtr);
            Jim_DecrRefCount(interp, scriptObjPtr);
            Jim_SetResult(interp, objPtr);
            /* Allow a signal to interrupt loops that dispatch no command. */
            if (Jim_CheckSignal(interp))
                return JIM_SIGNAL;
            return JIM_OK;
        }
    }
#endif

    /* Now we have to make sure the internal repr will not be
     * freed on shimmering.
     *
     * Think for example to this:
     *
     * set x {llength $x; ... some more code ...}; eval $x
     *
     * In order to preserve the internal rep, we increment the
     * inUse field of the script internal rep structure. */
    script->inUse++;

    JimPushEvalFrame(interp, &frame, scriptObjPtr);

    /* Collect a new error stack trace if an error occurs */
    interp->hasErrorStackTrace = 0;
    argv = sargv;

    /* Execute every command sequentially until the end of the script
     * or an error occurs.
     */
    for (i = 0; i < script->len && retcode == JIM_OK; ) {
        int argc;
        int j;

        /* First token of the line is always JIM_TT_LINE */
        argc = token[i].objPtr->internalRep.scriptLineValue.argc;
        script->linenr = token[i].objPtr->internalRep.scriptLineValue.line;

        /* Allocate the arguments vector if required */
        if (argc > JIM_EVAL_SARGV_LEN)
            argv = Jim_Alloc(sizeof(Jim_Obj *) * argc);

        /* Skip the JIM_TT_LINE token */
        i++;

        /* Populate the arguments objects.
         * If an error occurs, retcode will be set and
         * 'j' will be set to the number of args expanded
         */
        for (j = 0; j < argc; j++) {
            long wordtokens = 1;
            int expand = 0;
            Jim_Obj *wordObjPtr = NULL;

            if (token[i].type == JIM_TT_WORD) {
                wordtokens = JimWideValue(token[i++].objPtr);
                if (wordtokens < 0) {
                    expand = 1;
                    wordtokens = -wordtokens;
                }
            }

            if (wordtokens == 1) {
                /* Fast path if the token does not
                 * need interpolation */

                switch (token[i].type) {
                    case JIM_TT_ESC:
                    case JIM_TT_STR:
                        wordObjPtr = token[i].objPtr;
                        break;
                    case JIM_TT_VAR:
                        wordObjPtr = Jim_GetVariable(interp, token[i].objPtr, JIM_ERRMSG);
                        break;
                    case JIM_TT_EXPRSUGAR:
                        retcode = Jim_EvalExpression(interp, token[i].objPtr);
                        if (retcode == JIM_OK) {
                            wordObjPtr = Jim_GetResult(interp);
                        }
                        else {
                            wordObjPtr = NULL;
                        }
                        break;
                    case JIM_TT_DICTSUGAR:
                        wordObjPtr = JimExpandDictSugar(interp, token[i].objPtr);
                        break;
                    case JIM_TT_CMD:
                        retcode = Jim_EvalObj(interp, token[i].objPtr);
                        if (retcode == JIM_OK) {
                            wordObjPtr = Jim_GetResult(interp);
                        }
                        break;
                    default:
                        JimPanic((1, "default token type reached " "in Jim_EvalObj()."));
                }
            }
            else {
                /* For interpolation we call a helper
                 * function to do the work for us. */
                wordObjPtr = JimInterpolateTokens(interp, token + i, wordtokens, JIM_NONE);
            }

            if (!wordObjPtr) {
                if (retcode == JIM_OK) {
                    retcode = JIM_ERR;
                }
                break;
            }

            Jim_IncrRefCount(wordObjPtr);
            i += wordtokens;

            if (!expand) {
                argv[j] = wordObjPtr;
            }
            else {
                /* Need to expand wordObjPtr into multiple args from argv[j] ... */
                int len = Jim_ListLength(interp, wordObjPtr);
                int newargc = argc + len - 1;
                int k;

                if (len > 1) {
                    if (argv == sargv) {
                        if (newargc > JIM_EVAL_SARGV_LEN) {
                            argv = Jim_Alloc(sizeof(*argv) * newargc);
                            memcpy(argv, sargv, sizeof(*argv) * j);
                        }
                    }
                    else {
                        /* Need to realloc to make room for (len - 1) more entries */
                        argv = Jim_Realloc(argv, sizeof(*argv) * newargc);
                    }
                }

                /* Now copy in the expanded version */
                for (k = 0; k < len; k++) {
                    argv[j++] = wordObjPtr->internalRep.listValue.ele[k];
                    Jim_IncrRefCount(wordObjPtr->internalRep.listValue.ele[k]);
                }

                /* The original object reference is no longer needed,
                 * after the expansion it is no longer present on
                 * the argument vector, but the single elements are
                 * in its place. */
                Jim_DecrRefCount(interp, wordObjPtr);

                /* And update the indexes */
                j--;
                argc += len - 1;
            }
        }

        if (retcode == JIM_OK && argc) {
            /* Invoke the command */
            retcode = JimInvokeCommand(interp, argc, argv);
            interp->taint = 0;
            /* Check for a signal after each command */
            if (Jim_CheckSignal(interp)) {
                retcode = JIM_SIGNAL;
            }
        }

        /* Finished with the command, so decrement ref counts of each argument */
        while (j-- > 0) {
            Jim_DecrRefCount(interp, argv[j]);
        }

        if (argv != sargv) {
            Jim_Free(argv);
            argv = sargv;
        }
    }

    /* Possibly add to the error stack trace */
    if (retcode == JIM_ERR) {
        JimSetErrorStack(interp, NULL);
    }

    JimPopEvalFrame(interp);

    /* Note that we don't have to decrement inUse, because the
     * following code transfers our use of the reference again to
     * the script object. */
    Jim_FreeIntRep(interp, scriptObjPtr);
    scriptObjPtr->typePtr = &scriptObjType;
    Jim_SetIntRepPtr(scriptObjPtr, script);
    Jim_DecrRefCount(interp, scriptObjPtr);

    return retcode;
}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Restore all original absolute inputs through recorded-path-map.json, then reproduce each exact recorded compile, independent CLI-version and observer command with its selected environment. Keep all eleven fixed ASCII script/version inputs byte-exact. Install x and y through their actual public constructor/variable APIs, never source interpolation. Generic and held object-source evaluation remain independent workers; keep all original source objects alive until post-call observers finish. Whole raw streams/receipts are authoritative; both Root-derived arrays are separate review only. No unrecorded launcher or historic build operation is invented. New runs/providers/ABIs are independent evidence, not software or application admission.
