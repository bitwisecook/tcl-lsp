# naming.package.jim-hidden-list

Kind: `native-observation`

## Problem statement

The Jim package unknown-member error names forget, names, provide and require. A catalogue rebuilt only from that report could remove a supported hidden list operation, even though an error message is a presentation roster rather than the complete operation inventory.

## Question

Does current Jim package list succeed with the same result as package names even though the exact unknown-member error omits list from its advertised operations?

## Conclusion

Jim 0.84-9-g5bac7c9 accepts both list and names in this fresh process with catch code 0 and the same recorded package sequence. The deliberate absent operation returns catch code 1 and an error roster containing forget, names, provide and require, omitting the successfully executed list operation. This one native control proves that this error roster is incomplete for supported package operations; it supplies no C or BIG-IP answer and no guarantee about arbitrary prefixes or other hidden commands.

## Scope

One fixed ASCII script-file program in a fresh current Jim native process. Actual patchlevel, complete input/executable/stream digests, three caught outcomes and outer process status are recorded. Dynamic enumeration content belongs to this process, not a universal installed-package set.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

No observation for this exact question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

No observation for this exact question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

No observation for this exact question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

No observation for this exact question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: C Tcl.

No observation for this exact question.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Selected executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; compiler, configure/UTF flags and linked dependencies are unrecorded.. Channel: Fixed ASCII script-file argument in a fresh native shell process. Dialect: Jim Tcl.

Exact stdout:

```text
VERSION|0.84-9-g5bac7c9
RESULT list 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT names 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT __r2286_absent__ 1 {package, unknown command "__r2286_absent__": should be forget, names, provide, require}
```

The list/names catch results are both 0 and contain the identical printed package sequence. The absent-member catch result is 1 and omits list from the help roster. Outer process status is 0 and stderr is empty.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No observation for this exact question.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_jim_package_hidden_list/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/probe.tcl). SHA-256 `fbd0e3c6b630c0bf4dd82a79a7e9b1328f1cd7549d7980106f7975ea3ab76592`. Exact original ASCII program for successful list/names and deliberate miss.
- `capture` (provider): [rust/tcl-registry/tests/data/native_jim_package_hidden_list/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/receipt.json). SHA-256 `8046e43cba77bdf4646b402c869cecca556a6d3bd5388318807b12cc213c25fd`. Original selected executable/source/stream digests, argv and process status.
- `stdout` (observation): [rust/tcl-registry/tests/data/native_jim_package_hidden_list/stdout](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/stdout). SHA-256 `5c1ae80c92266addbd9e63038a5138dc613fa2b403f5f19d95129b4262abf4ce`. Exact complete stdout with actual patchlevel and all three caught results.
- `stderr` (observation): [rust/tcl-registry/tests/data/native_jim_package_hidden_list/stderr](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr byte stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded` (linked): Authored genuine Jim-point inventory keeps list and names with zero-argument List metadata; the native error text and returned package sequence are evidence-only, not reproduced by this Rust test.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "/workspace/.proofs/native-providers/jimtcl/jimsh",
  "/workspace/.proofs/captured-package-future-frame122/probe.tcl"
]
```

The original argument vector names capture-machine paths. The identical retained source is in rust/tcl-registry/tests/data/native_jim_package_hidden_list/probe.tcl; replay it with an independently verified current Jim executable in a fresh process. Executable digest is recorded, while compiler, configure/UTF flags and linked dependencies are unrecorded. The native script result is not a counted object-vector, physical native object, Rust test or compilation-admission receipt. C and appliance providers were not run for this question.
